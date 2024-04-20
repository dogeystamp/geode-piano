//! Key matrix scanner + other interfacing utilities

use crate::midi;
use crate::pins;
use crate::unwrap;
use core::cmp::min;
use embassy_rp::gpio;
use embassy_time::{Duration, Instant, Ticker};

/// Task to handle pedals in MIDI
#[embassy_executor::task]
pub async fn pedal(pedal: midi::Controller, pin: gpio::AnyPin) {
    let mut inp = gpio::Input::new(pin, gpio::Pull::Up);
    let chan = midi::MidiChannel::new(0);
    loop {
        inp.wait_for_low().await;
        chan.controller(pedal, 64).await;
        inp.wait_for_high().await;
        chan.controller(pedal, 0).await;
    }
}

/// Key matrix for the piano.
pub struct KeyMatrix<const N_ROWS: usize, const N_COLS: usize> {
    /// GND pins at the top of each column
    col_pins: [u8; N_COLS],
    /// Input pins at the left of each row
    row_pins: [u8; N_ROWS],
    keymap: [[midi::KeyAction; N_ROWS]; N_COLS],
}

impl<const N_ROWS: usize, const N_COLS: usize> KeyMatrix<N_ROWS, N_COLS> {
    /// New function.
    ///
    /// `col_pins` are GND pins at the top of the columns, and `row_pins` are the input pins at
    /// the ends of the rows.
    ///
    /// `keymap` represents the note that every combination of col/row gives.
    pub fn new(
        col_pins: [u8; N_COLS],
        row_pins: [u8; N_ROWS],
        keymap: [[midi::KeyAction; N_ROWS]; N_COLS],
    ) -> Self {
        KeyMatrix {
            col_pins,
            row_pins,
            keymap,
        }
    }

    pub async fn scan(&mut self, mut pin_driver: pins::TransparentPins) {
        for i in pin_driver.pins {
            unwrap(pin_driver.set_input(i)).await;
            unwrap(pin_driver.set_pull(i, gpio::Pull::Up)).await;
        }

        // scan frequency
        // this might(?) panic if the scan takes longer than the tick
        let mut ticker = Ticker::every(Duration::from_millis(2));

        let chan = midi::MidiChannel::new(0);
        const MAX_NOTES: usize = 128;

        // is note currently on
        let mut note_on = [false; MAX_NOTES];
        // (for velocity detection) moment key is first touched
        let mut note_first: [Option<Instant>; MAX_NOTES] = [None; MAX_NOTES];

        loop {
            for (i, col) in self.col_pins.iter().enumerate() {
                unwrap(pin_driver.set_output(*col)).await;
                let input = unwrap(pin_driver.read_all()).await;
                unwrap(pin_driver.set_input(*col)).await;

                // values that are logical ON
                let mask = input ^ (((1 << pin_driver.n_usable_pins()) - 1) ^ (1 << col));
                for (j, row) in self.row_pins.iter().enumerate() {
                    let key_action = self.keymap[i][j];
                    let key_active = mask & (1 << row) != 0;
                    match key_action {
                        midi::KeyAction::N1(note) => {
                            if !note_on[note as usize]
                                && note_first[note as usize].is_none()
                                && key_active
                            {
                                note_first[note as usize] = Some(Instant::now());
                            }
                        }
                        midi::KeyAction::N2(note) => {
                            if key_active {
                                if note_first[note as usize].is_some() && !note_on[note as usize] {
                                    // millisecond duration of keypress
                                    let dur =
                                        note_first[note as usize].unwrap().elapsed().as_millis();
                                    // 1905 millis is the minimum velocity
                                    let velocity: u8 = 127 - min(dur / 15, 127) as u8;
                                    note_first[note as usize] = None;
                                    note_on[note as usize] = true;
                                    chan.note_on(note, velocity).await;
                                }
                            } else if note_on[note as usize] {
                                note_on[note as usize] = false;
                                note_first[note as usize] = None;
                                chan.note_off(note, 0).await;
                            }
                        }
                        midi::KeyAction::N(note, velocity) => {
                            if key_active {
                                if !note_on[note as usize] {
                                    note_on[note as usize] = true;
                                    chan.note_on(note, velocity).await;
                                }
                            } else if note_on[note as usize] {
                                note_on[note as usize] = false;
                                chan.note_off(note, 0).await;
                            }
                        }
                    }
                }
            }
            ticker.next().await;
        }
    }
}
