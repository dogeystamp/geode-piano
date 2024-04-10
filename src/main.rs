#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio;
use embassy_rp::i2c;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::Timer;
use geode_usb::usb_task;
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

mod geode_midi;
mod geode_usb;
mod pins;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

/// Unwrap, but log before panic
///
/// Waits a bit to give time for the logger to flush before halting.
/// This exists because I do not own a debug probe 😎
async fn unwrap<T, E: core::fmt::Debug>(res: Result<T, E>) -> T {
    match res {
        Ok(v) => v,
        Err(e) => {
            log::error!("[FATAL] {:?}", e);
            log::error!("HALTING DUE TO PANIC.");
            Timer::after_millis(10).await;
            panic!();
        }
    }
}

#[embassy_executor::task]
async fn blink_task(pin: embassy_rp::gpio::AnyPin) {
    let mut led = Output::new(pin, Level::Low);

    loop {
        led.set_high();
        Timer::after_millis(100).await;

        led.set_low();
        Timer::after_millis(900).await;
    }
}

#[embassy_executor::task]
async fn read_task(mut pin_driver: pins::TransparentPins) {
    loop {
        log::warn!("{:b}", unwrap(pin_driver.read_all()).await);
        Timer::after_millis(1000).await;
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);
    _spawner.spawn(usb_task(driver)).unwrap();

    _spawner.spawn(blink_task(p.PIN_25.into())).unwrap();

    Timer::after_secs(2).await;

    log::info!("main: init i2c");
    let sda = p.PIN_16;
    let scl = p.PIN_17;

    let mut i2c_config = i2c::Config::default();
    let freq = 100_000;
    i2c_config.frequency = freq;
    let i2c = i2c::I2c::new_blocking(p.I2C0, scl, sda, i2c_config);

    log::info!("main: starting transparent pin driver");
    let mut pin_driver = pins::TransparentPins::new(i2c, [0x20], []);

    log::info!("main: setting pins as input");
    for i in 0..16 {
        log::debug!("main: setting pin {} as input, pull up", i);
        unwrap(pin_driver.set_input(i)).await;
        unwrap(pin_driver.set_pull(i, gpio::Pull::Up)).await;
    }

    // these pins are faulty as inputs
    // unwrap(pin_driver.set_output(7)).await;
    // unwrap(pin_driver.set_output(8 + 7)).await;
    // unwrap(pin_driver.set_output(16 + 7)).await;
    // unwrap(pin_driver.set_output(16 + 8 + 7)).await;

    log::debug!("main: starting read task");
    _spawner.spawn(read_task(pin_driver)).unwrap();
}
