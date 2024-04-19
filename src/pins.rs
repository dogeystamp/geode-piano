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
const PINS_PER_EXTENDER: usize = 16;
/// Number of MCP23017 chips used.
const N_PIN_EXTENDERS: usize = 2;
/// Number of pins driven directly by the board.
const N_REGULAR_PINS: usize = 12;
/// Number of total extended pins
const N_EXTENDED_PINS: usize = PINS_PER_EXTENDER * N_PIN_EXTENDERS;
/// Number of unsafe pins per extender (GPA7, GPB7)
const UNSAFE_PER_EXTENDER: usize = 2;
/// Single extender address offset of PORTA
const PORT_A: u8 = 0;
/// Single extender address offset of PORTB
const PORT_B: u8 = 8;

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
/// This interface uses a single addressing scheme for all the pins it manages. Extender A is 0-15,
/// Extender B is 16-31, and so on, then all the onboard pins. Port A is in the lower byte and port
/// B is in the upper byte of each extender range. This addressing scheme may be changed with some
/// options. The exact pins each address refers to are not supposed to be important.
///
/// The MCP23017 is known to have two defective pins, GPA7 and GPB7. These can not be set as inputs
/// without risks of weird behaviour. To disable these pins, you may set `disable_unsafe_pins` in
/// the constructor. This will set them to output pins, and then remove them from the transparent
/// pins addressing scheme.
pub struct TransparentPins {
    addrs: [u8; N_PIN_EXTENDERS],
    pins: [Flex<'static, AnyPin>; N_REGULAR_PINS],
    i2c_bus: I2cBus,
    disable_unsafe_pins: bool,
    /// Number of total usable pins. Transparent pins all have an address from `0..n_total_pins`.
    pub n_total_pins: usize,
    /// Usable pins per extender. Depends on `disable_unsafe_pins`.
    usable_pins_per_extender: usize,
    /// Usable pin count on all extenders. Depends on `disable_unsafe_pins`.
    usable_extended_pins: usize,
}

/// Helper to define the onboard pins in [`TransparentPins`]
#[macro_export]
macro_rules! pin_array {
    ($($pin: expr),*) => {
        [$($pin.into(),)*]
    }
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
    /// Transform addresses into a transparent pin number, taking into account pins that aren't being used.
    fn addr_to_pin(&self, addr: u8) -> u8 {
        if self.disable_unsafe_pins {
            if addr as usize >= (self.usable_pins_per_extender * N_PIN_EXTENDERS) {
                return addr + (UNSAFE_PER_EXTENDER as u8) * (N_PIN_EXTENDERS as u8);
            }
            // extender index
            let div = addr as usize / self.usable_pins_per_extender;
            // offset within extender
            let m = addr as usize % self.usable_pins_per_extender;
            // difference between `m` and the MCP23017 pin number within this extender
            let mut offset = 0;
            if m >= PORT_A as usize + 7 {
                // these pins are offset by one because GPA7 is missing
                offset += 1
            }
            // GPB7 doesn't need an offset because it is the last pin anyways
            // the div-mod above takes care of that

            (div * PINS_PER_EXTENDER + m + offset) as u8
        } else {
            addr
        }
    }

    /// Get a pin by its pin number.
    ///
    /// This is NOT by the transparent address.
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
        disable_unsafe_pins: bool,
    ) -> Result<Self, Error> {
        let mut ret = TransparentPins {
            addrs,
            pins: pins.map(Flex::new),
            i2c_bus: shared_bus::BusManagerSimple::new(i2c),
            disable_unsafe_pins: false,
            usable_pins_per_extender: PINS_PER_EXTENDER,
            usable_extended_pins: N_EXTENDED_PINS,
            n_total_pins: N_EXTENDED_PINS + N_REGULAR_PINS,
        };
        if disable_unsafe_pins {
            for i in 0..N_PIN_EXTENDERS {
                ret.set_output((i as u8) * (PINS_PER_EXTENDER as u8) + PORT_A + 7)?;
                ret.set_output((i as u8) * (PINS_PER_EXTENDER as u8) + PORT_B + 7)?;
                ret.usable_pins_per_extender = PINS_PER_EXTENDER - UNSAFE_PER_EXTENDER;
                ret.usable_extended_pins = N_PIN_EXTENDERS * ret.usable_pins_per_extender;
                ret.n_total_pins = ret.usable_extended_pins + N_REGULAR_PINS;
            }
            ret.disable_unsafe_pins = true;
            log::debug!("TransparentPins: {} usable pins", ret.n_total_pins)
        }
        Ok(ret)
    }

    /// Convert the raw pin input for an extender to just usable pins
    fn raw_to_usable(&self, val: u16) -> u16 {
        if self.disable_unsafe_pins {
            // read api is wonky (https://github.com/lucazulian/mcp23017/issues/8)
            // ports are flipped from what it should be
            let port_a = (val & (0xff00)) >> 8;
            let port_b = val & (0x00ff);
            log::trace!("raw_to_usable: raw {val:016b} a {port_a:08b} b {port_b:08b}");
            (port_a & 0x7f) | ((port_b & 0x7f) << 7)
        } else {
            val
        }
    }

    // Convert the usable pin mask to raw pin output
    fn usable_to_raw(&self, val: u16) -> u16 {
        if self.disable_unsafe_pins {
            (val & 0x00ff) | ((val & 0xff00) << 1)
        } else {
            val
        }
    }

    /// Write all pins from a single 64-bit value.
    pub fn write_all(&mut self, val: u64) -> Result<(), Error> {
        log::trace!("write_all: called with val {}", val);
        for i in 0..N_PIN_EXTENDERS {
            // value for this extender
            let ext_val = (val >> (i * self.usable_pins_per_extender))
                & ((1 << self.usable_pins_per_extender) - 1);
            extender!(self, i)?.write_gpioab(self.usable_to_raw(ext_val as u16))?;
        }
        for pin in 0..N_REGULAR_PINS {
            self.pins[pin].set_level(match (val >> self.usable_extended_pins >> pin) & 1 {
                0 => embassy_rp::gpio::Level::Low,
                1 => embassy_rp::gpio::Level::High,
                _ => panic!("Invalid level"),
            })
        }

        Ok(())
    }

    /// Read all pins into a single 64-bit value.
    pub fn read_all(&mut self) -> Result<u64, Error> {
        log::trace!("read_all: called");
        let mut ret: u64 = 0;
        for i in 0..N_PIN_EXTENDERS {
            let mut ext = extender!(self, i)?;
            let read_val = ext.read_gpioab()?;
            ret |= (self.raw_to_usable(read_val) as u64) << (i * self.usable_pins_per_extender);
        }
        for pin in 0..N_REGULAR_PINS {
            ret |= (self.pins[pin].is_high() as u64) << (self.usable_extended_pins + pin);
        }

        Ok(ret)
    }

    /// Set the pull on an individual pin (0-index).
    ///
    /// Note: MCP23017 pins do not support pull-down.
    pub fn set_pull(&mut self, addr: u8, pull: Pull) -> Result<(), Error> {
        let pin_n = self.addr_to_pin(addr);
        let pin = self.get_pin(pin_n)?;
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

    /// Sets a pin as an input.
    pub fn set_input(&mut self, addr: u8) -> Result<(), Error> {
        let pin_n = self.addr_to_pin(addr);
        let pin = self.get_pin(pin_n)?;
        match pin {
            TransparentPin::Onboard(p) => self.pins[p].set_as_input(),
            TransparentPin::Extended(p) => {
                extender!(self, p.ext_id)?.pin_mode(p.loc_pin, mcp23017::PinMode::INPUT)?
            }
        }
        Ok(())
    }

    /// Sets a pin as an output.
    pub fn set_output(&mut self, addr: u8) -> Result<(), Error> {
        let pin_n = self.addr_to_pin(addr);
        let pin = self.get_pin(pin_n)?;
        match pin {
            TransparentPin::Onboard(p) => self.pins[p].set_as_output(),
            TransparentPin::Extended(p) => {
                extender!(self, p.ext_id)?.pin_mode(p.loc_pin, mcp23017::PinMode::OUTPUT)?
            }
        }
        Ok(())
    }
}
