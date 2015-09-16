extern crate rimd;

use rimd::{SMF,SMFError};
use std::env::{args,Args};
use std::path::Path;

fn main() {
    let mut args: Args = args();
    args.next();
    let pathstr = match args.next() {
        Some(s) => s,
        None => { panic!("Please pass a path to an SMF to test") },
    };
    println!("Reading: {}",pathstr);
    match SMF::from_file(&Path::new(&pathstr[..])) {
        Ok(smf) => {
            println!("format: {}",smf.format);
            println!("tracks: {}",smf.tracks.len());
            println!("division: {}",smf.division);
            let mut tnum = 1;
            for track in smf.tracks.iter() {
                let mut time: u64 = 0;
                println!("\n{}: {}\nevents:",tnum,track);
                tnum+=1;
                for event in track.events.iter() {
                    println!("  {}",event.fmt_with_time_offset(time));
                    time += event.vtime;
                }
            }
        }
        Err(e) => {
            match e {
                SMFError::InvalidSMFFile(s) => {println!("{}",s);}
                SMFError::Error(e) => {println!("io: {}",e);}
                SMFError::MidiError(e) => {println!("Midi Error: {}",e);}
                SMFError::MetaError(_) => {println!("Meta Error");}
            }
        }
    }
}
