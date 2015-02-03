use std::cmp::Ordering;
use std::collections::BinaryHeap;


use ::{SMF,Event,SMFFormat,MetaEvent,MidiMessage,Track,TrackEvent};

/// An AbsoluteEvent is an event that has an absolute time
/// This is useful for apps that want to store events internally
/// with absolute times and then quickly build an SMF file for saving etc...
pub struct AbsoluteEvent {
    time: u64,
    event: Event,
}

impl AbsoluteEvent {
    pub fn new_midi(time: u64, midi: MidiMessage) -> AbsoluteEvent {
        AbsoluteEvent {
            time: time,
            event: Event::Midi(midi),
        }
    }
    pub fn new_meta(time: u64, meta: MetaEvent) -> AbsoluteEvent {
        AbsoluteEvent {
            time: time,
            event: Event::Meta(meta),
        }
    }
}

impl Eq for AbsoluteEvent {}

impl PartialEq for AbsoluteEvent {
    fn eq(&self, other: &AbsoluteEvent) -> bool {
        self.time == other.time
    }

    fn ne(&self, other: &AbsoluteEvent) -> bool {
        self.time != other.time
    }
}

// Implement `Ord` and sort messages by time
impl Ord for AbsoluteEvent {
    fn cmp(&self, other: &AbsoluteEvent) -> Ordering {
        let res = self.time.cmp(&other.time);
        match res {
            // vtime takes priority
            Ordering::Less | Ordering::Greater => res,
            // if vtime is the same, check types and make meta events
            // sort before standard events
            Ordering::Equal => {
                match (&self.event,&other.event) {
                    // I'm midi, other is meta, so I'm greater
                    (&Event::Midi(_),&Event::Meta(_)) => Ordering::Greater,
                    // I'm meta, other is midi, so I'm less
                    (&Event::Meta(_),&Event::Midi(_)) => Ordering::Less,
                    // same type, so just use above res as Equal
                    _ => res
                }
            }
        }
    }
}

impl PartialOrd for AbsoluteEvent {
    fn partial_cmp(&self, other: &AbsoluteEvent) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct TrackBuilder {
    copyright: Option<String>,
    name: Option<String>,
    events: BinaryHeap<AbsoluteEvent>,
}

impl TrackBuilder {

    fn result(self) -> Track {
        let mut cur_time: u64 = 0;
        Track {
            copyright: self.copyright,
            name: self.name,
            events: self.events.into_iter().map(|bev| {
                let vtime = bev.time - cur_time;
                cur_time = vtime;
                TrackEvent {
                    vtime: vtime,
                    event: bev.event,
                }
            }).collect(),
        }
    }

    fn abs_time_from_delta(&self,delta: u64) -> u64 {
        match self.events.peek() {
            Some(e) => { e.time + delta }
            None => { delta }
        }
    }
}

/// An SMFBuilder can be used to create an SMF file.  This is done by
/// adding tracks to the builder via `add_track` and then adding
/// events to each track.
pub struct SMFBuilder {
    tracks:Vec<TrackBuilder>
}

impl SMFBuilder {
    /// Create a new SMFBuilder.  Initially the builder will have no tracks
    pub fn new() -> SMFBuilder {
        SMFBuilder {
            tracks: Vec::new(),
        }
    }

    /// Get the number of tracks currenly in the builder
    pub fn num_tracks(&self) -> usize {
        self.tracks.len()
    }

    /// Add new a track to this builder
    pub fn add_track(&mut self) {
        self.tracks.push(TrackBuilder {
            copyright: None,
            name: None,
            events: BinaryHeap::new()
        });
    }

    /// Set the copyright for the track at index `track`.  This will
    /// also cause a copyright meta event to be inserted.
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this
    /// builder, or if the track already has a copyright set.
    pub fn set_copyright(&mut self, track: usize, copyright: String) {
        assert!(self.tracks.len() < track);
        assert!(self.tracks[track].copyright.is_none());
        let event = AbsoluteEvent {
            time: 0,
            event: Event::Meta(MetaEvent::copyright_notice(copyright.clone())),
        };
        self.tracks[track].events.push(event);
        self.tracks[track].copyright = Some(copyright);
    }

    /// Set the name for the track at index `track`.  This will
    /// also cause a name meta event to be inserted.
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this
    /// builder, or if the track already has a name set.
    pub fn set_name(&mut self, track: usize, name: String) {
        assert!(self.tracks.len() < track);
        assert!(self.tracks[track].name.is_none());
        let event = AbsoluteEvent{
            time: 0,
            event: Event::Meta(MetaEvent::sequence_or_track_name(name.clone())),
        };
        self.tracks[track].events.push(event);
        self.tracks[track].name = Some(name);
    }

    /// Add a midi message to track at index `track` at absolute time
    /// `time`.
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_midi_abs(&mut self, track: usize, time: u64, msg: MidiMessage) {
        assert!(self.tracks.len() < track);
        self.tracks[track].events.push(AbsoluteEvent {
            time: time,
            event: Event::Midi(msg),
        });
    }

    /// Add a midi message to track at index `track` at `delta` ticks
    /// after the last message (or at `delta` if no current messages
    /// exist)
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_midi_rel(&mut self, track: usize, delta: u64, msg: MidiMessage) {
        assert!(self.tracks.len() < track);
        let time = self.tracks[track].abs_time_from_delta(delta);
        self.tracks[track].events.push(AbsoluteEvent {
            time: time,
            event: Event::Midi(msg),
        });
    }

    /// Add a meta event to track at index `track` at absolute  time
    /// `time`.
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_meta_abs(&mut self, track: usize, time: u64, event: MetaEvent) {
        assert!(self.tracks.len() < track);
        self.tracks[track].events.push(AbsoluteEvent {
            time: time,
            event: Event::Meta(event),
        });
    }

    /// Add a meta event to track at index `track` at `delta` ticks
    /// after the last message (or at `delta` if no current messages
    /// exist)
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_meta_rel(&mut self, track: usize, delta: u64, event: MetaEvent) {
        assert!(self.tracks.len() < track);
        let time = self.tracks[track].abs_time_from_delta(delta);
        self.tracks[track].events.push(AbsoluteEvent {
            time: time,
            event: Event::Meta(event),
        });
    }

    /// Add a TrackEvent to the track at index `track`.  The event
    /// will be added at `event.vtime` after the last event currently
    /// in the builder for the track.
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_event(&mut self, track: usize, event: TrackEvent) {
        assert!(self.tracks.len() < track);
        let bevent = AbsoluteEvent {
            time: self.tracks[track].abs_time_from_delta(event.vtime),
            event: event.event,
        };
        self.tracks[track].events.push(bevent);
    }

    /// Generate an SMF file with the events that have been added to
    /// the builder
    pub fn result(self) -> SMF {
        SMF {
            format: SMFFormat::MultiTrack,
            tracks: self.tracks.into_iter().map(|tb| tb.result()).collect(),
            division: 0,
        }
    }
}
