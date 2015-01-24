use std::io::Reader;
use std::io::util::IterReader;

use SMF;
use ::{Event,SMFError,SMFFormat,MetaCommand,MetaEvent,MidiMessage,Track,TrackEvent};

/// An SMFReader can parse a byte stream into an SMF
pub struct SMFReader;
impl Copy for SMFReader {}

impl SMFReader {
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
        let division = (header[12] as i16) << 8 | header[13] as i16;

        Ok(SMF { format: format,
                 tracks: Vec::with_capacity(tracks as usize),
                 division: division } )
    }

    fn next_event(reader: &mut Reader, cur_time: &mut u64) -> Result<TrackEvent,SMFError> {
        let time = try!(SMFReader::read_vtime(reader));
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

    fn parse_track(reader: &mut Reader) -> Result<Track,SMFError> {
        let mut res:Vec<TrackEvent> = Vec::new();
        let mut buf:[u8;4] = [0;4];

        let mut copyright = None;
        let mut name = None;

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
            (buf[3] as u32)) as usize;
        let mut data = IterReader::new(try!(reader.read_exact(len)).into_iter());
        let mut time: u64 = 0;
        loop {
            match SMFReader::next_event(&mut data,&mut time) {
                Ok(event) => {
                    match event.event {
                        Event::Meta(ref me) => {
                            match me.command {
                                MetaCommand::CopyrightNotice => copyright = Some(try!(me.data_as_text())),
                                MetaCommand::SequenceOrTrackName => name = Some(try!(me.data_as_text())),
                                _ => {}
                            }
                        },
                        _ => {}
                    }
                    res.push(event)
                },
                Err(err) => {
                    if err.is_eof() { break; }
                    else { return Err(err); }
                }
            }
        }
        Ok(Track {
            copyright: copyright,
            name: name,
            events: res
        })
    }

    /// Read a variable sized value from the reader.
    /// This is usually used for the times of midi events but is used elsewhere as well.
    pub fn read_vtime(reader: &mut Reader) -> Result<u64,SMFError> {
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

    /// Read an entire SMF file
    pub fn read_smf(reader: &mut Reader) -> Result<SMF,SMFError> {
        let mut smf = SMFReader::parse_header(reader);
        match smf {
            Ok(ref mut s) => {
                for _ in 0..s.tracks.capacity() {
                    s.tracks.push(try!(SMFReader::parse_track(reader)));
                }
            }
            _ => {}
        }
        smf
    }
}
