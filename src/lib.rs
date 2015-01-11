use std::error;
use std::io::{File,IoError,IoErrorKind,Reader};
use std::io::util::{IterReader};
use std::fmt;

pub use midi:: {
    MidiError,
    MidiMessage,
};

pub use meta:: {
    MetaError,
    MetaEvent,
};

mod midi;
mod meta;

pub enum SMFFormat {
    Single,
    MultiTrack,
    MultiSong,
}

impl Copy for SMFFormat {}

impl fmt::Show for SMFFormat {
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

impl fmt::Show for Event {
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

impl fmt::Show for TrackEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "time: {}\t{}",self.vtime,self.event)
    }
}

pub struct SMF {
    pub format: SMFFormat,
    pub tracks: Vec<Vec<TrackEvent>>,
    pub division: u16,
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

impl error::Error for SMFError {
    fn description(&self) -> &str {
        match *self {
            SMFError::InvalidSMFFile(_) => "The SMF file was invalid",
            SMFError::IoError(_)        => "An I/O error occured",
            SMFError::MidiError(_)      => "Invalid Midi Data",
            SMFError::MetaError(_)      => "Invalid Meta Data",
        }
    }

    fn detail(&self) -> Option<String> {
        match *self {
            SMFError::InvalidSMFFile(s) => Some(format!("SMF file is invalid: {}",s)),
            SMFError::IoError(ref err)  => err.detail(),
            SMFError::MidiError(_) => Some(format!("Invalid Midi Data detail")),
            SMFError::MetaError(_) => Some(format!("Invalid Meta Data detail")),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            SMFError::IoError(ref err) => Some(err as &error::Error),
            _ => None,
        }
    }
}

impl SMF {

    fn read_vtime(reader: &mut Reader) -> Result<u64,SMFError> {
        let mut res: u64 = 0;
        let mut i = 0;
        let cont_mask = 0x80;
        let val_mask = 0x7F;
        loop {
            i+=1;
            if i > 9 {
                return Err(SMFError::InvalidSMFFile("Variable length value too long"));
            }
            let next = try!(reader.read_byte());
            res |= next as u64 & val_mask;
            if (next & cont_mask) == 0 {
                break;
            }
            res = res << 7;
        }
        Ok(res)
    }

    fn parse_header(reader: &mut Reader) -> Result<SMF,SMFError> {
        let mut header:[u8;14] = [0;14];
        try!(reader.read_at_least(14,&mut header));

        if header[0] != 0x4D ||
           header[1] != 0x54 ||
           header[2] != 0x68 ||
           header[3] != 0x64 {
               return Err(SMFError::InvalidSMFFile("Invalid header magic"));
           }
        let format = match header[9] {
            0 => SMFFormat::Single,
            1 => SMFFormat::MultiTrack,
            2 => SMFFormat::MultiSong,
            _ => return Err(SMFError::InvalidSMFFile("Invalid format bytes")),
        };

        let tracks = (header[10] as u16) << 8 | header[11] as u16;
        let division = (header[12] as u16) << 8 | header[13] as u16;

        Ok(SMF { format: format,
                 tracks: Vec::with_capacity(tracks as uint),
                 division: division } )
    }

    fn next_event(reader: &mut Reader, cur_time: &mut u64) -> Result<TrackEvent,SMFError> {
        let time = try!(SMF::read_vtime(reader));
        *cur_time += time;
        let stat = try!(reader.read_byte());
        match stat {
            0xFF => {
                let event = try!(MetaEvent::next_event(reader));
                Ok( TrackEvent {
                    vtime: *cur_time,
                    event: Event::Meta(event),
                })
            }
            _ => {
                let msg = try!(MidiMessage::next_message_given_status(stat,reader));
                Ok( TrackEvent {
                    vtime: *cur_time,
                    event: Event::Midi(msg),
                })
            }
        }
    }

    fn parse_track(reader: &mut Reader) -> Result<Vec<TrackEvent>,SMFError> {
        let mut res:Vec<TrackEvent> = Vec::new();
        let mut buf:[u8;4] = [0;4];
        try!(reader.read_at_least(4,&mut buf));
        if buf[0] != 0x4D ||
           buf[1] != 0x54 ||
           buf[2] != 0x72 ||
           buf[3] != 0x6B {
               return Err(SMFError::InvalidSMFFile("Invalid track magic"));
           }
        try!(reader.read_at_least(4,&mut buf));
        let len =
            ((buf[0] as u32) << 24 |
            (buf[1] as u32) << 16 |
            (buf[2] as u32) << 8 |
            (buf[3] as u32)) as uint;
        let mut data = IterReader::new(try!(reader.read_exact(len)).into_iter());
        let mut time: u64 = 0;
        loop {
            match SMF::next_event(&mut data,&mut time) {
                Ok(event) => res.push(event),
                Err(err) => {
                    if err.is_eof() { break; }
                    else { return Err(err); }
                }
            }
        }
        Ok(res)
    }

    pub fn from_file(path: &Path) -> Result<SMF,SMFError> {
        let mut file = try!(File::open(path));
        let mut smf = SMF::parse_header(&mut file);
        match smf {
            Ok(ref mut s) => {
                for _ in range(0,s.tracks.capacity()) {
                    s.tracks.push(try!(SMF::parse_track(&mut file)));
                }
            }
            _ => {}
        }
        smf
    }
}


#[test]
fn it_works() {
}
