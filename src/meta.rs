use std::error;
use std::io::{Error, Read};
use std::fmt;

use reader::SMFReader;

use num_traits::FromPrimitive;

use util::{read_byte, read_amount, latin1_decode};

/// An error that can occur parsing a meta command
#[derive(Debug)]
pub enum MetaError {
    InvalidCommand(u8),
    OtherErr(&'static str),
    Error(Error),
}

impl From<Error> for MetaError {
    fn from(err: Error) -> MetaError {
        MetaError::Error(err)
    }
}

impl error::Error for MetaError {
    fn description(&self) -> &str {
        match *self {
            MetaError::InvalidCommand(_) => "Invalid meta command",
            MetaError::OtherErr(_) => "A general midi error has occured",
            MetaError::Error(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            MetaError::Error(ref err) => Some(err as &error::Error),
            _ => None,
        }
    }
}

impl fmt::Display for MetaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MetaError::InvalidCommand(ref c) => write!(f,"Invalid Meta command: {}",c),
            MetaError::OtherErr(ref s) => write!(f,"Meta Error: {}",s),
            MetaError::Error(ref e) => write!(f,"{}",e),
        }
    }
}

/// Commands that meta messages can represent
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd,Ord,  FromPrimitive)]
pub enum MetaCommand {
    SequenceNumber = 0x00,
    TextEvent = 0x01,
    CopyrightNotice = 0x02,
    SequenceOrTrackName = 0x03,
    InstrumentName = 0x04,
    LyricText = 0x05,
    MarkerText = 0x06,
    CuePoint = 0x07,
    MIDIChannelPrefixAssignment = 0x20,
    MIDIPortPrefixAssignment = 0x21,
    EndOfTrack = 0x2F,
    TempoSetting = 0x51,
    SMPTEOffset = 0x54,
    TimeSignature = 0x58,
    KeySignature = 0x59,
    SequencerSpecificEvent = 0x7F,
    Unknown,
}

/// Meta event building and parsing.  See
/// http://cs.fit.edu/~ryan/cse4051/projects/midi/midi.html#meta_event
/// for a description of the various meta events and their formats
#[derive(Debug)]
pub struct MetaEvent {
    pub command: MetaCommand,
    pub length: u64,
    pub data: Vec<u8>,
}

impl Clone for MetaEvent {
    fn clone(&self) -> MetaEvent {
        MetaEvent {
            command: self.command,
            length: self.length,
            data: self.data.clone(),
        }
    }
}

impl fmt::Display for MetaEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Meta Event: {}",
               match self.command {
                   MetaCommand::SequenceNumber => format!("Sequence Number: {}", ((self.data[0] as u16) << 8) | self.data[1] as u16),
                   MetaCommand::TextEvent => {
                       format!("Text Event. Len: {} Text: {}", self.length, latin1_decode(&self.data))
                   },
                   MetaCommand::CopyrightNotice => {
                       format!("Copyright Notice: {}", latin1_decode(&self.data))
                   },
                   MetaCommand::SequenceOrTrackName => {
                       format!("Sequence/Track Name, length: {}, name: {}", self.length, latin1_decode(&self.data))
                   },
                   MetaCommand::InstrumentName => {
                       format!("InstrumentName: {}", latin1_decode(&self.data))
                   },
                   MetaCommand::LyricText => {
                       format!("LyricText: {}", latin1_decode(&self.data))
                   }
                   MetaCommand::MarkerText => {
                       format!("MarkerText: {}", latin1_decode(&self.data))
                   }
                   MetaCommand::CuePoint => format!("CuePoint: {}", latin1_decode(&self.data)),
                   MetaCommand::MIDIChannelPrefixAssignment => format!("MIDI Channel Prefix Assignment, channel: {}", self.data[0]+1),
                   MetaCommand::MIDIPortPrefixAssignment => format!("MIDI Port Prefix Assignment, port: {}", self.data[0]),
                   MetaCommand::EndOfTrack => format!("End Of Track"),
                   MetaCommand::TempoSetting => format!("Set Tempo, microseconds/quarter note: {}", self.data_as_u64(3)),
                   MetaCommand::SMPTEOffset => format!("SMPTEOffset"),
                   MetaCommand::TimeSignature => format!("Time Signature: {}/{}, {} ticks/metronome click, {} 32nd notes/quarter note",
                                                         self.data[0],
                                                         2usize.pow(self.data[1] as u32),
                                                         self.data[2],
                                                         self.data[3]),
                   MetaCommand::KeySignature => format!("Key Signature, {} sharps/flats, {}",
                                                        self.data[0] as i8,
                                                        match self.data[1] {
                                                            0 => "Major",
                                                            1 => "Minor",
                                                            _ => "Invalid Signature",
                                                        }),
                   MetaCommand::SequencerSpecificEvent => format!("SequencerSpecificEvent"),
                   MetaCommand::Unknown => format!("Unknown, length: {}", self.data.len()),
               })
    }
}

impl MetaEvent {

    /// Turn `bytes` bytes of the data of this event into a u64
    pub fn data_as_u64(&self, bytes: usize) -> u64 {
        let mut res = 0;
        for i in 0..bytes {
            res <<= 8;
            res |= self.data[i] as u64;
        }
        res
    }

    /// Extract the next meta event from a reader
    pub fn next_event(reader: &mut Read) -> Result<MetaEvent, MetaError> {
        let command =
            match MetaCommand::from_u8(try!(read_byte(reader))) {
                Some(c) => {c},
                None => MetaCommand::Unknown,
            };
        let len = match SMFReader::read_vtime(reader) {
            Ok(t) => { t }
            Err(_) => { return Err(MetaError::OtherErr("Couldn't read time for meta command")); }
        };
        let mut data = Vec::new();
        try!(read_amount(reader,&mut data,len as usize));
        Ok(MetaEvent{
            command: command,
            length: len,
            data: data
        })
    }


    // util functions for event constructors
    fn u16_to_vec(val: u16) -> Vec<u8> {
        let mut res = Vec::with_capacity(2);
        res.push((val >> 8) as u8);
        res.push(val as u8);
        res
    }

    fn u24_to_vec(val: u32) -> Vec<u8> {
        assert!(val <= 2u32.pow(24));
        let mut res = Vec::with_capacity(3);
        res.push((val >> 16) as u8);
        res.push((val >> 8) as u8);
        res.push(val as u8);
        res
    }

    // event constructors below

    /// Create a sequence number meta event
    pub fn sequence_number(sequence_number: u16) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::SequenceNumber,
            length: 0x02,
            data: MetaEvent::u16_to_vec(sequence_number),
        }
    }

    /// Create a text meta event
    pub fn text_event(text: String) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::TextEvent,
            length: text.len() as u64,
            data: text.into_bytes(),
        }
    }

    /// Create a copyright notice meta event
    pub fn copyright_notice(copyright: String) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::CopyrightNotice,
            length: copyright.len() as u64,
            data: copyright.into_bytes(),
        }
    }

    /// Create a name meta event
    pub fn sequence_or_track_name(name: String) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::SequenceOrTrackName,
            length: name.len() as u64,
            data: name.into_bytes(),
        }
    }

    /// Create an instrument name meta event
    pub fn instrument_name(name: String) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::InstrumentName,
            length: name.len() as u64,
            data: name.into_bytes(),
        }
    }

    /// Create a lyric text meta event
    pub fn lyric_text(text: String) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::LyricText,
            length: text.len() as u64,
            data: text.into_bytes(),
        }
    }


    /// Create a marker text meta event
    pub fn marker_text(text: String) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::MarkerText,
            length: text.len() as u64,
            data: text.into_bytes(),
        }
    }

    /// Create a cue point meta event
    pub fn cue_point(text: String) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::CuePoint,
            length: text.len() as u64,
            data: text.into_bytes(),
        }
    }

    /// Create a midi channel prefix assignment meta event
    pub fn midichannel_prefix_assignment(channel: u8) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::MIDIChannelPrefixAssignment,
            length: 1,
            data: vec![channel],
        }
    }

    /// Create a midi port prefix assignment meta event
    pub fn midiport_prefix_assignment(port: u8) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::MIDIPortPrefixAssignment,
            length: 1,
            data: vec![port],
        }
    }

    /// Create an end of track meta event
    pub fn end_of_track() -> MetaEvent {
        MetaEvent {
            command: MetaCommand::EndOfTrack,
            length: 0,
            data: vec![],
        }
    }

    /// Create an event to set track tempo.  This is stored
    /// as a 24-bit value.  This method will fail an assertion if
    /// the supplied tempo is greater than 2^24.
    pub fn tempo_setting(tempo: u32) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::TempoSetting,
            length: 3,
            data: MetaEvent::u24_to_vec(tempo),
        }
    }

    /// Create an smpte offset meta event
    pub fn smpte_offset(hours: u8, minutes: u8, seconds: u8, frames: u8, fractional: u8) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::SMPTEOffset,
            length: 5,
            data: vec![hours,minutes,seconds,frames,fractional],
        }
    }

    /// Create a time signature event.
    /// Time signature of the form:
    /// `numerator`/2^`denominator`
    ///  eg: 6/8 would be specified using `numerator`=6, `denominator`=3
    ///
    /// The parameter `clocks_per_tick` is the number of MIDI Clocks per metronome tick.

    /// Normally, there are 24 MIDI Clocks per quarter note.
    /// However, some software allows this to be set by the user.
    /// The parameter `num_32nd_notes_per_24_clocks` defines this in terms of the
    /// number of 1/32 notes which make up the usual 24 MIDI Clocks
    /// (the 'standard' quarter note).  8 is standard
    pub fn time_signature(numerator: u8, denominator: u8, clocks_per_tick: u8, num_32nd_notes_per_24_clocks: u8) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::TimeSignature,
            length: 4,
            data: vec![numerator,denominator,clocks_per_tick,num_32nd_notes_per_24_clocks],
        }
    }

    ///  Create a Key Signature event
    ///  expressed as the number of sharps or flats, and a major/minor flag.

    /// `sharps_flats` of 0 represents a key of C, negative numbers represent
    /// 'flats', while positive numbers represent 'sharps'.
    pub fn key_signature(sharps_flats: u8, major_minor: u8) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::KeySignature,
            length: 2,
            data: vec![sharps_flats, major_minor],
        }
    }

    /// This is the MIDI-file equivalent of the System Exclusive Message.
    /// sequencer-specific directives can be incorporated into a
    /// MIDI file using this event.
    pub fn sequencer_specific_event(data: Vec<u8>) -> MetaEvent {
        MetaEvent {
            command: MetaCommand::SequencerSpecificEvent,
            length: data.len() as u64,
            data: data,
        }
    }

}
