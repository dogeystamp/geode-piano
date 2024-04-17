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

//! Scanner utility to detect which pins are directly connected.
//! This can be useful to reverse-engineer a key-matrix.

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

/// Represents a connection between two pins as detected by the scanner.
#[derive(Clone, Copy)]
struct Connection {
    /// Active low pin number
    gnd_pin: u8,
    /// Pull-up input pin number
    input_pin: u8,
}

#[embassy_executor::task]
async fn scanner_task(mut pin_driver: pins::TransparentPins) {
    log::info!("scanner_task: setting pins as input");
    for i in 0..pin_driver.n_total_pins {
        unwrap(pin_driver.set_input(i as u8)).await;
        unwrap(pin_driver.set_pull(i as u8, gpio::Pull::Up)).await;
    }

    loop {
        const MAX_CONNECTIONS: usize = 10;
        let mut n_connections = 0;
        let mut connections: [Option<Connection>; MAX_CONNECTIONS] = [None; MAX_CONNECTIONS];

        // for all outputs, use active low
        // (only one pin will be output at a time)
        unwrap(pin_driver.write_all(0)).await;
        log::info!("");
        log::info!("---");
        log::info!("STARTING SCAN...");
        for gnd_pin in 0..pin_driver.n_total_pins {
            let gnd_pin = gnd_pin as u8;
            unwrap(pin_driver.set_output(gnd_pin)).await;
            let input = unwrap(pin_driver.read_all()).await;
            unwrap(pin_driver.set_input(gnd_pin)).await;

            // this represents the pins that are different from expected
            let mask = input ^ (((1 << pin_driver.n_total_pins) - 1) ^ (1 << gnd_pin));
            for input_pin in 0..pin_driver.n_total_pins {
                let input_pin = input_pin as u8;
                if ((1 << input_pin) & mask) != 0 {
                    if n_connections < MAX_CONNECTIONS {
                        connections[n_connections] = Some(Connection { gnd_pin, input_pin });
                        n_connections += 1;
                    }
                }
            }
            // this should avoid overexerting the components
            // in total it will take 0.4 seconds per scan
            Timer::after_millis(10).await;
        }

        log::info!("SCAN RESULTS");
        for i in 0..n_connections {
            match connections[i] {
                None => {}
                Some(con) => {
                    log::warn!("GND {:0>2} -> INPUT {:0>2}", con.gnd_pin, con.input_pin);
                }
            }
        }
        if n_connections < MAX_CONNECTIONS {
            log::info!("{n_connections} connections found.");
        } else {
            log::warn!("more than maximum ({MAX_CONNECTIONS}) connections found. list has been truncated. this might mean you used pins GPA7 or GPB7 on the MCP23017, which are unsafe as inputs, and therefore set as outputs.");
        }
        Timer::after_millis(3000).await;
    }
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

    Timer::after_secs(2).await;

    log::info!("main: init i2c");
    let sda = p.PIN_16;
    let scl = p.PIN_17;

    let mut i2c_config = i2c::Config::default();
    let freq = 100_000;
    i2c_config.frequency = freq;
    let i2c = i2c::I2c::new_blocking(p.I2C0, scl, sda, i2c_config);

    log::info!("main: starting transparent pin driver");
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

    log::info!("main: starting scanner task");
    _spawner.spawn(scanner_task(pin_driver)).unwrap();
}
