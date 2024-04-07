# geode-piano

Digital piano firmware for the Raspberry Pi Pico.
This project only attempts to expose the keyboard as a MIDI device.

## installation

- Clone project.
- Go into project directory.
- `cargo install probe-rs --features cli`
- `cargo run --bin firmware`

If you are missing dependencies, consult [Alex Wilson's guide](https://www.alexdwilson.dev/learning-in-public/how-to-program-a-raspberry-pi-pico) on Rust Pico development.
