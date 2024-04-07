#![no_std]
#![no_main]

use defmt::*;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_time::Timer;
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut led = Output::new(p.PIN_25, Level::Low);

    let driver = Driver::new(p.USB, Irqs);
    _spawner.spawn(logger_task(driver)).unwrap();

    loop {
        log::info!("led on!");
        led.set_high();
        Timer::after_millis(100).await;

        log::info!("led off!");
        led.set_low();
        Timer::after_secs(1).await;
    }
}
