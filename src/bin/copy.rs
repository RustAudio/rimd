extern crate rimd;

use rimd::{SMF,SMFError,SMFWriter};
use std::env::{args,Args};
use std::path::Path;

fn main() {
    let mut args: Args = args();
    args.next();
    let pathstr = match args.next() {
        Some(s) => s,
        None => { panic!("Need a source path"); }
    };
    let deststr = match args.next() {
        Some(s) => s,
        None => { panic!("Need a destination path") },
    };
    match SMF::from_file(&Path::new(&pathstr[..])) {
        Ok(smf) => {
            let writer = SMFWriter::from_smf(smf);
            writer.write_to_file(&Path::new(&deststr[..])).unwrap();
        }
        Err(e) => {
            match e {
                SMFError::InvalidSMFFile(s) => {println!("{}",s);}
                SMFError::Error(e) => {println!("io: {}",e);}
                SMFError::MidiError(_) => {println!("Midi Error");}
                SMFError::MetaError(_) => {println!("Meta Error");}
            }
        }
    }
}
