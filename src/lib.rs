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

use reader:: {
    SMFReader,
};

mod midi;
mod meta;
mod reader;

pub enum SMFFormat {
    Single,
    MultiTrack,
    MultiSong,
}

impl Copy for SMFFormat {}

impl fmt::String for SMFFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}",match *self {
            SMFFormat::Single     => "single track",
            SMFFormat::MultiTrack => "multiple track",
            SMFFormat::MultiSong  => "multiple song",
        })
    }
}


pub enum Event {
    Midi(MidiMessage),
    Meta(MetaEvent),
}

impl fmt::String for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Event::Midi(ref m) => { write!(f, "{}", m) }
            Event::Meta(ref m) => { write!(f, "{}", m) }
        }
    }
}

pub struct TrackEvent {
    pub vtime: u64,
    pub event: Event,
}

impl fmt::String for TrackEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "time: {}\t{}",self.vtime,self.event)
    }
}

pub struct Track {
    pub copyright: Option<String>,
    pub name: Option<String>,
    pub events: Vec<TrackEvent>
}

impl fmt::String for Track {
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
    fn from_error(err: FromUtf8Error) -> SMFError {
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

    fn detail(&self) -> Option<String> {
        match *self {
            SMFError::InvalidSMFFile(s) => Some(format!("SMF file is invalid: {}",s)),
            SMFError::IoError(ref err)  => err.detail(),
            SMFError::MidiError(ref m) => m.detail(),
            SMFError::MetaError(ref m) => m.detail(),
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

pub struct SMF {
    pub format: SMFFormat,
    pub tracks: Vec<Track>,
    pub division: u16,
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


#[test]
fn it_works() {
}
