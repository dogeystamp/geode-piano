//! Key matrix scanner + other interfacing utilities

use crate::midi;
use crate::pins;
use crate::unwrap;
use core::cmp::{max, min};
use embassy_rp::gpio;
use embassy_time::{Duration, Instant, Timer};

pub enum NormalState {
    /// Normal open
    NO,
    /// Normal closed
    NC,
}

/// Profile to map from key press duration to MIDI velocity.
/// https://www.desmos.com/calculator/mynk7thhzp
pub enum VelocityProfile {
    Linear,
    Heavy,
    Light,
}

fn velocity_light(us: u64) -> u8 {
    if us <= 60000 {
        min(127, (135000 - us * 6 / 5) / 1000) as u8
    } else {
        (127 - min(us, 240000) / 4000 - 60) as u8
    }
}

fn velocity_heavy(us: u64) -> u8 {
    if us <= 17000 {
        ((113000 - us) / 1000) as u8
    } else {
        ((127000 - min(us, 190000) / 2 - 22000) / 1000) as u8
    }
}

fn velocity_linear(us: u64) -> u8 {
    (max(120900 - (us as i32), 5000) / 1000) as u8
}

pub struct Config {
    pub velocity_prof: VelocityProfile,
}

/// Task to handle pedals in MIDI
///
/// `norm_open` represents a normally open switch
#[embassy_executor::task]
pub async fn pedal(pedal: midi::Controller, pin: gpio::AnyPin, norm_state: NormalState) {
    let mut inp = gpio::Input::new(pin, gpio::Pull::Up);
    let chan = midi::MidiChannel::new(0);
    loop {
        let (off_val, on_val) = match norm_state {
            NormalState::NO => (0, 64),
            NormalState::NC => (64, 0),
        };
        inp.wait_for_low().await;
        chan.controller(pedal, on_val).await;
        defmt::debug!("{} set to {}", pedal, on_val);
        inp.wait_for_high().await;
        chan.controller(pedal, off_val).await;
        defmt::debug!("{} set to {}", pedal, off_val);
    }
}

/// Key matrix for the piano.
pub struct KeyMatrix<const N_ROWS: usize, const N_COLS: usize> {
    /// GND pins at the top of each column
    col_pins: [u8; N_COLS],
    /// Input pins at the left of each row
    row_pins: [u8; N_ROWS],
    keymap: [[midi::KeyAction; N_COLS]; N_ROWS],
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
        keymap: [[midi::KeyAction; N_COLS]; N_ROWS],
    ) -> Self {
        KeyMatrix {
            col_pins,
            row_pins,
            keymap,
        }
    }

    pub async fn scan(&mut self, mut pin_driver: pins::TransparentPins, config: Config) {
        for i in pin_driver.pins {
            unwrap(pin_driver.set_input(i)).await;
            unwrap(pin_driver.set_pull(i, gpio::Pull::Up)).await;
        }

        let chan = midi::MidiChannel::new(0);
        const MAX_NOTES: usize = 128;

        // (for velocity detection) moment key is first touched
        let mut note_first: [Option<Instant>; MAX_NOTES] = [None; MAX_NOTES];
        // (for debouncing) moment note was last on
        let mut note_on: [Option<Instant>; MAX_NOTES] = [None; MAX_NOTES];

        let mut counter = 0;
        let mut prof_col_idx = 0;

        defmt::debug!("using {} columns", N_COLS);

        loop {
            let profile: bool = counter == 0;
            counter += 1;
            counter %= 5000;
            let _prof_start = Instant::now();
            let mut _prof_time_last_col = _prof_start;
            let mut _prof_dur_col = Duration::from_ticks(0);

            for (i, col) in self.col_pins.iter().enumerate() {
                unwrap(pin_driver.set_output(*col)).await;
                let input = unwrap(pin_driver.read_all()).await;
                unwrap(pin_driver.set_input(*col)).await;

                if profile && i == prof_col_idx {
                    _prof_dur_col = _prof_time_last_col.elapsed();
                }

                // values that are logical ON
                let mask = input ^ (((1 << pin_driver.n_usable_pins()) - 1) ^ (1 << col));
                for (j, row) in self.row_pins.iter().enumerate() {
                    let key_action = self.keymap[j][i];
                    let key_active = mask & (1 << row) != 0;
                    match key_action {
                        midi::KeyAction::N1(note) => {
                            if key_active {
                                if note_first[note as usize].is_none() {
                                    note_first[note as usize] = Some(Instant::now());
                                }
                            } else if note_first[note as usize].is_some() {
                                note_first[note as usize] = None;

                                if let Some(note_on_time) = note_on[note as usize] {
                                    note_on[note as usize] = None;
                                    chan.note_off(note, 0).await;
                                    defmt::debug!(
                                        "turned off note {} after {} us",
                                        note,
                                        note_on_time.elapsed().as_micros()
                                    );
                                }
                            }
                        }
                        midi::KeyAction::N2(note) => {
                            if key_active {
                                if note_first[note as usize].is_some()
                                    && note_on[note as usize].is_none()
                                {
                                    // microsecond duration of keypress
                                    let dur =
                                        note_first[note as usize].unwrap().elapsed().as_micros();
                                    let velocity = match config.velocity_prof {
                                        VelocityProfile::Heavy => velocity_heavy(dur),
                                        VelocityProfile::Linear => velocity_linear(dur),
                                        VelocityProfile::Light => velocity_light(dur),
                                    };
                                    defmt::debug!(
                                        "{} velocity {} from dur {}us",
                                        note,
                                        velocity,
                                        dur
                                    );
                                    note_on[note as usize] = Some(Instant::now());
                                    chan.note_on(note, velocity).await;
                                } else if note_on[note as usize].is_some() {
                                    // keep refreshing the note
                                    note_on[note as usize] = Some(Instant::now());
                                }
                            }
                        }
                        midi::KeyAction::N(note, velocity) => {
                            if key_active {
                                if note_on[note as usize].is_none() {
                                    note_on[note as usize] = Some(Instant::now());
                                    chan.note_on(note, velocity).await;
                                }
                            } else if note_on[note as usize].is_some() {
                                note_on[note as usize] = None;
                                chan.note_off(note, 0).await;
                            }
                        }
                        midi::KeyAction::NOP => {}
                    }
                }
                _prof_time_last_col = Instant::now();
            }
            if profile {
                let _time_total = _prof_start.elapsed();
                prof_col_idx += 1;
                prof_col_idx %= N_COLS;
                // defmt::debug!(
                //     "profile: total scan took {}us, {}-th column {}us",
                //     time_total.as_micros(),
                //     prof_col_idx,
                //     prof_dur_col.as_micros()
                // );
            }

            // relinquish to other tasks for a moment
            Timer::after_micros(50).await;
        }
    }
}
