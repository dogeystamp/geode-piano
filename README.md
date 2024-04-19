# geode-piano

Digital piano firmware for the Raspberry Pi Pico.
This project only attempts to expose the keyboard as a MIDI device.

## installation

- Clone project.
- Go into project directory.
- Install `elf2uf2-rs`.
- Follow the materials and wiring sections below.
- Set the Pico into BOOTSEL mode:
    - Hold down the BOOTSEL button on the Pico. Keep holding it during the following steps.
    - Reset the Pico: either replug the power, or short Pin 30 (RUN) to GND through a button or wire.
- Mount the Pico's storage on your device.
- `cargo run --release --bin [binary]`
    - `[binary]` can be any binary under `src/bin/`. Run `cargo run --bin` to list them.

If you are missing dependencies, consult [Alex Wilson's guide](https://www.alexdwilson.dev/learning-in-public/how-to-program-a-raspberry-pi-pico) on Rust Pico development.

## materials

- 1 Raspberry Pi Pico (preferably with pre-soldered headers)
- 2 MCP23017 I/O extender chips, DIP format
- 2 pull-up resistors for I²C (1-10kΩ), these are optional but recommended
- 1 USB to Micro-USB cable with data transfer
- Many jumper cables
- Breadboard

## wiring

**Ensure all wires are well plugged in every time you use this circuit.** 

### rails

- Pin 3 -> GND rail
- Pin 36 (3V3OUT) -> power (positive) rail

### i2c

Let's call the closest MCP23017 chip to the Pico MCP A, and the further one MCP B.

- GP16 -> MCP A SDA
- GP17 -> MCP A SCL
- MCP A SDA -> MCP B SDA
- MCP A SCL -> MCP B SCL
- Pull-up resistor from GP16 to power rail
- Pull-up resistor from GP17 to power rail

For both MCP23017s:

- MCP RESET -> power rail
- MCP A0, A1, A2 -> GND rail for 0, power rail for 1
    - MCP A should be 0x20 (GND, GND, GND), MCP B 0x27 (3V3, 3V3, 3V3)
- MCP VDD -> power rail
- MCP VSS -> GND rail
