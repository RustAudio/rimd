use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::ops::IndexMut;

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

    /// Return true if the event inside this AbsoluteEvent is a midi
    /// event, false if it's a meta event
    pub fn is_midi(&self) -> bool {
        match self.event {
            Event::Midi(_) => true,
            Event::Meta(_) => false,
        }
    }

    /// Return true if the event inside this AbsoluteEvent is a meta
    /// event, false if it's a midi event
    pub fn is_meta(&self) -> bool {
        match self.event {
            Event::Midi(_) => false,
            Event::Meta(_) => true,
        }
    }

    pub fn get_event(&self) -> &Event {
        &self.event
    }

    pub fn get_time(&self) -> u64 {
        self.time
    }
}

impl Eq for AbsoluteEvent {}

impl PartialEq for AbsoluteEvent {
    fn eq(&self, other: &AbsoluteEvent) -> bool {
        if self.time == other.time {
            match (&self.event,&other.event) {
                (&Event::Midi(_),&Event::Meta(_)) => false,
                (&Event::Meta(_),&Event::Midi(_)) => false,
                (&Event::Meta(ref me),&Event::Meta(ref you)) => {
                    me.command == you.command
                },
                (&Event::Midi(ref me),&Event::Midi(ref you)) => {
                    me.data(0) == you.data(0)
                        &&
                    me.data(1) == me.data(1)
                },
            }
        } else {
            false
        }
    }

    fn ne(&self, other: &AbsoluteEvent) -> bool {
        !(self.eq(other))
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
                    (&Event::Meta(ref me),&Event::Meta(ref you)) => {
                        me.command.cmp(&you.command)
                    },
                    (&Event::Midi(ref me),&Event::Midi(ref you)) => {
                        if      me.data(0) < you.data(0) { Ordering::Less }
                        else if me.data(0) > you.data(0) { Ordering::Greater }
                        else {
                            if me.data(1) < you.data(1) {
                                Ordering::Less
                            } else if me.data(1) > you.data(1) {
                                Ordering::Greater
                            } else {
                                res
                            }
                        }
                    },
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

enum EventContainer {
    Heap(BinaryHeap<AbsoluteEvent>),
    Static(Vec<TrackEvent>),
}

struct TrackBuilder {
    copyright: Option<String>,
    name: Option<String>,
    events: EventContainer,
}

impl TrackBuilder {

    fn result(self) -> Track {
        Track {
            copyright: self.copyright,
            name: self.name,
            events: match self.events {
                EventContainer::Heap(heap) => {
                    let mut events = Vec::with_capacity(heap.len());
                    let absevents = heap.into_sorted_vec();
                    let mut prev_time = 0;
                    for ev in absevents.into_iter() {
                        let vtime =
                            if prev_time == 0 {
                                ev.time
                            } else {
                                ev.time - prev_time
                            };
                        prev_time = ev.time;
                        events.push(TrackEvent {
                            vtime: vtime,
                            event: ev.event,
                        });
                    }
                    events
                },
                EventContainer::Static(vec) => vec,
            },
        }
    }

    fn abs_time_from_delta(&self,delta: u64) -> u64 {
        match self.events {
            EventContainer::Heap(ref heap) => {
                match heap.peek() {
                    Some(e) => { e.time + delta }
                    None => { delta }
                }
            }
            _ => { panic!("Can't call abs_time_from_delta on non-heap builder") }
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
            events: EventContainer::Heap(BinaryHeap::new()),
        });
    }

    /// Add a static track to the builder (note this will clone all events in the passed iterator)
    pub fn add_static_track<'a,I>(&mut self, track: I) where I: Iterator<Item=&'a AbsoluteEvent> {
        let mut cur_time: u64 = 0;
        let vec = track.map(|bev| {
            assert!(bev.time >= cur_time);
            let vtime = bev.time - cur_time;
            cur_time = vtime;
            TrackEvent {
                vtime: vtime,
                event: bev.event.clone(),
            }
        }).collect();
        self.tracks.push(TrackBuilder {
            copyright: None,
            name: None,
            events: EventContainer::Static(vec),
        });
    }

    /// Set the copyright for the track at index `track`.  This will
    /// also cause a copyright meta event to be inserted.
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this
    /// builder, or if the track already has a copyright set.
    pub fn set_copyright(&mut self, track: usize, copyright: String) {
        assert!(self.tracks.len() > track);
        assert!(self.tracks[track].copyright.is_none());
        // let event = AbsoluteEvent {
        //     time: 0,
        //     event: Event::Meta(MetaEvent::copyright_notice(copyright.clone())),
        // };
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
        assert!(self.tracks.len() > track);
        assert!(self.tracks[track].name.is_none());
        // let event = AbsoluteEvent{
        //     time: 0,
        //     event: Event::Meta(MetaEvent::sequence_or_track_name(name.clone())),
        // };
        self.tracks[track].name = Some(name);
    }

    /// Add a midi message to track at index `track` at absolute time
    /// `time`.
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_midi_abs(&mut self, track: usize, time: u64, msg: MidiMessage) {
        assert!(self.tracks.len() > track);
        match self.tracks.index_mut(track).events {
            EventContainer::Heap(ref mut heap) => {
                heap.push(AbsoluteEvent {
                    time: time,
                    event: Event::Midi(msg),
                });
            }
            _ => { panic!("Can't add events to static tracks") }
        }
    }

    /// Add a midi message to track at index `track` at `delta` ticks
    /// after the last message (or at `delta` if no current messages
    /// exist)
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_midi_rel(&mut self, track: usize, delta: u64, msg: MidiMessage) {
        assert!(self.tracks.len() > track);
        let time = self.tracks[track].abs_time_from_delta(delta);
        self.add_midi_abs(track,time,msg);
    }

    /// Add a meta event to track at index `track` at absolute  time
    /// `time`.
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_meta_abs(&mut self, track: usize, time: u64, event: MetaEvent) {
        assert!(self.tracks.len() > track);
        match self.tracks.index_mut(track).events {
            EventContainer::Heap(ref mut heap) => {
                heap.push(AbsoluteEvent {
                    time: time,
                    event: Event::Meta(event),
                });
            }
            _ => { panic!("Can't add events to static tracks") }
        }
    }

    /// Add a meta event to track at index `track` at `delta` ticks
    /// after the last message (or at `delta` if no current messages
    /// exist)
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_meta_rel(&mut self, track: usize, delta: u64, event: MetaEvent) {
        assert!(self.tracks.len() > track);
        let time = self.tracks[track].abs_time_from_delta(delta);
        self.add_meta_abs(track,time,event);
    }

    /// Add a TrackEvent to the track at index `track`.  The event
    /// will be added at `event.vtime` after the last event currently
    /// in the builder for the track.
    ///
    /// ## Panics
    ///
    /// Panics if `track` is >= to the number of tracks in this builder
    pub fn add_event(&mut self, track: usize, event: TrackEvent) {
        assert!(self.tracks.len() > track);
        let bevent = AbsoluteEvent {
            time: self.tracks[track].abs_time_from_delta(event.vtime),
            event: event.event,
        };
        match self.tracks.index_mut(track).events {
            EventContainer::Heap(ref mut heap) => {
                heap.push(bevent);
            }
            _ => { panic!("Can't add events to static tracks") }
        }
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

#[test]
fn simple_build() {
    let note_on = MidiMessage::note_on(69,100,0);
    let note_off = MidiMessage::note_off(69,100,0);


    let mut builder = SMFBuilder::new();
    builder.add_track();

    builder.add_event(0, TrackEvent{vtime: 0, event: Event::Midi(note_on)});
    builder.add_event(0, TrackEvent{vtime: 10, event: Event::Midi(note_off)});
    builder.result();
}
