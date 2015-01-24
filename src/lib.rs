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


#![allow(unstable)]

use std::error;
use std::io::{File,IoError,IoErrorKind,Reader};

use std::fmt;
use std::string::FromUtf8Error;

pub use midi:: {
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
};

use reader:: {
    SMFReader,
};

mod builder;
mod midi;
mod meta;
mod reader;

/// Format of the SMF
pub enum SMFFormat {
    /// single track file format
    Single,
    /// multiple track file format
    MultiTrack,
    /// multiple song file format (i.e., a series of single type files)
    MultiSong,
}

impl Copy for SMFFormat {}

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

/// An event occuring in the track.
pub struct TrackEvent {
    /// A delta offset, indicating how many ticks after the previous
    /// event this event occurs
    pub vtime: u64,
    /// The actual event
    pub event: Event,
}


impl fmt::Display for TrackEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "time: {}\t{}",self.vtime,self.event)
    }
}

/// A sequence of midi/meta events
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
                   Some(ref c) => c.as_slice(),
                   None => "[none]"
               },
               match self.name {
                   Some(ref n) => n.as_slice(),
                   None => "[none]"
               })
    }
}


/// An error that occured in parsing an SMF
pub enum SMFError {
    InvalidSMFFile(&'static str),
    MidiError(MidiError),
    MetaError(MetaError),
    IoError(IoError),
}

impl SMFError {
    fn is_eof(&self) -> bool {
        match *self {
            SMFError::IoError(ref err) => {
                err.kind == IoErrorKind::EndOfFile
            }
            _ => false
        }
    }
}

impl error::FromError<IoError> for SMFError {
    fn from_error(err: IoError) -> SMFError {
        SMFError::IoError(err)
    }
}

impl error::FromError<MidiError> for SMFError {
    fn from_error(err: MidiError) -> SMFError {
        SMFError::MidiError(err)
    }
}

impl error::FromError<MetaError> for SMFError {
    fn from_error(err: MetaError) -> SMFError {
        SMFError::MetaError(err)
    }
}

impl error::FromError<FromUtf8Error> for SMFError {
    fn from_error(_: FromUtf8Error) -> SMFError {
        SMFError::InvalidSMFFile("Invalid UTF8 data in file")
    }
}

impl error::Error for SMFError {
    fn description(&self) -> &str {
        match *self {
            SMFError::InvalidSMFFile(_) => "The SMF file was invalid",
            SMFError::IoError(ref e)        => e.description(),
            SMFError::MidiError(ref m)      => m.description(),
            SMFError::MetaError(ref m)      => m.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            SMFError::MidiError(ref m) => Some(m as &error::Error),
            SMFError::MetaError(ref m) => Some(m as &error::Error),
            SMFError::IoError(ref err) => Some(err as &error::Error),
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
           SMFError::IoError(ref err) => { write!(f,"{}",err) },
       }
    }
}

/// A standard midi file
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
    pub fn from_reader(reader: &mut Reader) -> Result<SMF,SMFError> {
        SMFReader::read_smf(reader)
    }
}

