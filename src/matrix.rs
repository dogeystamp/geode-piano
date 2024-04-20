//! Key matrix scanner

use crate::pins;
use crate::midi;
use crate::unwrap;
use embassy_rp::gpio;
use embassy_time::{Duration, Ticker};

/// Key matrix for the piano.
pub struct KeyMatrix<const N_ROWS: usize, const N_COLS: usize> {
    /// GND pins at the top of each column
    col_pins: [u8; N_COLS],
    /// Input pins at the left of each row
    row_pins: [u8; N_ROWS],
    keymap: [[midi::Note; N_ROWS]; N_COLS],
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
        keymap: [[midi::Note; N_ROWS]; N_COLS],
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
        let mut ticker = Ticker::every(Duration::from_millis(10));
        let chan = midi::MidiChannel::new(0);
        let mut note_on = [false; 128];

        loop {
            for (i, col) in self.col_pins.iter().enumerate() {
                unwrap(pin_driver.set_output(*col)).await;
                let input = unwrap(pin_driver.read_all()).await;
                unwrap(pin_driver.set_input(*col)).await;

                // values that are logical ON
                let mask = input ^ (((1 << pin_driver.n_usable_pins()) - 1) ^ (1 << col));
                for (j, row) in self.row_pins.iter().enumerate() {
                    let note = self.keymap[i][j];
                    if mask & (1 << row) != 0 {
                        if !note_on[note as usize] {
                            note_on[note as usize] = true;
                            chan.note_on(note, 40).await;
                        }
                    } else if note_on[note as usize] {
                        note_on[note as usize] = false;
                        chan.note_off(note, 0).await;
                    }
                }
            }
            ticker.next().await;
        }
    }
}
