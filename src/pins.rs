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

use embassy_rp::{
    gpio::{AnyPin, Flex, Pull},
    i2c::{self, Blocking},
    peripherals::I2C0,
};

use mcp23017;
use mcp23017::MCP23017;

/// Number of pins driven by each MCP23017 pin extender.
pub const PINS_PER_EXTENDER: usize = 16;
/// Number of MCP23017 chips used.
pub const N_PIN_EXTENDERS: usize = 2;
/// Number of pins driven directly by the board.
pub const N_REGULAR_PINS: usize = 0;
/// Number of total extended pins
pub const N_EXTENDED_PINS: usize = PINS_PER_EXTENDER * N_PIN_EXTENDERS;

type I2cPeripheral = i2c::I2c<'static, I2C0, Blocking>;
type I2cBus = shared_bus::BusManagerSimple<I2cPeripheral>;

/// GPIO extender pin
struct ExtendedPin {
    /// Index of extender being used
    ext_id: usize,
    /// Pin number in the extender's addressing scheme
    loc_pin: u8,
}

enum TransparentPin {
    /// On-board GPIO (this is an index into `TransparentPins::pins` not the Pico numbering)
    Onboard(usize),
    /// Extender pin
    Extended(ExtendedPin),
}

#[derive(Debug)]
pub enum Error {
    InvalidPin(u8),
    I2cError(i2c::Error),
    ExtenderError,
}

impl From<i2c::Error> for Error {
    fn from(err: i2c::Error) -> Error {
        Error::I2cError(err)
    }
}

impl<E> From<mcp23017::Error<E>> for Error {
    fn from(_err: mcp23017::Error<E>) -> Error {
        Error::ExtenderError
    }
}

/// "Transparent pins" to consistently interface with a GPIO extender + onboard GPIO ports.
///
/// This interface uses a single addressing scheme for all the pins it manages.
/// `ext[0]` is 0-15, `ext[1]` is 16-31, regular pins are 32-63.
pub struct TransparentPins {
    addrs: [u8; N_PIN_EXTENDERS],
    pins: [Flex<'static, AnyPin>; N_REGULAR_PINS],
    i2c_bus: I2cBus,
}

/// Create a new short-lived MCP23017 struct.
///
/// This is needed because our bus proxy uses references to the bus,
/// and having long-lived references angers the borrow-checker
macro_rules! extender {
    ($self:ident,$ext_id:expr) => {
        MCP23017::new($self.i2c_bus.acquire_i2c(), $self.addrs[$ext_id])
    };
}

impl TransparentPins {
    fn get_pin(&mut self, pin: u8) -> Result<TransparentPin, Error> {
        if pin as usize >= N_EXTENDED_PINS + N_REGULAR_PINS {
            return Err(Error::InvalidPin(pin));
        }
        if pin < (N_EXTENDED_PINS as u8) {
            let ext_id = (pin as usize) / PINS_PER_EXTENDER;
            let loc_pin = pin % (PINS_PER_EXTENDER as u8);
            Ok(TransparentPin::Extended(ExtendedPin { ext_id, loc_pin }))
        } else {
            Ok(TransparentPin::Onboard(pin as usize - N_EXTENDED_PINS))
        }
    }

    pub fn new(
        i2c: i2c::I2c<'static, I2C0, Blocking>,
        addrs: [u8; N_PIN_EXTENDERS],
        pins: [AnyPin; N_REGULAR_PINS],
    ) -> Self {
        TransparentPins {
            addrs,
            pins: pins.map(|x| Flex::new(x)),
            i2c_bus: shared_bus::BusManagerSimple::new(i2c),
        }
    }

    /// Read all pins into a single 64-bit value.
    ///
    /// For a given extender's range, port B is in the lower byte and port A in the upper byte.
    pub fn read_all(&mut self) -> Result<u64, Error> {
        log::trace!("read_all: called");
        let mut ret: u64 = 0;
        for i in 0..N_PIN_EXTENDERS {
            let mut ext = extender!(self, i)?;
            ret |= (ext.read_gpioab()? as u64) << (i * PINS_PER_EXTENDER);
        }
        for pin in 0..N_REGULAR_PINS {
            log::trace!("pin read: {}", pin);
            ret |= (self.pins[pin].is_high() as u64) << (N_EXTENDED_PINS + pin);
        }

        Ok(ret)
    }

    /// Set the pull on an individual pin (0-index).
    ///
    /// Note: MCP23017 pins do not support pull-down.
    pub fn set_pull(&mut self, pin: u8, pull: Pull) -> Result<(), Error> {
        let pin = self.get_pin(pin)?;
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
                extender!(self, p.ext_id)?.pull_up(p.loc_pin, pull_on)?
            }
        }
        Ok(())
    }

    pub fn set_input(&mut self, pin: u8) -> Result<(), Error> {
        let pin = self.get_pin(pin)?;
        match pin {
            TransparentPin::Onboard(p) => self.pins[p].set_as_input(),
            TransparentPin::Extended(p) => {
                extender!(self, p.ext_id)?.pin_mode(p.loc_pin, mcp23017::PinMode::INPUT)?
            }
        }
        Ok(())
    }

    pub fn set_output(&mut self, pin: u8) -> Result<(), Error> {
        let pin = self.get_pin(pin)?;
        match pin {
            TransparentPin::Onboard(p) => self.pins[p].set_as_output(),
            TransparentPin::Extended(p) => {
                extender!(self, p.ext_id)?.pin_mode(p.loc_pin, mcp23017::PinMode::OUTPUT)?
            }
        }
        Ok(())
    }
}
