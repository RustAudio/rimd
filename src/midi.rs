use std::error;
use std::fmt;
use std::convert::From;
use std::io::{Error,Read};

use num_traits::FromPrimitive;

use util::read_byte;

/// An error that can occur trying to parse a midi message
#[derive(Debug)]
pub enum MidiError {
    InvalidStatus(u8),
    OtherErr(&'static str),
    Error(Error),
}

impl From<Error> for MidiError {
    fn from(err: Error) -> MidiError {
        MidiError::Error(err)
    }
}

impl error::Error for MidiError {
    fn description(&self) -> &str {
        match *self {
            MidiError::InvalidStatus(_) => "Midi data has invalid status byte",
            MidiError::OtherErr(_) => "A general midi error has occured",
            MidiError::Error(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            MidiError::Error(ref err) => Some(err as &error::Error),
            _ => None,
        }
    }
}

impl fmt::Display for MidiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MidiError::InvalidStatus(ref s) => write!(f,"Invalid Midi status: {}",s),
            MidiError::OtherErr(ref s) => write!(f,"Midi Error: {}",s),
            MidiError::Error(ref e) => write!(f,"{}",e),
        }
    }
}

/// The status field of a midi message indicates what midi command it
/// represents and what channel it is on
#[derive(Debug, PartialEq, Clone, Copy, FromPrimitive)]
pub enum Status {
    // voice
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyphonicAftertouch = 0xA0,
    ControlChange = 0xB0,
    ProgramChange = 0xC0,
    ChannelAftertouch = 0xD0,
    PitchBend = 0xE0,

    // sysex
    SysExStart = 0xF0,
    MIDITimeCodeQtrFrame = 0xF1,
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    TuneRequest = 0xF6, // F4 anf 5 are reserved and unused
    SysExEnd = 0xF7,
    TimingClock = 0xF8,
    Start = 0xFA,
    Continue = 0xFB,
    Stop = 0xFC,
    ActiveSensing = 0xFE, // FD also res/unused
    SystemReset = 0xFF,
}

/// Midi message building and parsing.  See
/// http://www.midi.org/techspecs/midimessages.php for a description
/// of the various Midi messages that exist.
#[derive(Debug, Default)]
pub struct MidiMessage {
    pub data: Vec<u8>,
}

impl Clone for MidiMessage {
    fn clone(&self) -> MidiMessage {
        MidiMessage {
            data: self.data.clone()
        }
    }
}

pub const STATUS_MASK: u8 = 0xF0;
pub const CHANNEL_MASK: u8 = 0x0F;

// Or in the channel bits to a status
#[inline(always)]
pub fn make_status(status: Status, channel: u8) -> u8 {
    status as u8 | channel
}

impl MidiMessage {
    /// Return the status (type) of this message
    pub fn status(&self) -> Status {
        Status::from_u8(self.data[0] & STATUS_MASK).unwrap()
    }

    /// Return the channel this message is on (TODO: return 0 for messages with no channel)
    pub fn channel(&self) -> Option<u8> {
        match self.status() {
            Status::NoteOff |
            Status::NoteOn |
            Status::PolyphonicAftertouch |
            Status::ControlChange |
            Status::ProgramChange |
            Status::ChannelAftertouch |
            Status::PitchBend => Some(self.data[0] & CHANNEL_MASK),
            Status::SysExStart |
            Status::MIDITimeCodeQtrFrame |
            Status::SongPositionPointer |
            Status::SongSelect |
            Status::TuneRequest |
            Status::SysExEnd |
            Status::TimingClock |
            Status::Start |
            Status::Continue |
            Status::Stop |
            Status::ActiveSensing |
            Status::SystemReset => None
        }
    }

    /// Get te data at index `index` from this message.  Status is at
    /// index 0
    #[inline(always)]
    pub fn data(&self, index: usize) -> u8 {
        self.data[index]
    }

    /// Create a midi message from a vector of bytes
    #[inline(always)]
    pub fn from_bytes(bytes: Vec<u8>) -> MidiMessage{
        // TODO: Validate bytes
        MidiMessage {
            data: bytes,
        }
    }

    // return the number of data bytes for a message with the given status
    // -1 -> variable sized message, call get_variable_size
    // -2 -> sysex, read until SysExEnd
    // -3 -> invalid status
    pub fn data_bytes(status: u8) -> isize {
        match Status::from_u8(status & STATUS_MASK) {
            Some(stat) => {
                match stat {
                    Status::NoteOff |
                    Status::NoteOn |
                    Status::PolyphonicAftertouch |
                    Status::ControlChange |
                    Status::PitchBend |
                    Status::SongPositionPointer => { 2 }

                    Status::SysExStart => { -2 }

                    Status::ProgramChange |
                    Status::ChannelAftertouch |
                    Status::MIDITimeCodeQtrFrame |
                    Status::SongSelect => { 1 }

                    Status::TuneRequest |
                    Status::SysExEnd |
                    Status::TimingClock |
                    Status::Start |
                    Status::Continue |
                    Status::Stop |
                    Status::ActiveSensing |
                    Status::SystemReset => { 0 }
                }
            }
            None => -3
        }
    }

    /// Get the next midi message from the reader given that the
    /// status `stat` has just been read
    pub fn next_message_given_status(stat: u8, reader: &mut Read) -> Result<MidiMessage, MidiError> {
        let mut ret:Vec<u8> = Vec::with_capacity(3);
        ret.push(stat);
        match MidiMessage::data_bytes(stat) {
            0 => {}
            1 => { ret.push(try!(read_byte(reader))); }
            2 => { ret.push(try!(read_byte(reader)));
                   ret.push(try!(read_byte(reader))); }
            -1 => { return Err(MidiError::OtherErr("Don't handle variable sized yet")); }
            -2 => {
                // skip SysEx message
                while {
                    let byte = try!(read_byte(reader));
                    ret.push(byte);
                    byte != Status::SysExEnd as u8
                } {}
            }
            _ =>  { return Err(MidiError::InvalidStatus(stat)); }
        }
        Ok(MidiMessage{data: ret})
    }

    /// Get the next midi message from the reader given that there's a running
    /// status of `stat` and that in place of a status was read `databyte`
    pub fn next_message_running_status(stat: u8, databyte: u8, reader: &mut Read) -> Result<MidiMessage, MidiError> {
        let mut ret:Vec<u8> = Vec::with_capacity(3);
        ret.push(stat);
        ret.push(databyte);
        match MidiMessage::data_bytes(stat) {
            0 => { panic!("Can't have zero length message with running status"); }
            1 => { } // already read it
            2 => { ret.push(try!(read_byte(reader))); } // only need one more byte
            -1 => { return Err(MidiError::OtherErr("Don't handle variable sized yet")); }
            -2 => { return Err(MidiError::OtherErr("Running status not permitted with meta and sysex event")); }
            _ =>  { return Err(MidiError::InvalidStatus(stat)); }
        }
        Ok(MidiMessage{data: ret})
    }

    /// Extract next midi message from a reader
    pub fn next_message(reader: &mut Read) -> Result<MidiMessage,MidiError> {
        let stat = try!(read_byte(reader));
        MidiMessage::next_message_given_status(stat,reader)
    }


    // Functions to build midi messages

    /// Create a note on message
    pub fn note_on(note: u8, velocity: u8, channel: u8) -> MidiMessage {
        MidiMessage {
            data: vec![make_status(Status::NoteOn,channel), note, velocity],
        }
    }

    /// Create a note off message
    pub fn note_off(note: u8, velocity: u8, channel: u8) -> MidiMessage {
        MidiMessage {
            data: vec![make_status(Status::NoteOff,channel), note, velocity],
        }
    }

    /// Create a polyphonic aftertouch message
    /// This message is most often sent by pressing down on the key after it "bottoms out".
    pub fn polyphonic_aftertouch(note: u8, pressure: u8, channel: u8) -> MidiMessage {
        MidiMessage {
            data: vec![make_status(Status::PolyphonicAftertouch,channel), note, pressure],
        }
    }

    /// Create a control change message
    /// This message is sent when a controller value changes. Controllers include devices such as
    /// pedals and levers. Controller numbers 120-127 are reserved as "Channel Mode Messages".
    pub fn control_change(controler: u8, data: u8, channel: u8) -> MidiMessage {
        MidiMessage {
            data: vec![make_status(Status::ControlChange,channel), controler, data],
        }
    }

    /// Create a program change message
    /// This message sent when the patch number changes. `program` is the new program number.
    pub fn program_change(program: u8, channel: u8) -> MidiMessage {
        MidiMessage {
            data: vec![make_status(Status::ProgramChange,channel), program],
        }
    }

    /// Create a channel aftertouch
    /// This message is most often sent by pressing down on the key after it "bottoms out". This message
    /// is different from polyphonic after-touch. Use this message to send the single greatest pressure
    /// value (of all the current depressed keys). `pressure` is the pressure value.
    pub fn channel_aftertouch(pressure: u8, channel: u8) -> MidiMessage {
        MidiMessage {
            data: vec![make_status(Status::ChannelAftertouch,channel), pressure],
        }
    }

    /// Create a pitch bench message
    /// This message is sent to indicate a change in the pitch bender (wheel or lever, typically).
    /// The pitch bender is measured by a fourteen bit value. Center (no pitch change) is 2000H.
    /// Sensitivity is a function of the transmitter. `lsb` are the least significant 7 bits.
    /// `msb` are the most significant 7 bits.
    pub fn pitch_bend(lsb: u8, msb: u8, channel: u8) -> MidiMessage {
        MidiMessage {
            data: vec![make_status(Status::PitchBend,channel), lsb, msb],
        }
    }

}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}",
               match *self {
                   Status::NoteOff => "Note Off",
                   Status::NoteOn => "Note On",
                   Status::PolyphonicAftertouch => "Polyphonic Aftertouch",
                   Status::ControlChange => "Control Change",
                   Status::ProgramChange => "Program Change",
                   Status::ChannelAftertouch => "Channel Aftertouch",
                   Status::PitchBend => "Pitch Bend",
                   Status::SysExStart => "SysEx Start",
                   Status::MIDITimeCodeQtrFrame => "MIDI Time Code Qtr Frame",
                   Status::SongPositionPointer => "Song Position Pointer",
                   Status::SongSelect => "Song Select",
                   Status::TuneRequest => "Tune Request",
                   Status::SysExEnd => "SysEx End",
                   Status::TimingClock => "Timing Clock",
                   Status::Start => "Start",
                   Status::Continue => "Continue",
                   Status::Stop => "Stop",
                   Status::ActiveSensing => "Active Sensing",
                   Status::SystemReset => "System Reset",
               })
    }
}

impl fmt::Display for MidiMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.data.len() == 2 {
            write!(f, "{}: [{}]\tchannel: {:?}", self.status(), self.data[1], self.channel())
        }
        else if self.data.len() == 3 {
            write!(f, "{}: [{},{}]\tchannel: {:?}", self.status(), self.data[1], self.data[2], self.channel())
        }
        else if self.data.len() == 0 {
            write!(f, "{}: [no data]\tchannel: {:?}", self.status(), self.channel())
        }
        else {
            write!(f, "{}: {:?}\tchannel: {:?}", self.status(), self.data, self.channel())
        }
    }
}
