#![no_std]
#![no_main]
#![allow(static_mut_refs)]
#![feature(asm_experimental_arch)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

use core::mem::MaybeUninit;

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
    info!("Running!");
}