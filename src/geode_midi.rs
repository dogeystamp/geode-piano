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

use embassy_rp::{
    peripherals::USB,
    usb::{Driver, Instance},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_time::Timer;
use embassy_usb::{class::midi::MidiClass, driver::EndpointError};

struct NoteMsg {
    on: bool,
    note: u8,
    velocity: u8,
}

impl NoteMsg {
    fn new(on: bool, note: u8, velocity: u8) -> Self {
        return NoteMsg { on, note, velocity };
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
        return ControllerMsg { controller, value };
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
        return MidiMsg {
            msg,
            channel: channel & 0xf,
        };
    }
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
                let packet = [8, status, note.note, note.velocity];
                log::debug!("midi_session: note {:?}", packet);
                midi.write_packet(&packet).await?
            }
            MsgType::Controller(ctrl) => {
                let status: u8 = (0b1011_0000) | msg.channel;
                let packet = [8, status, ctrl.controller as u8, ctrl.value];
                log::debug!("midi_session: control {:?}", packet);
                midi.write_packet(&packet).await?
            }
        }
    }
}

/// Wrapper to reconnect to MIDI upon disconnection.
#[embassy_executor::task]
pub async fn midi_task(mut midi: MidiClass<'static, Driver<'static, USB>>) -> ! {
    loop {
        log::info!("Connected");
        midi_session(&mut midi);
        log::info!("Disconnected");
    }
}

/// Public MIDI interface that can be used to send notes/control packets.
pub struct MidiChannel {
    channel: u8,
}

impl MidiChannel {
    pub fn new(channel: u8) -> Self {
        return MidiChannel { channel };
    }

    pub async fn note_on(&self, note: u8, velocity: u8) {
        MIDI_QUEUE
            .send(MidiMsg::new(
                MsgType::Note(NoteMsg::new(true, note, velocity)),
                self.channel,
            ))
            .await;
    }

    pub async fn note_off(&self, note: u8, velocity: u8) {
        MIDI_QUEUE
            .send(MidiMsg::new(
                MsgType::Note(NoteMsg::new(false, note, velocity)),
                self.channel,
            ))
            .await;
    }

    pub async fn controller(&self, ctrl: Controller, value: u8) {
        MIDI_QUEUE
            .send(MidiMsg::new(
                MsgType::Controller(ControllerMsg::new(ctrl, value)),
                self.channel,
            ))
            .await;
    }
}
