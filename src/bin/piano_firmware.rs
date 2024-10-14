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
            N1(GS5),
            N1(AS5),
            N1(C6),
            NOP,
            N1(F5),
            NOP,
            NOP,
            N1(G5),
            N1(A5),
            N1(B5),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N1(FS5),
        ],
        [
            NOP,
            NOP,
            NOP,
            N1(F1),
            NOP,
            N1(A1),
            N1(G1),
            NOP,
            NOP,
            NOP,
            N1(B1),
            N1(C2),
            N1(GS1),
            N1(AS1),
            N1(FS1),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N1(A0),
            NOP,
            N1(CS1),
            N1(B0),
            NOP,
            NOP,
            NOP,
            N1(DS1),
            N1(E1),
            N1(C1),
            N1(D1),
            N1(AS0),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N1(CS2),
            NOP,
            N1(F2),
            N1(DS2),
            NOP,
            NOP,
            NOP,
            N1(G2),
            N1(GS2),
            N1(E2),
            N1(FS2),
            N1(D2),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N2(A0),
            NOP,
            N2(CS1),
            N2(B0),
            NOP,
            NOP,
            NOP,
            N2(DS1),
            N2(E1),
            N2(C1),
            N2(D1),
            N2(AS0),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N2(F1),
            NOP,
            N2(A1),
            N2(G1),
            NOP,
            NOP,
            NOP,
            N2(B1),
            N2(C2),
            N2(GS1),
            N2(AS1),
            N2(FS1),
            NOP,
        ],
        [
            N2(GS5),
            N2(AS5),
            N2(C6),
            NOP,
            N2(F5),
            NOP,
            NOP,
            N2(G5),
            N2(A5),
            N2(B5),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N2(FS5),
        ],
        [
            N2(C7),
            N2(D7),
            N2(E7),
            NOP,
            N2(A6),
            NOP,
            NOP,
            N2(B6),
            N2(CS7),
            N2(DS7),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N2(AS6),
        ],
        [
            N2(E6),
            N2(FS6),
            N2(GS6),
            NOP,
            N2(CS6),
            NOP,
            NOP,
            N2(DS6),
            N2(F6),
            N2(G6),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N2(D6),
        ],
        [
            NOP,
            NOP,
            NOP,
            N1(A2),
            NOP,
            N1(CS3),
            N1(B2),
            NOP,
            NOP,
            NOP,
            N1(DS3),
            N1(E3),
            N1(C3),
            N1(D3),
            N1(AS2),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N1(CS4),
            NOP,
            N1(F4),
            N1(DS4),
            NOP,
            NOP,
            NOP,
            N1(G4),
            N1(GS4),
            N1(E4),
            N1(FS4),
            N1(D4),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N1(F3),
            NOP,
            N1(A3),
            N1(G3),
            NOP,
            NOP,
            NOP,
            N1(B3),
            N1(C4),
            N1(GS3),
            N1(AS3),
            N1(FS3),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N2(A2),
            NOP,
            N2(CS3),
            N2(B2),
            NOP,
            NOP,
            NOP,
            N2(DS3),
            N2(E3),
            N2(C3),
            N2(D3),
            N2(AS2),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N1(A4),
            NOP,
            N1(CS5),
            N1(B4),
            NOP,
            NOP,
            NOP,
            N1(DS5),
            N1(E5),
            N1(C5),
            N1(D5),
            N1(AS4),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N2(A4),
            NOP,
            N2(CS5),
            N2(B4),
            NOP,
            NOP,
            NOP,
            N2(DS5),
            N2(E5),
            N2(C5),
            N2(D5),
            N2(AS4),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N2(F3),
            NOP,
            N2(A3),
            N2(G3),
            NOP,
            NOP,
            NOP,
            N2(B3),
            N2(C4),
            N2(GS3),
            N2(AS3),
            N2(FS3),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N2(CS4),
            NOP,
            N2(F4),
            N2(DS4),
            NOP,
            NOP,
            NOP,
            N2(G4),
            N2(GS4),
            N2(E4),
            N2(FS4),
            N2(D4),
            NOP,
        ],
        [
            NOP,
            NOP,
            NOP,
            N2(CS2),
            NOP,
            N2(F2),
            N2(DS2),
            NOP,
            NOP,
            NOP,
            N2(G2),
            N2(GS2),
            N2(E2),
            N2(FS2),
            N2(D2),
            NOP,
        ],
        [
            N1(E6),
            N1(FS6),
            N1(GS6),
            NOP,
            N1(CS6),
            NOP,
            NOP,
            N1(DS6),
            N1(F6),
            N1(G6),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N1(D6),
        ],
        [
            N1(C7),
            N1(D7),
            N1(E7),
            NOP,
            N1(A6),
            NOP,
            NOP,
            N1(B6),
            N1(CS7),
            N1(DS7),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N1(AS6),
        ],
        [
            N1(GS7),
            N1(AS7),
            N1(C8),
            NOP,
            N1(F7),
            NOP,
            NOP,
            N1(G7),
            N1(A7),
            N1(B7),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N1(FS7),
        ],
        [
            N2(GS7),
            N2(AS7),
            N2(C8),
            NOP,
            N2(F7),
            NOP,
            NOP,
            N2(G7),
            N2(A7),
            N2(B7),
            NOP,
            NOP,
            NOP,
            NOP,
            NOP,
            N2(FS7),
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

    defmt::debug!("main: init i2c");
    let sda = p.PIN_16;
    let scl = p.PIN_17;

    let mut i2c_config = i2c::Config::default();
    let freq = 400_000;
    i2c_config.frequency = freq;
    let i2c = i2c::I2c::new_blocking(p.I2C0, scl, sda, i2c_config);

    defmt::debug!("main: starting transparent pin driver");
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

    defmt::info!("main: starting piano task");
    _spawner.spawn(piano_task(pin_driver)).unwrap();

    defmt::info!("main: starting sustain pedal task");
    _spawner
        .spawn(matrix::pedal(
            midi::Controller::SustainPedal,
            p.PIN_8.into(),
            true,
        ))
        .unwrap();
}
