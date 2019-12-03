extern crate rimd;

use rimd::{SMF,SMFWriter};
use std::env::{args,Args};
use std::path::Path;
use std::convert::TryFrom;

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
    match SMF::try_from(Path::new(&pathstr[..])) {
        Ok(smf) => {
            let writer = SMFWriter::from(smf);
            writer.write_to_file(&Path::new(&deststr[..])).unwrap();
        }
        Err(e) => {
            println!("{}",e)
        }
    }
}
