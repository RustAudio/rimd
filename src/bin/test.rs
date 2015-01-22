#![allow(unstable)]
extern crate rimd;

use rimd::{SMF,SMFError};

fn main() {
    let args: Vec<String> = std::os::args();
    match SMF::from_file(&Path::new(args[1].clone())) {
        Ok(smf) => {
            println!("format: {}",smf.format);
            println!("tracks: {}",smf.tracks.len());
            println!("division: {}",smf.division);
            let mut tnum = 1;
            for track in smf.tracks.iter() {
                println!("\n{}: {}\nevents:",tnum,track);
                tnum+=1;
                for event in track.events.iter() {
                    println!("  {}",event);
                }
            }
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
