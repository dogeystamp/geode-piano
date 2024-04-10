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

//! Manage I²C and provide a transparent pin interface for both onboard and MCP23017 pins.

extern crate embedded_hal_02;
use embassy_rp::{
    gpio::{AnyPin, Flex, Pull},
    i2c::{self, Blocking},
    peripherals::{I2C0, I2C1},
};

extern crate mcp23017;
use embassy_time::Timer;
use mcp23017::MCP23017;

/// Number of pins driven by each MCP23017 pin extender.
const PINS_PER_EXTENDER: usize = 16;
/// Number of MCP23017 chips used. This can not be changed without changing code.
const N_PIN_EXTENDERS: usize = 1;
/// Number of pins driven directly by the board.
const N_REGULAR_PINS: usize = 0;
/// Number of total extended pins
const N_EXTENDED_PINS: usize = PINS_PER_EXTENDER * N_PIN_EXTENDERS;

/// "Transparent pins" to consistently interface with a GPIO extender + onboard GPIO ports.
///
/// This interface uses a single addressing scheme for all the pins it manages.
/// ext0 is 0-15, ext1 is 16-31, regular pins are 32-63.
pub struct TransparentPins {
    ext0: MCP23017<i2c::I2c<'static, I2C0, Blocking>>,
    //ext1: MCP23017<i2c::I2c<'static, I2C1, Blocking>>,
    pins: [Flex<'static, AnyPin>; N_REGULAR_PINS],
}

/// GPIO extender pin
struct ExtendedPin {
    /// ID (not address) of the extender being used
    ext_id: u8,
    /// Pin number in the extender's addressing scheme
    loc_pin: u8,
}

enum TransparentPin {
    /// On-board GPIO (this is an index into `TransparentPins::pins` not the Pico numbering)
    Onboard(usize),
    /// Extender pin
    Extended(ExtendedPin),
}

impl TransparentPins {
    fn get_pin(pin: u8) -> TransparentPin {
        if pin < (N_EXTENDED_PINS as u8) {
            let ext_id = pin / (PINS_PER_EXTENDER as u8);
            let loc_pin = pin % (PINS_PER_EXTENDER as u8);
            if ext_id >= N_PIN_EXTENDERS as u8 {
                panic!("invalid pin")
            }
            TransparentPin::Extended(ExtendedPin { ext_id, loc_pin })
        } else {
            TransparentPin::Onboard(pin as usize - N_EXTENDED_PINS)
        }
    }

    pub fn new(
        i2c0: i2c::I2c<'static, I2C0, Blocking>,
        //i2c1: i2c::I2c<'static, I2C1, Blocking>,
        addrs: [u8; N_PIN_EXTENDERS],
        pins: [AnyPin; N_REGULAR_PINS],
    ) -> Self {
        let pin_init = pins.map(|x| Flex::new(x));
        return TransparentPins {
            ext0: MCP23017::new(i2c0, addrs[0]).unwrap(),
            pins: pin_init,
        };
    }

    /// Read all pins into a single 64-bit value.
    pub fn read_all(&mut self) -> Result<u64, i2c::Error> {
        log::trace!("read_all: called");
        let mut ret: u64 = 0;
        // remember here port b is in the lower byte and port a in the upper byte
        ret |= self.ext0.read_gpioab()? as u64;
        for pin in 0..N_REGULAR_PINS {
            log::trace!("pin read: {}", pin);
            ret |= (self.pins[pin].is_high() as u64) << (N_EXTENDED_PINS + pin);
        }

        Ok(ret)
    }

    /// Set the pull on an individual pin (0-index).
    ///
    /// Note: MCP23017 pins do not support pull-down.
    pub fn set_pull(&mut self, pin: u8, pull: Pull) -> Result<(), i2c::Error> {
        let pin = TransparentPins::get_pin(pin);
        match pin {
            TransparentPin::Onboard(p) => {
                self.pins[p].set_pull(pull);
            }
            TransparentPin::Extended(p) => {
                let pull_on: bool = match pull {
                    Pull::None => false,
                    Pull::Up => true,
                    // Extended pins don't seem to support pull-down
                    Pull::Down => unimplemented!("MCP23017 does not support pull-down."),
                };
                match p.ext_id {
                    0 => self.ext0.pull_up(p.loc_pin, pull_on)?,
                    //1 => self.ext1.pull_up(p.loc_pin, pull_on)?,
                    _ => panic!("invalid pin"),
                }
            }
        }
        Ok(())
    }

    pub fn set_input(&mut self, pin: u8) -> Result<(), i2c::Error> {
        let pin = TransparentPins::get_pin(pin);
        match pin {
            TransparentPin::Onboard(p) => self.pins[p].set_as_input(),
            TransparentPin::Extended(p) => {
                match p.ext_id {
                    0 => self.ext0.pin_mode(p.loc_pin, mcp23017::PinMode::INPUT)?,
                    //1 => self.ext1.pin_mode(p.loc_pin, mcp23017::PinMode::INPUT).unwrap(),
                    _ => panic!("invalid pin"),
                }
            }
        }
        Ok(())
    }

    pub fn set_output(&mut self, pin: u8) -> Result<(), i2c::Error> {
        let pin = TransparentPins::get_pin(pin);
        match pin {
            TransparentPin::Onboard(p) => self.pins[p].set_as_output(),
            TransparentPin::Extended(p) => {
                match p.ext_id {
                    0 => self.ext0.pin_mode(p.loc_pin, mcp23017::PinMode::OUTPUT)?,
                    //1 => self.ext1.pin_mode(p.loc_pin, mcp23017::PinMode::OUTPUT).unwrap(),
                    _ => panic!("invalid pin"),
                }
            }
        }
        Ok(())
    }
}
