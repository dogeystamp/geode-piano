#![doc = include_str!("../README.md")]
#![no_std]
#![no_main]
#![deny(rust_2018_idioms)]
#![deny(rustdoc::broken_intra_doc_links)]

use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

pub mod blinky;
pub mod matrix;
pub mod midi;
pub mod pins;
pub mod usb;

/// Wrapper over unwrap.
///
/// Logs over usb instead of instantly panicking.
/// If you don't have a debug probe, comment out the first line.
pub async fn unwrap<T, E: core::fmt::Debug>(res: Result<T, E>) -> T {
    return res.unwrap();
    #[allow(unreachable_code)]
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
