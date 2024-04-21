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
use geode_piano::matrix;
use geode_piano::matrix::KeyMatrix;
use geode_piano::midi;
use geode_piano::usb::usb_task;
use geode_piano::{blinky, pin_array, pins, unwrap};

#[embassy_executor::task]
async fn piano_task(pin_driver: pins::TransparentPins) {
    use geode_piano::midi::KeyAction::*;
    use geode_piano::midi::Note::*;

    // GND pins
    let col_pins = [32, 33, 34, 4, 36, 6, 7, 37, 38, 39, 15, 19, 24, 25, 26, 31];
    // Input pins
    let row_pins = [
        1, 2, 3, 5, 8, 9, 10, 12, 13, 14, 16, 17, 18, 20, 21, 22, 23, 27, 28, 29, 30, 35,
    ];
    // Notes for each key
    let keymap = [
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N(A0, 64),
            NOP,
            N(CS1, 64),
            N(B0, 64),
            NOP,
            NOP,
            NOP,
            N(DS1, 64),
            N(E1, 64),
            N(C1, 64),
            N(D1, 64),
            N(AS0, 64),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N(F1, 64),
            NOP,
            N(A1, 64),
            N(G1, 64),
            NOP,
            NOP,
            NOP,
            N(B1, 64),
            N(C2, 64),
            N(GS1, 64),
            N(AS1, 64),
            N(FS1, 64),
            NOP,
        ],
        [
            N(GS5, 64),
            N(AS5, 64),
            N(C6, 64),
            NOP,
            N(F5, 64),
            NOP,
            NOP,
            N(G5, 64),
            N(A5, 64),
            N(B5, 64),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N(FS5, 64),
        ],
        [
            N(C7, 64),
            N(D7, 64),
            N(E7, 64),
            NOP,
            N(A6, 64),
            NOP,
            NOP,
            N(B6, 64),
            N(CS7, 64),
            N(DS7, 64),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N(AS6, 64),
        ],
        [
            N(E6, 64),
            N(FS6, 64),
            N(GS6, 64),
            NOP,
            N(CS6, 64),
            NOP,
            NOP,
            N(DS6, 64),
            N(F6, 64),
            N(G6, 64),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N(D6, 64),
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N(A2, 64),
            NOP,
            N(CS3, 64),
            N(B2, 64),
            NOP,
            NOP,
            NOP,
            N(DS3, 64),
            N(E3, 64),
            N(C3, 64),
            N(D3, 64),
            N(AS2, 64),
            NOP,
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N(A4, 64),
            NOP,
            N(CS5, 64),
            N(B4, 64),
            NOP,
            NOP,
            NOP,
            N(DS5, 64),
            N(E5, 64),
            N(C5, 64),
            N(D5, 64),
            N(AS4, 64),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N(F3, 64),
            NOP,
            N(A3, 64),
            N(G3, 64),
            NOP,
            NOP,
            NOP,
            N(B3, 64),
            N(C4, 64),
            N(GS3, 64),
            N(AS3, 64),
            N(FS3, 64),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N(CS4, 64),
            NOP,
            N(F4, 64),
            N(DS4, 64),
            NOP,
            NOP,
            NOP,
            N(G4, 64),
            N(GS4, 64),
            N(E4, 64),
            N(FS4, 64),
            N(D4, 64),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N(CS2, 64),
            NOP,
            N(F2, 64),
            N(DS2, 64),
            NOP,
            NOP,
            NOP,
            N(G2, 64),
            N(GS2, 64),
            N(E2, 64),
            N(FS2, 64),
            N(D2, 64),
            NOP,
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP, NOP,
        ],
        [
            N(GS7, 64),
            N(AS7, 64),
            N(C8, 64),
            NOP,
            N(F7, 64),
            NOP,
            NOP,
            N(G7, 64),
            N(A7, 64),
            N(B7, 64),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N(FS7, 64),
        ],
    ];

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
    let freq = 400_000;
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

    log::info!("main: starting sustain pedal task");
    _spawner
        .spawn(matrix::pedal(
            midi::Controller::SustainPedal,
            p.PIN_8.into(),
        ))
        .unwrap();
}
