extern crate rimd;

use rimd::{SMF};
use std::env::{args,Args};
use std::path::Path;
use std::convert::TryFrom;

fn main() {
    let mut args: Args = args();
    args.next();
    let pathstr = match args.next() {
        Some(s) => s,
        None => { panic!("Please pass a path to an SMF to test") },
    };
    println!("Reading: {}",pathstr);
    match SMF::try_from(Path::new(&pathstr[..])) {
        Ok(smf) => {
            println!("{}", smf);
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
            println!("{}", e);
        }
    }
}
