use std::collections::BTreeSet;

use ::{SMF,Event,SMFFormat,MetaEvent,MidiMessage,Track,TrackEvent};

pub struct TrackBuilder {
    copyright: Option<String>,
    name: Option<String>,
    events: BTreeSet<TrackEvent>,
}

impl TrackBuilder {

    /// Set the copyright for this track.  This will also cause
    /// a copyright meta event to be inserted.  Calling this method more
    /// than once will result in an assertion failure.
    pub fn set_copyright(&mut self, copyright: String) {
        assert!(self.copyright.is_none());
        let event = TrackEvent{
            vtime: 0,
            event: Event::Meta(MetaEvent::copyright_notice(copyright.clone())),
        };
        self.events.insert(event);
        self.copyright = Some(copyright);
    }

    /// Set the copyright for this track.  This will also cause
    /// a name meta event to be inserted.  Calling this method more
    /// than once will result in an assertion failure.
    pub fn set_name(&mut self, name: String) {
        assert!(self.name.is_none());
        let event = TrackEvent{
            vtime: 0,
            event: Event::Meta(MetaEvent::sequence_or_track_name(name.clone())),
        };
        self.events.insert(event);
        self.name = Some(name);
    }

    fn result(self) -> Track {
        Track {
            copyright: self.copyright,
            name: self.name,
            events: self.events.into_iter().collect(),
        }
    }
}

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

    /// Add new a track to this builder
    pub fn add_track(&mut self) {
        self.tracks.push(TrackBuilder {
            copyright: None,
            name: None,
            events: BTreeSet::new()
        });
    }

    /// Add a midi message to track at index `track` at time `time`.
    pub fn add_midi(&mut self, track: usize, time: u64, msg: MidiMessage) {
        assert!(self.tracks.len() < track);
        self.tracks[track].events.insert(TrackEvent {
            vtime: time,
            event: Event::Midi(msg),
        });
    }

    /// Add a meta event to track at index `track` at time `time`.
    pub fn add_meta(&mut self, track: usize, time: u64, event: MetaEvent) {
        assert!(self.tracks.len() < track);
        self.tracks[track].events.insert(TrackEvent {
            vtime: time,
            event: Event::Meta(event),
        });
    }

    /// Add a TrackEvent to the track at index `track`
    pub fn add_event(&mut self, track: usize, event: TrackEvent) {
        assert!(self.tracks.len() < track);
        self.tracks[track].events.insert(event);
    }

    pub fn result(self) -> SMF {
        SMF {
            format: SMFFormat::MultiTrack,
            tracks: self.tracks.into_iter().map(|tb| tb.result()).collect(),
            division: 0,
        }
    }
}
