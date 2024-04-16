//! blinky task

use embassy_rp::gpio::{Level, Output};
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn blink_task(pin: embassy_rp::gpio::AnyPin) {
    let mut led = Output::new(pin, Level::Low);

    loop {
        led.set_high();
        Timer::after_millis(100).await;

        led.set_low();
        Timer::after_millis(900).await;
    }
}
