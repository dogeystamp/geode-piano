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

//! MIDI utilities
//!
//! This sets up a queue of MIDI packets to send on behalf of other tasks.

use embassy_rp::usb::{Driver, Instance};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_usb::{class::midi::MidiClass, driver::EndpointError};

////////////////////////////////
////////////////////////////////
// MIDI message types
////////////////////////////////
////////////////////////////////

struct NoteMsg {
    on: bool,
    note: Note,
    velocity: u8,
}

impl NoteMsg {
    fn new(on: bool, note: Note, velocity: u8) -> Self {
        NoteMsg { on, note, velocity }
    }
}

#[derive(Copy, Clone)]
pub enum Controller {
    SustainPedal = 64,
}

struct ControllerMsg {
    controller: Controller,
    value: u8,
}

impl ControllerMsg {
    fn new(controller: Controller, value: u8) -> Self {
        ControllerMsg { controller, value }
    }
}

enum MsgType {
    Note(NoteMsg),
    Controller(ControllerMsg),
}

struct MidiMsg {
    msg: MsgType,
    channel: u8,
}

impl MidiMsg {
    fn new(msg: MsgType, channel: u8) -> Self {
        MidiMsg {
            msg,
            channel: channel & 0xf,
        }
    }
}

////////////////////////////////
////////////////////////////////
// Public MIDI interface
////////////////////////////////
////////////////////////////////

/// Note identifiers
///
/// See src/midi/note_def.py for how this is generated
#[derive(Clone, Copy)]
pub enum Note {
    A0 = 21,
    AS0 = 22,
    B0 = 23,
    C1 = 24,
    CS1 = 25,
    D1 = 26,
    DS1 = 27,
    E1 = 28,
    F1 = 29,
    FS1 = 30,
    G1 = 31,
    GS1 = 32,
    A1 = 33,
    AS1 = 34,
    B1 = 35,
    C2 = 36,
    CS2 = 37,
    D2 = 38,
    DS2 = 39,
    E2 = 40,
    F2 = 41,
    FS2 = 42,
    G2 = 43,
    GS2 = 44,
    A2 = 45,
    AS2 = 46,
    B2 = 47,
    C3 = 48,
    CS3 = 49,
    D3 = 50,
    DS3 = 51,
    E3 = 52,
    F3 = 53,
    FS3 = 54,
    G3 = 55,
    GS3 = 56,
    A3 = 57,
    AS3 = 58,
    B3 = 59,
    C4 = 60,
    CS4 = 61,
    D4 = 62,
    DS4 = 63,
    E4 = 64,
    F4 = 65,
    FS4 = 66,
    G4 = 67,
    GS4 = 68,
    A4 = 69,
    AS4 = 70,
    B4 = 71,
    C5 = 72,
    CS5 = 73,
    D5 = 74,
    DS5 = 75,
    E5 = 76,
    F5 = 77,
    FS5 = 78,
    G5 = 79,
    GS5 = 80,
    A5 = 81,
    AS5 = 82,
    B5 = 83,
    C6 = 84,
    CS6 = 85,
    D6 = 86,
    DS6 = 87,
    E6 = 88,
    F6 = 89,
    FS6 = 90,
    G6 = 91,
    GS6 = 92,
    A6 = 93,
    AS6 = 94,
    B6 = 95,
    C7 = 96,
    CS7 = 97,
    D7 = 98,
    DS7 = 99,
    E7 = 100,
    F7 = 101,
    FS7 = 102,
    G7 = 103,
    GS7 = 104,
    A7 = 105,
    AS7 = 106,
    B7 = 107,
    C8 = 108,
    CS8 = 109,
    D8 = 110,
    DS8 = 111,
    E8 = 112,
    F8 = 113,
    FS8 = 114,
    G8 = 115,
    GS8 = 116,
    A8 = 117,
    AS8 = 118,
    B8 = 119,
}

#[derive(Clone, Copy)]
pub enum KeyAction {
    /// Switch that is first triggered when pressing a key.
    N1(Note),
    /// Switch triggered when key bottoms out.
    N2(Note),
    /// Basic switch with fixed velocity. Be careful not to mix with actions with velocity detection.
    N(Note, u8),
    /// NOP
    NOP,
}

pub struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

static MIDI_QUEUE: Channel<ThreadModeRawMutex, MidiMsg, 3> = Channel::new();

/// Handle sending MIDI until connection breaks
pub async fn midi_session<'d, T: Instance + 'd>(
    midi: &mut MidiClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    loop {
        let msg = MIDI_QUEUE.receive().await;
        match msg.msg {
            MsgType::Note(note) => {
                let status: u8 = (if note.on { 0b1001_0000 } else { 0b1000_0000 }) | msg.channel;
                // i'll be honest i have no idea where the first number here comes from
                let packet = [8, status, note.note as u8, note.velocity];
                log::trace!("midi_session: note {:?}", packet);
                midi.write_packet(&packet).await?
            }
            MsgType::Controller(ctrl) => {
                let status: u8 = (0b1011_0000) | msg.channel;
                let packet = [8, status, ctrl.controller as u8, ctrl.value];
                log::trace!("midi_session: control {:?}", packet);
                midi.write_packet(&packet).await?
            }
        }
    }
}

/// Public MIDI interface that can be used to send notes/control packets.
pub struct MidiChannel {
    channel: u8,
}

impl MidiChannel {
    pub fn new(channel: u8) -> Self {
        MidiChannel { channel }
    }

    /// MIDI Note-On
    pub async fn note_on(&self, note: Note, velocity: u8) {
        MIDI_QUEUE
            .send(MidiMsg::new(
                MsgType::Note(NoteMsg::new(true, note, velocity)),
                self.channel,
            ))
            .await;
    }

    /// MIDI Note-Off
    pub async fn note_off(&self, note: Note, velocity: u8) {
        MIDI_QUEUE
            .send(MidiMsg::new(
                MsgType::Note(NoteMsg::new(false, note, velocity)),
                self.channel,
            ))
            .await;
    }

    /// MIDI Controller (e.g. sustain pedal on/off)
    pub async fn controller(&self, ctrl: Controller, value: u8) {
        MIDI_QUEUE
            .send(MidiMsg::new(
                MsgType::Controller(ControllerMsg::new(ctrl, value)),
                self.channel,
            ))
            .await;
    }
}
