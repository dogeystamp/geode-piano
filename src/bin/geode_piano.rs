/*
    geode-piano
    Copyright (C) 2024 dogeystamp <dogeystamp@disroot.org>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

//! Main firmware for geode-piano. Reads key-matrix and sends MIDI output.

#![no_std]
#![no_main]
#![deny(rust_2018_idioms)]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use geode_piano::usb::usb_task;
use geode_piano::matrix::KeyMatrix;
use geode_piano::{blinky, pin_array, pins, unwrap};

#[embassy_executor::task]
async fn piano_task(pin_driver: pins::TransparentPins) {
    use geode_piano::midi::Note::*;

    // GND pins
    let col_pins = [23];
    // Input pins
    let row_pins = [20, 15, 4];
    // Notes for each key
    let keymap = [[C4, D4, E4]];

    let mut mat = KeyMatrix::new(col_pins, row_pins, keymap);
    mat.scan(pin_driver).await;
}

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);
    unwrap(_spawner.spawn(usb_task(driver, log::LevelFilter::Debug))).await;
    unwrap(_spawner.spawn(blinky::blink_task(p.PIN_25.into()))).await;

    log::debug!("main: init i2c");
    let sda = p.PIN_16;
    let scl = p.PIN_17;

    let mut i2c_config = i2c::Config::default();
    let freq = 100_000;
    i2c_config.frequency = freq;
    let i2c = i2c::I2c::new_blocking(p.I2C0, scl, sda, i2c_config);

    log::debug!("main: starting transparent pin driver");
    let pin_driver = unwrap(pins::TransparentPins::new(
        i2c,
        [0x20, 0x27],
        pin_array!(
            p.PIN_15, p.PIN_14, p.PIN_13, p.PIN_12, p.PIN_11, p.PIN_10, p.PIN_9, p.PIN_18,
            p.PIN_19, p.PIN_20, p.PIN_21, p.PIN_22
        ),
        true,
    ))
    .await;

    log::info!("main: starting piano task");
    _spawner.spawn(piano_task(pin_driver)).unwrap();
}
