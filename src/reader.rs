use std::io::Read;

use SMF;
use ::{Event,SMFError,SMFFormat,MetaCommand,MetaEvent,MidiMessage,Track,TrackEvent};

use util::{fill_buf, read_byte, latin1_decode};

/// An SMFReader can parse a byte stream into an SMF
#[derive(Clone,Copy)]
pub struct SMFReader;

impl SMFReader {
    fn parse_header(reader: &mut Read) -> Result<SMF,SMFError> {
        let mut header:[u8;14] = [0;14];
        try!(fill_buf(reader,&mut header));

        // skip RIFF header if present
        if header[0] == 0x52 &&
           header[1] == 0x49 &&
           header[2] == 0x46 &&
           header[3] == 0x46 {
            let mut skip:[u8; 6] = [0; 6];
            try!(fill_buf(reader, &mut skip));
            try!(fill_buf(reader, &mut header));
        }

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

    fn next_event(reader: &mut Read, laststat: u8, was_running: &mut bool) -> Result<TrackEvent,SMFError> {
        let time = try!(SMFReader::read_vtime(reader));
        let stat = try!(read_byte(reader));

        if (stat & 0x80) == 0 {
            *was_running = true;
        } else {
            *was_running = false;
        }

        match stat {
            0xFF => {
                let event = try!(MetaEvent::next_event(reader));
                Ok( TrackEvent {
                    vtime: time,
                    event: Event::Meta(event),
                })
            }
            _ => {
                let msg =
                    if (stat & 0x80) == 0 {
                        // this is a running status, so assume we have the same status as last time
                        try!(MidiMessage::next_message_running_status(laststat,stat,reader))
                    } else {
                        try!(MidiMessage::next_message_given_status(stat,reader))
                    };
                Ok( TrackEvent {
                    vtime: time,
                    event: Event::Midi(msg),
                })
            }
        }
    }

    fn parse_track(reader: &mut Read) -> Result<Track,SMFError> {
        let mut res:Vec<TrackEvent> = Vec::new();
        let mut buf:[u8;4] = [0;4];

        let mut copyright = None;
        let mut name = None;

        try!(fill_buf(reader,&mut buf));
        if buf[0] != 0x4D || // "MTrk"
           buf[1] != 0x54 ||
           buf[2] != 0x72 ||
           buf[3] != 0x6B {
               return Err(SMFError::InvalidSMFFile("Invalid track magic"));
           }
        try!(fill_buf(reader,&mut buf));
        let len =
            ((buf[0] as u32) << 24 |
             (buf[1] as u32) << 16 |
             (buf[2] as u32) << 8 |
             (buf[3] as u32)) as usize;

        let mut read_so_far = 0;

        loop {
            let last = { // use status from last midi event, skip meta events
                let mut last = 0u8;
                for e in res.iter().rev() {
                    match e.event {
                        Event::Midi(ref m) => { last = m.data[0]; break; }
                        _ => ()
                    }
                }
                last
            };
            let mut was_running = false;
            match SMFReader::next_event(reader,last,&mut was_running) {
                Ok(event) => {
                    match event.event {
                        Event::Meta(ref me) => {
                            match me.command {
                                MetaCommand::CopyrightNotice => copyright = Some(latin1_decode(&me.data)),
                                MetaCommand::SequenceOrTrackName => name = Some(latin1_decode(&me.data)),
                                _ => {}
                            }
                        },
                        _ => {}
                    }
                    read_so_far += event.len();
                    if was_running {
                        // used a running status, so didn't actually read a status byte
                        read_so_far -= 1;
                    }
                    res.push(event);
                    if read_so_far == len {
                        break;
                    }
                    if read_so_far > len {
                        return Err(SMFError::InvalidSMFFile("Invalid MIDI file"));
                    }
                },
                Err(err) => {
                    /* // uncomment for debugging to print the last parsed events
                    for e in &res[res.len()-10..] {
                        match e.event {
                            Event::Midi(MidiMessage {ref data}) | Event::Meta(MetaEvent {ref data, ..}) => {
                                for b in data {
                                    print!("{:02X}", b);
                                }
                            }
                        }
                        println!(": {:?} {}", e, e);
                    }*/
                    return Err(err);
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
    pub fn read_vtime(reader: &mut Read) -> Result<u64,SMFError> {
        let mut res: u64 = 0;
        let mut i = 0;
        let cont_mask = 0x80;
        let val_mask = 0x7F;
        loop {
            i+=1;
            if i > 9 {
                return Err(SMFError::InvalidSMFFile("Variable length value too long"));
            }
            let next = try!(read_byte(reader));
            res |= next as u64 & val_mask;
            if (next & cont_mask) == 0 {
                break;
            }
            res = res << 7;
        }
        Ok(res)
    }

    /// Read an entire SMF file
    pub fn read_smf(reader: &mut Read) -> Result<SMF,SMFError> {
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
