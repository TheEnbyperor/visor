use core::mem::MaybeUninit;
use embassy_executor::Spawner;
use esp_hal::interrupt;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::interrupt::Priority;
use esp_hal::peripherals::Interrupt;
use esp_hal_embassy::InterruptExecutor;
use static_cell::StaticCell;
use core::fmt::Write;

esp_bootloader_esp_idf::esp_app_desc!();

static LOGGER: PrintlnLogger = PrintlnLogger;

static mut APP_CORE_STACK: esp_hal::system::Stack<8192> = esp_hal::system::Stack::new();
static mut APP_CORE_GUARD: Option<esp_hal::system::AppCoreGuard> = None;

pub static mut RAND: MaybeUninit<esp_hal::rng::Rng> = MaybeUninit::uninit();

static EXEC1: StaticCell<InterruptExecutor<1>> = StaticCell::new();

static UART_RX: StaticCell<esp_hal::uart::UartRx<esp_hal::Async>> = StaticCell::new();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    let hal_config = esp_hal::Config::default()
        .with_cpu_clock(esp_hal::clock::CpuClock::max());
    let peripherals = esp_hal::init(hal_config);

    let (uart_rx, uart_tx) = esp_hal::uart::Uart::new(peripherals.UART0, esp_hal::uart::Config::default())
        .unwrap()
        .with_rx(peripherals.GPIO3)
        .with_tx(peripherals.GPIO1)
        .split();
    let uart_rx = UART_RX.init(uart_rx.into_async());
    *crate::UART_TX.lock().await = MaybeUninit::new(uart_tx);

    init_heap();
    setup_logger();
    log::set_max_level(log::LevelFilter::Info);

    let timg1 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init([timg1.timer0, timg1.timer1]);

    let swints = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let mut cpu_control = esp_hal::system::CpuControl::new(peripherals.CPU_CTRL);
    let guard = unsafe {
        cpu_control
            .start_app_core(&mut APP_CORE_STACK, move || {
                let exec1 = EXEC1.init(InterruptExecutor::new(swints.software_interrupt1));
                interrupt::enable(Interrupt::FROM_CPU_INTR1, Priority::Priority2).unwrap();
                let spawner = exec1.start(Priority::Priority2);

                spawner.must_spawn(crate::display::main());

                loop { core::hint::spin_loop() }
            })
            .expect("Failed to start on core 1")
    };
    unsafe {
        APP_CORE_GUARD = Some(guard);
    }

    spawner.must_spawn(crate::main());

    loop {
        embassy_time::Timer::after_secs(5).await;
    }
}

pub struct PrintlnLogger;
impl log::Log for PrintlnLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let level = match record.level() {
            log::Level::Error => b"\x1b[31mERROR\x1b[0m ",
            log::Level::Warn => b"\x1b[33mWARN\x1b[0m  ",
            log::Level::Info => b"\x1b[32mINFO\x1b[0m  ",
            log::Level::Debug => b"\x1b[36mDEBUG\x1b[0m ",
            log::Level::Trace => b"\x1b[36mTRACE\x1b[0m ",
        };
        let mut uart = embassy_futures::block_on(crate::UART_TX.lock());
        unsafe { uart.assume_init_mut() }.write(level).unwrap();
        unsafe { uart.assume_init_mut() }
            .write_fmt(*record.args())
            .unwrap();
        unsafe { uart.assume_init_mut() }.write_char('\n').unwrap();
        unsafe { uart.assume_init_mut() }.flush().unwrap();
    }

    fn flush(&self) {}
}

pub fn setup_logger() {
    let _ = log::set_logger(&LOGGER);
}

pub fn init_heap() {
    fn add_region<const N: usize>(region: &'static mut core::mem::MaybeUninit<[u8; N]>) {
        unsafe {
            esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
                region.as_mut_ptr() as *mut u8,
                N,
                esp_alloc::MemoryCapability::Internal.into(),
            ));
        }
    }

    static mut HEAP1: core::mem::MaybeUninit<[u8; 256 * 1024]> = core::mem::MaybeUninit::uninit();
    #[link_section = ".dram2_uninit"]
    static mut HEAP2: core::mem::MaybeUninit<[u8; 72 * 1024]> = core::mem::MaybeUninit::uninit();

    add_region(unsafe { &mut HEAP1 });
    add_region(unsafe { &mut HEAP2 });
}

struct Backtrace(heapless::Vec<BacktraceFrame, 10>);

impl Backtrace {
    #[inline]
    pub fn capture() -> Self {
        let sp = sp();

        let mut result = Self(heapless::Vec::new());

        let mut fp = sp;

        if !is_valid_ram_address(fp) {
            return result;
        }

        while !result.0.is_full() {
            // RA/PC
            let address = unsafe { (fp as *const u32).offset(-4).read_volatile() };
            let address = remove_window_increment(address);
            // next FP
            fp = unsafe { (fp as *const u32).offset(-3).read_volatile() };

            // the return address is 0 but we sanitized the address - then 0 becomes
            // 0x40000000
            if address == 0x40000000 {
                break;
            }

            if !is_valid_ram_address(fp) {
                break;
            }

            _ = result.0.push(BacktraceFrame {
                pc: address as usize,
            });
        }

        result
    }

    /// Returns the backtrace frames as a slice.
    #[inline]
    pub fn frames(&self) -> &[BacktraceFrame] {
        &self.0
    }
}

#[inline(never)]
#[cold]
fn sp() -> u32 {
    let mut sp: u32;
    unsafe {
        core::arch::asm!(
        "mov {0}, a1",
        "add a12,a12,a12",
        "rotw 3",
        "add a12,a12,a12",
        "rotw 3",
        "add a12,a12,a12",
        "rotw 3",
        "add a12,a12,a12",
        "rotw 3",
        "add a12,a12,a12",
        "rotw 4",
        out(reg) sp
        );
    }

    // current frame pointer, caller's stack pointer
    unsafe { ((sp - 12) as *const u32).read_volatile() }
}

fn remove_window_increment(address: u32) -> u32 {
    (address & 0x3fff_ffff) | 0x4000_0000
}

fn is_valid_ram_address(address: u32) -> bool {
    esp_metadata_generated::memory_range!("DRAM").contains(&address)
}

pub struct BacktraceFrame {
    pub(crate) pc: usize,
}

impl BacktraceFrame {
    pub fn program_counter(&self) -> usize {
        const RA_OFFSET: usize = 3;
        self.pc - RA_OFFSET
    }
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    error!("====================== PANIC ======================");
    error!("{}", info);
    error!("Backtrace:");

    let backtrace = Backtrace::capture();
    for frame in backtrace.frames() {
        error!("0x{:x}", frame.program_counter());
    }

    // ESP32
    // const OPTIONS0: *mut u32 = 0x3ff48000 as *mut u32;
    // const SW_CPU_STALL: *mut u32 = 0x3ff480ac as *mut u32;

    // ESP32S3
    const OPTIONS0: *mut u32 = 0x60008000 as *mut u32;
    const SW_CPU_STALL: *mut u32 = 0x600080bc as *mut u32;

    unsafe {
        OPTIONS0.write_volatile(OPTIONS0.read_volatile() & !(0b1111) | 0b1010);
        SW_CPU_STALL.write_volatile(
            SW_CPU_STALL.read_volatile() & !(0b111111 << 20) & !(0b111111 << 26)
                | (0x21 << 20)
                | (0x21 << 26),
        );
    }

    loop {}
}
