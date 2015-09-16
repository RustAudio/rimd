//! rimd is a set of utilities to deal with midi messages and standard
//! midi files (SMF).  It handles both standard midi messages and the meta
//! messages that are found in SMFs.
//!
//! rimd is fairly low level, and  messages are stored and accessed in
//! their underlying format (i.e. a vector of u8s).  There are some
//! utility methods for accessing the various pieces of a message, and
//! for constructing new messages.
//!
//! For a description of the underlying format of midi messages see:<br/>
//! http://www.midi.org/techspecs/midimessages.php<br/>
//! For a description of the underlying format of meta messages see:<br/>
//! http://cs.fit.edu/~ryan/cse4051/projects/midi/midi.html#meta_event

extern crate byteorder;
#[macro_use] extern crate enum_primitive;
extern crate num;

use std::error;
use std::convert::From;
use std::fs::File;
use std::io::{Error,Read};
use std::path::Path;

use std::fmt;
use std::string::FromUtf8Error;

pub use midi:: {
    Status,
    MidiError,
    MidiMessage,
};

pub use meta:: {
    MetaCommand,
    MetaError,
    MetaEvent,
};

pub use builder:: {
    SMFBuilder,
    AbsoluteEvent,
};

use reader:: {
    SMFReader,
};

pub use writer:: {
    SMFWriter,
};

pub use util:: {
    note_num_to_name,
};

mod builder;
mod midi;
mod meta;
mod reader;
mod writer;
mod util;

/// Format of the SMF
#[derive(Debug,Clone,Copy)]
pub enum SMFFormat {
    /// single track file format
    Single = 0,
    /// multiple track file format
    MultiTrack = 1,
    /// multiple song file format (i.e., a series of single type files)
    MultiSong = 2,
}


impl fmt::Display for SMFFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}",match *self {
            SMFFormat::Single     => "single track",
            SMFFormat::MultiTrack => "multiple track",
            SMFFormat::MultiSong  => "multiple song",
        })
    }
}

/// An event can be either a midi message or a meta event
#[derive(Debug,Clone)]
pub enum Event {
    Midi(MidiMessage),
    Meta(MetaEvent),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Event::Midi(ref m) => { write!(f, "{}", m) }
            Event::Meta(ref m) => { write!(f, "{}", m) }
        }
    }
}

impl Event {
    /// Return the number of bytes this event uses.
    pub fn len(&self) -> usize {
        match *self {
            Event::Midi(ref m) => { m.data.len() }
            Event::Meta(ref m) => {
                let v = SMFWriter::vtime_to_vec(m.length);
                // +1 for command byte +1 for 0xFF to indicate Meta event
                v.len() + m.data.len() + 2
            }
        }
    }
}

/// An event occuring in the track.
#[derive(Debug)]
pub struct TrackEvent {
    /// A delta offset, indicating how many ticks after the previous
    /// event this event occurs
    pub vtime: u64,
    /// The actual event
    pub event: Event,
}


impl fmt::Display for TrackEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "vtime: {}\t{}",self.vtime,self.event)
    }
}

impl TrackEvent {
    pub fn fmt_with_time_offset(&self, cur_time: u64) -> String {
        format!("time: {}\t{}",(self.vtime+cur_time),self.event)
    }

    /// Return the number of bytes this event uses in the track,
    /// including the space for the time offset.
    pub fn len(&self) -> usize {
        let v = SMFWriter::vtime_to_vec(self.vtime);
        v.len() + self.event.len()
    }
}

/// A sequence of midi/meta events
#[derive(Debug)]
pub struct Track {
    /// Optional copyright notice
    pub copyright: Option<String>,
    /// Optional name for this track
    pub name: Option<String>,
    /// Vector of the events in this track
    pub events: Vec<TrackEvent>
}

impl fmt::Display for Track {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Track, copyright: {}, name: {}",
               match self.copyright {
                   Some(ref c) => &c[..],
                   None => "[none]"
               },
               match self.name {
                   Some(ref n) => &n[..],
                   None => "[none]"
               })
    }
}


/// An error that occured in parsing an SMF
#[derive(Debug)]
pub enum SMFError {
    InvalidSMFFile(&'static str),
    MidiError(MidiError),
    MetaError(MetaError),
    Error(Error),
}

impl SMFError {
    fn is_eof(&self) -> bool {
        match *self {
            // SMFError::Error(ref err) => {
            //     err.kind() == ErrorKind::EndOfFile
            // }
            _ => false
        }
    }
}

impl From<Error> for SMFError {
    fn from(err: Error) -> SMFError {
        SMFError::Error(err)
    }
}

impl From<MidiError> for SMFError {
    fn from(err: MidiError) -> SMFError {
        SMFError::MidiError(err)
    }
}

impl From<MetaError> for SMFError {
    fn from(err: MetaError) -> SMFError {
        SMFError::MetaError(err)
    }
}

impl From<FromUtf8Error> for SMFError {
    fn from(_: FromUtf8Error) -> SMFError {
        SMFError::InvalidSMFFile("Invalid UTF8 data in file")
    }
}

impl error::Error for SMFError {
    fn description(&self) -> &str {
        match *self {
            SMFError::InvalidSMFFile(_) => "The SMF file was invalid",
            SMFError::Error(ref e)        => e.description(),
            SMFError::MidiError(ref m)      => m.description(),
            SMFError::MetaError(ref m)      => m.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            SMFError::MidiError(ref m) => Some(m as &error::Error),
            SMFError::MetaError(ref m) => Some(m as &error::Error),
            SMFError::Error(ref err) => Some(err as &error::Error),
            _ => None,
        }
    }
}

impl fmt::Display for SMFError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match *self {
           SMFError::InvalidSMFFile(s) => write!(f,"SMF file is invalid: {}",s),
           SMFError::MidiError(ref err) => { write!(f,"{}",err) },
           SMFError::MetaError(ref err) => { write!(f,"{}",err) },
           SMFError::Error(ref err) => { write!(f,"{}",err) },
       }
    }
}

/// A standard midi file
#[derive(Debug)]
pub struct SMF {
    /// The format of the SMF
    pub format: SMFFormat,
    /// Vector holding each track in this SMF
    pub tracks: Vec<Track>,
    /// The unit of time for delta timing. If the value is positive,
    /// then it represents the units per beat. For example, +96 would
    /// mean 96 ticks per beat. If the value is negative, delta times
    /// are in SMPTE compatible units.
    pub division: i16,
}


impl SMF {
    /// Read an SMF file at the given path
    pub fn from_file(path: &Path) -> Result<SMF,SMFError> {
        let mut file = try!(File::open(path));
        SMFReader::read_smf(&mut file)
    }

    /// Read an SMF from the given reader
    pub fn from_reader(reader: &mut Read) -> Result<SMF,SMFError> {
        SMFReader::read_smf(reader)
    }
}

