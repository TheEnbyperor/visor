#![no_std]
#![no_main]
#![allow(static_mut_refs)]
#![feature(asm_experimental_arch)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

use core::mem::MaybeUninit;
use embedded_hal_async::digital::Wait;

mod init;
mod display;

pub static IO_LOCK: embassy_sync::semaphore::FairSemaphore<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    2,
> = embassy_sync::semaphore::FairSemaphore::new(1);

pub static UART_TX: embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    MaybeUninit<esp_hal::uart::UartTx<esp_hal::Blocking>>,
> = embassy_sync::mutex::Mutex::new(MaybeUninit::uninit());

#[embassy_executor::task]
async fn main() {
    let button_pin = unsafe { esp_hal::peripherals::GPIO0::steal() };
    let mut button = esp_hal::gpio::Input::new(
        button_pin,
        esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up)
    );

    info!("Running!");

    loop {
        button.wait_for_low().await;
        display::NEXT_EYE.signal(());
        button.wait_for_high().await;
    }
}