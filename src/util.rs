//! Some useful utility functions

use std::num::Float;

static NSTRS: &'static str = "C C#D D#E F F#G G#A A#B ";

/// convert a midi note number to a name
pub fn note_num_to_name(num: u32) -> String {
    let oct = (num as f32 /12 as f32).floor()-1.0;
    let nmt = ((num%12)*2) as usize;
    let slice =
        if NSTRS.char_at(nmt+1) == ' ' {
            &NSTRS[nmt..(nmt+1)]
        } else {
            &NSTRS[nmt..(nmt+2)]
        };
    format!("{}{}",slice,oct)
}

#[test]
fn test_note_num_to_name() {
    assert_eq!(note_num_to_name(48).as_slice(),"C3");
    assert_eq!(note_num_to_name(49).as_slice(),"C#3");
    assert_eq!(note_num_to_name(65).as_slice(),"F4");
    assert_eq!(note_num_to_name(104).as_slice(),"G#7");
}
