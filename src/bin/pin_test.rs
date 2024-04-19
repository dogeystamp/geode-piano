//! Tester for `geode_piano::pins::TransparentPins`.
//!
//! This is quickly hacked together.

#![no_std]
#![no_main]
#![deny(rust_2018_idioms)]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio;
use embassy_rp::i2c;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::Timer;
use geode_piano::usb::usb_task;
use geode_piano::{blinky, pin_array, pins, unwrap};

#[embassy_executor::task]
async fn read_task(mut pin_driver: pins::TransparentPins) {
    loop {
        log::warn!("{:036b}", unwrap(pin_driver.read_all()).await);
        Timer::after_millis(1000).await;
    }
}

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);
    unwrap(_spawner.spawn(usb_task(driver, log::LevelFilter::Info))).await;
    unwrap(_spawner.spawn(blinky::blink_task(p.PIN_25.into()))).await;

    Timer::after_secs(2).await;

    log::info!("main: init i2c");
    let sda = p.PIN_16;
    let scl = p.PIN_17;

    let mut i2c_config = i2c::Config::default();
    let freq = 100_000;
    i2c_config.frequency = freq;
    let i2c = i2c::I2c::new_blocking(p.I2C0, scl, sda, i2c_config);

    log::info!("main: starting transparent pin driver");
    let mut pin_driver = unwrap(pins::TransparentPins::new(
        i2c,
        [0x20, 0x27],
        pin_array!(
            p.PIN_15, p.PIN_14, p.PIN_13, p.PIN_12, p.PIN_11, p.PIN_10, p.PIN_9, p.PIN_18,
            p.PIN_19, p.PIN_20, p.PIN_21, p.PIN_22
        ),
        true,
    ))
    .await;

    log::info!("main: setting pins as input");
    for i in pin_driver.pins {
        unwrap(pin_driver.set_input(i)).await;
        unwrap(pin_driver.set_pull(i, gpio::Pull::Up)).await;
    }
    log::debug!("main: setting pin 0 as output, active low");
    unwrap(pin_driver.set_output(0)).await;
    unwrap(pin_driver.write_all(0)).await;

    log::debug!("main: starting read task");
    _spawner.spawn(read_task(pin_driver)).unwrap();
}
