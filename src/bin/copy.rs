#![feature(env,os,path)]
extern crate rimd;

use rimd::{SMF,SMFError,SMFWriter};
use std::env::{args,Args};

fn main() {
    let mut args: Args = args();
    args.next();
    let pathstr = match args.next().unwrap().into_string().clone() {
        Ok(s) => s,
        Err(_) => { panic!("Invalid path") },
    };
    let deststr = match args.next().unwrap().into_string().clone() {
        Ok(s) => s,
        Err(_) => { panic!("Invalid destination") },
    };
    match SMF::from_file(&Path::new(pathstr)) {
        Ok(smf) => {
            let writer = SMFWriter::from_smf(smf);
            writer.write_to_file(&Path::new(deststr)).unwrap();
        }
        Err(e) => {
            match e {
                SMFError::InvalidSMFFile(s) => {println!("{}",s);}
                SMFError::IoError(e) => {println!("io: {}",e);}
                SMFError::MidiError(_) => {println!("Midi Error");}
                SMFError::MetaError(_) => {println!("Meta Error");}
            }
        }
    }
}
