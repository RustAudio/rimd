//! Some useful utility functions

use std::iter;
use std::io::{Read,Error,ErrorKind};

static NSTRS: &'static str = "C C#D D#E F F#G G#A A#B ";

/// convert a midi note number to a name
pub fn note_num_to_name(num: u32) -> String {
    let oct = (num as f32 /12 as f32).floor()-1.0;
    let nmt = ((num%12)*2) as usize;
    let slice =
        if NSTRS.as_bytes()[nmt+1] == ' ' as u8{
            &NSTRS[nmt..(nmt+1)]
        } else {
            &NSTRS[nmt..(nmt+2)]
        };
    format!("{}{}",slice,oct)
}

/// Read a single byte from a Reader
pub fn read_byte(reader: &mut Read) -> Result<u8,Error> {
    let mut b = [0; 1];
    try!(reader.read(&mut b));
    Ok(b[0])
}

/// Read from reader until buffer is full, or an error occurs
pub fn fill_buf(reader: &mut Read, buf: &mut [u8]) -> Result<(),Error> {
    let mut read = 0;
    while read < buf.len() {
        let bytes_read = try!(reader.read(&mut buf[read..]));
        if bytes_read == 0 {
            return Err(Error::new(ErrorKind::InvalidData, "file ends before it should"));
        }
        read += bytes_read;
    }
    Ok(())
}

/// Read amt from reader and put result in dest.  Errors in underlying
/// reader will cause this function to return an error
pub fn read_amount(reader: &mut Read, dest: &mut Vec<u8>, amt: usize) -> Result<(),Error> {
    let start_len = dest.len();
    let mut len = start_len;
    if dest.capacity() < start_len + amt {
        dest.extend(iter::repeat(0).take(amt));
    }
    let mut ret = Ok(());
    while (len-start_len) < amt {
        match reader.read(&mut dest[len..]) {
            Ok(0) => {
                // read 0 before amount
                ret = Err(Error::new(ErrorKind::InvalidData,
                                     "Stream ended before specified number of bytes could be read"));
            },
            Ok(n) => len += n,
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => {
                ret = Err(e);
                break;
            }
        }
    }
    dest.truncate(len);
    ret
}

pub fn latin1_decode(s: &[u8]) -> String {
    use encoding::{Encoding, DecoderTrap};
    use encoding::all::ISO_8859_1;
    use std::str;
    match ISO_8859_1.decode(s, DecoderTrap::Replace) {
        Ok(s) => s,
        Err(_) => match str::from_utf8(s) {
            Ok(s) => s.to_string(),
            Err(_) => format!("[invalid string data]"),
        }
    }
}

#[test]
fn test_note_num_to_name() {
    assert_eq!(&note_num_to_name(48)[..],"C3");
    assert_eq!(&note_num_to_name(49)[..],"C#3");
    assert_eq!(&note_num_to_name(65)[..],"F4");
    assert_eq!(&note_num_to_name(104)[..],"G#7");
}
