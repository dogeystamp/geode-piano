#![doc = include_str!("../README.md")]

#![no_std]
#![no_main]
#![deny(rust_2018_idioms)]
#![deny(rustdoc::broken_intra_doc_links)]

use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

pub mod blinky;
pub mod midi;
pub mod pins;
pub mod usb;

/// Unwrap, but log before panic
///
/// Waits a bit to give time for the logger to flush before halting.
/// This exists because I do not own a debug probe 😎
pub async fn unwrap<T, E: core::fmt::Debug>(res: Result<T, E>) -> T {
    match res {
        Ok(v) => v,
        Err(e) => {
            log::error!("[FATAL] {:?}", e);
            log::error!("HALTING DUE TO PANIC.");
            Timer::after_secs(1).await;
            panic!();
        }
    }
}
