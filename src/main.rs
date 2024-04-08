#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio;
use embassy_rp::gpio::AnyPin;
use embassy_rp::gpio::Input;
use embassy_rp::gpio::Pull;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::Timer;
use geode_usb::usb_task;
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

mod geode_midi;
mod geode_usb;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn blink_task(pin: embassy_rp::gpio::AnyPin) {
    let mut led = Output::new(pin, Level::Low);

    loop {
        led.set_high();
        Timer::after_millis(100).await;

        led.set_low();
        Timer::after_secs(5).await;
    }
}

enum Note {
    C,
    Pedal,
}

#[embassy_executor::task(pool_size = 2)]
async fn button(pin: AnyPin, note: Note) {
    let mut button = Input::new(pin, Pull::Up);
    let chan = geode_midi::MidiChannel::new(0);
    loop {
        let mut counter = 10;
        button.wait_for_falling_edge().await;
        loop {
            Timer::after_millis(5).await;
            if button.is_low() {
                counter -= 1;
            } else {
                counter = 10;
            }
            if counter <= 0 {
                break;
            }
        }
        match note {
            Note::C => chan.note_on(72, 64).await,
            Note::Pedal => {
                chan.controller(geode_midi::Controller::SustainPedal, 64)
                    .await
            }
        }
        log::info!("button press");
        counter = 10;
        button.wait_for_rising_edge().await;
        loop {
            Timer::after_millis(5).await;
            if button.is_high() {
                counter -= 1;
            } else {
                counter = 10;
            }
            if counter <= 0 {
                break;
            }
        }
        match note {
            Note::C => chan.note_off(72, 0).await,
            Note::Pedal => {
                chan.controller(geode_midi::Controller::SustainPedal, 0)
                    .await
            }
        }
        log::info!("button release");
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);

    _spawner.spawn(blink_task(p.PIN_25.into())).unwrap();
    _spawner.spawn(button(p.PIN_16.into(), Note::C)).unwrap();
    _spawner
        .spawn(button(p.PIN_17.into(), Note::Pedal))
        .unwrap();
    _spawner.spawn(usb_task(driver)).unwrap();
}
