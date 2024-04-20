# geode-piano

Digital piano firmware for the Raspberry Pi Pico.
This project only attempts to expose the keyboard as a MIDI device.
The purpose is to revive digital pianos that have working keys, but faulty electronics.

## installation

- Clone project.
- Go into project directory.
- Install the `thumbv6m-none-eabi` target using rustup.
- Install `elf2uf2-rs`.
- Follow the materials and wiring sections below.
- Set the Pico into BOOTSEL mode:
    - Hold down the BOOTSEL button on the Pico. Keep holding it during the following step.
    - Reset the Pico: either replug the power, or short Pin 30 (RUN) to GND through a button or wire.
- Mount the Pico's storage on your device.
- `cargo run --release --bin [binary]`
    - `[binary]` can be any binary under `src/bin/`. Run `cargo run --bin` to list them.

If you are missing dependencies, consult [Alex Wilson's guide](https://www.alexdwilson.dev/learning-in-public/how-to-program-a-raspberry-pi-pico) on Rust Pico development.

## usage

The intended usage is to first plug the device into the piano keyboard, then use the `pin_scanner` binary to
scan the key-matrix. (See the next sections on how to wire it up.)
On every key, press it down half-way and then fully and note the pins connections detected at each level.
These correspond to the [`midi::KeyAction::N1`] and [`midi::KeyAction::N2`] actions respectively.
There should be two switches per key for velocity detection.
If there isn't, then the key is an [`midi::KeyAction::N`] (it will be stuck at a fixed velocity).

Put the connections in a spreadsheet and reorganize it so that GND pins are column headers, and the Input pins are row headers.
This will comprise the keymap.
The keymap is a an array with the same dimensions as the spreadsheet grid.
This is comprised of N1, N2, and N entries, indicating which note a key corresponds to.

Once the keymap is done, run the `piano_firmware` binary and plug the USB cable to your computer.
Open up a DAW and select Geode-Piano as a MIDI input device.
I use LMMS with the [Maestro Concert Grand v2](https://www.linuxsampler.org/instruments.html) samples.
You should be able to play now.

## materials

- 1 Raspberry Pi Pico (preferably with pre-soldered headers)
- 2 MCP23017 I/O extender chips, DIP format
- 2 pull-up resistors for I²C (1-10kΩ), these are optional but [recommended](https://www.joshmcguigan.com/blog/internal-pull-up-resistor-i2c/)
- 1 USB to Micro-USB cable with data transfer
- Ribbon cable sockets. The following is for my own piano, yours might be different:
    - 18-pin 1.25mm pitch FFC connector
    - 22-pin 1.25mm pitch FFC connector
- Many jumper cables
- Breadboard

For the ribbon cable sockets, open up your piano and find the ribbon cables.
Unplug them from the PCB, and count the amount of pins on them.
Also, measure the distance between each pin,
or the distance between the first and last pin.
This will help you find the right pin pitch and pin count.
Usually, these measurements can be found on the datasheets for FFC sockets.

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

### ribbon cables

Connect the following pins to the ribbon cable sockets in any order (use more or less pins depending on how many you need):

- GP15
- GP14
- GP13
- GP12
- GP11
- GP10
- GP9
- GP18
- GP19
- GP20
- GP21
- GP22
- All MCP GPIO pins except GPB7 and GPA7 on both chips (see [datasheet](https://ww1.microchip.com/downloads/aemDocuments/documents/APID/ProductDocuments/DataSheets/MCP23017-Data-Sheet-DS20001952.pdf) for diagram of pins)

GPB7 and GPA7 have known issues and therefore can not be inputs.
Again, refer to the datasheet about this.
It is simpler to exclude them instead of working around that limitation.

I used male-to-female jumpers with the female end trimmed to reveal the metal part inside.
This was necessary to attach to the short pins on the jumpers.
The opening in the plastic on the female end should face inwards when connected to the sockets.

Then, plug the ribbon cables from your piano into the sockets.
