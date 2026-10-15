//! The project: everything a song is made of, apart from the audio itself.
//!
//! A project holds tracks, and tracks hold regions that point into audio files.
//! It is plain data, so it can be saved, copied for undo, and tested without
//! any audio device or UI.
//!
//! Positions on the timeline are in beats (one note of the time signature's
//! denominator); positions inside audio files are in seconds.

mod history;
mod storage;

use std::collections::BTreeMap;

pub use isai_engine::effects::{EffectKind, MAX_PARAMS};
pub use isai_engine::instruments::{InstrumentKind, MAX_INST_PARAMS};
use isai_engine::{Meter, TempoMap};
use serde::{Deserialize, Serialize};

pub use history::History;
pub use storage::{AUDIO_DIR, Autosave, EXTENSION, ProjectPaths, load, resolve, save};

/// Bump when the file format changes, and upgrade older files in `storage::load`.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Snap {
    Bar,
    Beat,
    Half,
    Quarter,
}

impl Snap {
    pub fn key(self) -> &'static str {
        match self {
            Snap::Bar => "bar",
            Snap::Beat => "beat",
            Snap::Half => "1/2",
            Snap::Quarter => "1/4",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "bar" => Snap::Bar,
            "beat" => Snap::Beat,
            "1/2" => Snap::Half,
            "1/4" => Snap::Quarter,
            _ => return None,
        })
    }

    /// Grid step in beats.
    pub fn step(self, meter: &Meter) -> f64 {
        match self {
            Snap::Bar => f64::from(meter.numerator.max(1)),
            Snap::Beat => 1.0,
            Snap::Half => 0.5,
            Snap::Quarter => 0.25,
        }
    }
}

/// A piece of an audio file placed on a track.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Region {
    pub id: u64,
    pub name: String,
    /// The audio file. Relative to the project's Audio folder once saved;
    /// an absolute path for audio imported into a project not yet saved.
    pub file: String,
    /// Where the region starts on the timeline, in beats.
    pub start: f64,
    /// Where in the file the region starts playing, in seconds.
    pub offset: f64,
    /// How much of the file the region plays, in seconds.
    pub length: f64,
    /// Fade-in and fade-out lengths, in seconds.
    #[serde(default)]
    pub fade_in: f64,
    #[serde(default)]
    pub fade_out: f64,
    /// Length of the whole audio file in seconds, so trimming can't go past
    /// its end. 0 when unknown.
    #[serde(default)]
    pub source_length: f64,
    /// Set for a MIDI region (notes instead of audio; `file`, `offset`,
    /// `length` and the fades are unused).
    #[serde(default)]
    pub midi: Option<MidiData>,
    /// Volume change in decibels (0 = as recorded). Audio regions only.
    #[serde(default)]
    pub gain_db: f32,
    /// Plays the audio backwards. Audio regions only.
    #[serde(default)]
    pub reverse: bool,
    /// The region's contents repeat until this many beats from its start
    /// (dragging its top-right corner); 0 when it doesn't loop.
    #[serde(default)]
    pub loop_to: f64,
    /// Follow the song's tempo: the tempo this audio was recorded at. The
    /// region keeps its length in beats and its audio is time-stretched to
    /// fit when the song's tempo is different. Audio regions only.
    #[serde(default)]
    pub tempo: Option<f64>,
}

/// A tempo change: from `beat` on, the song plays at `bpm`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct TempoChange {
    pub beat: f64,
    pub bpm: f64,
}

/// A time-signature change at the start of `bar` (counted from 1, so 2 or later).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct SignatureChange {
    pub bar: u32,
    pub numerator: u32,
    pub denominator: u32,
}

/// A named place on the timeline (Intro, Verse, Chorus…).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Marker {
    pub id: u64,
    /// Where it is, in beats.
    pub beat: f64,
    pub name: String,
}

/// A MIDI region's notes. Everything is in beats, so it follows the tempo.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MidiData {
    /// How long the region is.
    pub length: f64,
    /// Where the region starts within its notes (moves when the left edge is trimmed).
    pub offset: f64,
    /// Sorted by start, then key.
    pub notes: Vec<Note>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Note {
    /// Beats from the start of the notes (not of the region; see `MidiData::offset`).
    pub start: f64,
    pub length: f64,
    /// MIDI note number: 60 is middle C.
    pub key: u8,
    /// 1..=127
    pub velocity: u8,
}

/// A third-party plugin (VST3 now; CLAP and Audio Units next) on an insert
/// or as an instrument, and its saved state.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct PluginSlot {
    /// "vst3" or "clap".
    pub format: String,
    /// Where the plugin is installed.
    pub path: String,
    /// Which plugin in the bundle.
    pub uid: String,
    pub name: String,
    #[serde(default)]
    pub vendor: String,
    /// The plugin's own state (its sound and settings), base64, from the last save.
    #[serde(default)]
    pub state: Option<String>,
}

/// A track's software instrument (it makes the track an instrument track).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Instrument {
    pub id: u64,
    /// `InstrumentKind::key()`, e.g. "synth".
    pub kind: String,
    pub params: BTreeMap<String, f32>,
    /// The sampler's audio file (like a region's `file`), if it has one.
    #[serde(default)]
    pub sample: Option<String>,
    /// Where each slice starts in the sample, in seconds (the sampler's slices mode).
    #[serde(default)]
    pub slices: Vec<f64>,
    /// The plugin, when `kind` is "plugin".
    #[serde(default)]
    pub plugin: Option<PluginSlot>,
}

impl Instrument {
    pub fn instrument_kind(&self) -> Option<InstrumentKind> {
        if self.kind == InstrumentKind::Plugin.key() {
            return Some(InstrumentKind::Plugin);
        }
        InstrumentKind::from_key(&self.kind)
    }

    /// Values in the instrument's parameter order, for the engine.
    pub fn values(&self) -> [f32; MAX_INST_PARAMS] {
        let Some(kind) = self.instrument_kind() else { return [0.0; MAX_INST_PARAMS] };
        let mut out = kind.defaults();
        for (o, def) in out.iter_mut().zip(kind.params()) {
            if let Some(&v) = self.params.get(def.key) {
                *o = v.clamp(def.min, def.max);
            }
        }
        out
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Track {
    pub id: u64,
    pub name: String,
    /// A colour key from the app's track palette, e.g. "blue".
    pub color: String,
    /// Fader level in decibels; 0 is unity, `MIN_DB` and below is silent.
    pub volume_db: f32,
    /// -1 is hard left, 0 centre, 1 hard right.
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    /// Armed for recording: the next take lands on this track.
    #[serde(default)]
    pub armed: bool,
    /// Hear the live input through this track while armed.
    #[serde(default)]
    pub monitor: bool,
    /// Copies of the track's sound (after its fader) sent to buses.
    #[serde(default)]
    pub sends: Vec<Send>,
    /// Effects, in order, before the fader.
    #[serde(default)]
    pub inserts: Vec<Insert>,
    /// Parameters that change over time.
    #[serde(default)]
    pub automation: Vec<Lane>,
    /// The automation lane shown under the track, if any (a `Lane::target`).
    #[serde(default)]
    pub automation_view: Option<String>,
    /// Set for an instrument track (MIDI regions, played by this instrument).
    #[serde(default)]
    pub instrument: Option<Instrument>,
    /// A frozen track plays this rendered file (its instrument, regions and
    /// effects, baked) instead, which saves processing power. Its fader, pan
    /// and sends still work.
    #[serde(default)]
    pub frozen: Option<String>,
    pub regions: Vec<Region>,
}

/// A point on an automation lane.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct AutoPoint {
    pub beat: f64,
    pub value: f32,
}

/// A track parameter over time. `target` is "volume", "pan",
/// "send:<bus id>" or "fx:<insert id>:<parameter key>".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Lane {
    pub target: String,
    /// Sorted by beat.
    pub points: Vec<AutoPoint>,
}

/// An effect in a track's, bus's or the master's chain.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Insert {
    pub id: u64,
    /// `EffectKind::key()`, e.g. "reverb".
    pub kind: String,
    pub enabled: bool,
    /// Parameter values by key. Missing ones use the effect's default, so
    /// effects can gain parameters without breaking old projects.
    pub params: BTreeMap<String, f32>,
    /// The plugin, when `kind` is "plugin".
    #[serde(default)]
    pub plugin: Option<PluginSlot>,
}

impl Insert {
    pub fn effect_kind(&self) -> Option<EffectKind> {
        if self.kind == EffectKind::Plugin.key() {
            return Some(EffectKind::Plugin);
        }
        EffectKind::from_key(&self.kind)
    }

    /// The name shown for it: the effect's, or the plugin's.
    pub fn label(&self) -> String {
        match (&self.plugin, self.effect_kind()) {
            (Some(p), _) => p.name.clone(),
            (None, Some(k)) => k.label().into(),
            (None, None) => self.kind.clone(),
        }
    }

    /// Values in the effect's parameter order, for the engine.
    pub fn values(&self) -> [f32; MAX_PARAMS] {
        let Some(kind) = self.effect_kind() else { return [0.0; MAX_PARAMS] };
        let mut out = kind.defaults();
        for (o, def) in out.iter_mut().zip(kind.params()) {
            if let Some(&v) = self.params.get(def.key) {
                *o = v.clamp(def.min, def.max);
            }
        }
        out
    }
}

/// Which chain of effects: a track's, a bus's or the master's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chain {
    Track(u64),
    Bus(u64),
    Master,
}

/// Part of a track's sound sent to a bus, e.g. to share one reverb.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Send {
    pub bus: u64,
    /// Send level in decibels; `Track::MIN_DB` and below sends nothing.
    pub level_db: f32,
}

/// A bus (aux channel): tracks send to it, it plays into the master.
/// Shared effects such as reverb and delay go on buses.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Bus {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub volume_db: f32,
    pub pan: f32,
    pub mute: bool,
    #[serde(default)]
    pub inserts: Vec<Insert>,
}

/// Linear gain for a fader or send level in dB (silent at `Track::MIN_DB`).
pub fn db_to_gain(db: f32) -> f32 {
    if db <= Track::MIN_DB { 0.0 } else { 10f32.powf(db.min(Track::MAX_DB) / 20.0) }
}

impl Track {
    pub const MIN_DB: f32 = -60.0;
    pub const MAX_DB: f32 = 6.0;

    /// Linear gain for the fader position.
    pub fn gain(&self) -> f32 {
        db_to_gain(self.volume_db)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Project {
    pub version: u32,
    pub name: String,
    /// Quarter notes per minute.
    pub tempo: f64,
    pub numerator: u32,
    pub denominator: u32,
    /// e.g. "C major"
    pub key: String,
    pub metronome: bool,
    pub count_in: bool,
    pub cycle: bool,
    pub cycle_start: f64,
    pub cycle_end: f64,
    pub snap: bool,
    pub snap_value: Snap,
    pub tracks: Vec<Track>,
    pub buses: Vec<Bus>,
    /// Effects on the master, after everything is mixed.
    pub master_inserts: Vec<Insert>,
    /// Named places on the timeline, sorted by beat.
    pub markers: Vec<Marker>,
    /// Tempo changes after the start, sorted by beat. `tempo` is the opening tempo.
    pub tempo_changes: Vec<TempoChange>,
    /// Time-signature changes after bar 1, sorted by bar. `numerator` and
    /// `denominator` are the opening signature (and the denominator is the beat unit).
    pub signature_changes: Vec<SignatureChange>,
    /// Source of unique ids for tracks and regions.
    pub next_id: u64,
}

impl Default for Project {
    fn default() -> Self {
        Self::new("Untitled", Meter::default(), "C major")
    }
}

/// Regions copied with ⌘C or ⌘X.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clipboard {
    /// Track row relative to the topmost copied region's track, and the
    /// region with `start` relative to the earliest copied region.
    pub items: Vec<(usize, Region)>,
    /// Beats from the earliest start to the latest end.
    pub span: f64,
}

/// A key name like "C major", "F♯ minor" or "Bb major": the root's pitch
/// class (0 = C) and whether it is minor.
pub fn parse_key(key: &str) -> Option<(u8, bool)> {
    let mut chars = key.trim().chars();
    let base: i32 = match chars.next()?.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let shift = match rest.chars().next() {
        Some('#' | '♯') => 1,
        Some('b' | '♭') => -1,
        _ => 0,
    };
    Some(((base + shift).rem_euclid(12) as u8, rest.to_lowercase().contains("min")))
}

/// Track colours, in the order new tracks use them.
pub const TRACK_COLORS: [&str; 8] = ["blue", "green", "yellow", "orange", "red", "pink", "violet", "teal"];

/// Empty projects are at least this many bars long, like a fresh DAW session.
const MIN_BARS: f64 = 32.0;
/// Extra room after the last region so there is always space to keep working.
const TAIL_BARS: f64 = 4.0;

impl Project {
    pub fn new(name: &str, meter: Meter, key: &str) -> Self {
        let bar = f64::from(meter.numerator.max(1));
        Self {
            version: FORMAT_VERSION,
            name: if name.trim().is_empty() { "Untitled".into() } else { name.trim().into() },
            tempo: meter.bpm,
            numerator: meter.numerator,
            denominator: meter.denominator,
            key: key.into(),
            metronome: false,
            count_in: false,
            cycle: false,
            // A four-bar cycle from the start, ready to switch on.
            cycle_start: 0.0,
            cycle_end: 4.0 * bar,
            snap: true,
            snap_value: Snap::Beat,
            tracks: Vec::new(),
            buses: Vec::new(),
            master_inserts: Vec::new(),
            markers: Vec::new(),
            tempo_changes: Vec::new(),
            signature_changes: Vec::new(),
            next_id: 1,
        }
    }

    pub fn meter(&self) -> Meter {
        Meter { bpm: self.tempo, numerator: self.numerator, denominator: self.denominator }
    }

    /// The tempo and time signature over the whole song.
    pub fn tempo_map(&self) -> TempoMap {
        let tempos: Vec<isai_engine::TempoPoint> = self.tempo_changes.iter().map(|t| isai_engine::TempoPoint { beat: t.beat, bpm: t.bpm }).collect();
        let sigs: Vec<isai_engine::SigChange> = self
            .signature_changes
            .iter()
            .map(|c| isai_engine::SigChange { bar: c.bar, numerator: c.numerator, denominator: c.denominator })
            .collect();
        TempoMap::new(self.meter(), &tempos, &sigs)
    }

    /// The tempo and time signature in force at a position.
    pub fn meter_at(&self, beat: f64) -> Meter {
        self.tempo_map().meter_at(beat)
    }

    /// Adds a tempo change (or changes the one already there). At the very
    /// start it sets the opening tempo instead.
    pub fn add_tempo_change(&mut self, beat: f64, bpm: f64) -> bool {
        let bpm = (bpm.clamp(Meter::MIN_BPM, Meter::MAX_BPM) * 10.0).round() / 10.0;
        let beat = beat.max(0.0);
        if beat < 1e-6 {
            self.tempo = bpm;
            return true;
        }
        match self.tempo_changes.iter_mut().find(|t| (t.beat - beat).abs() < 1e-6) {
            Some(t) => t.bpm = bpm,
            None => self.tempo_changes.push(TempoChange { beat, bpm }),
        }
        self.tempo_changes.sort_by(|a, b| a.beat.total_cmp(&b.beat));
        true
    }

    /// Moves the tempo change at `from` to `to` with a new tempo; at the
    /// start (or dragged onto it) it becomes the opening tempo.
    pub fn move_tempo_change(&mut self, from: f64, to: f64, bpm: f64) -> bool {
        let Some(i) = self.tempo_changes.iter().position(|t| (t.beat - from).abs() < 1e-6) else {
            return from < 1e-6 && self.add_tempo_change(0.0, bpm);
        };
        self.tempo_changes.remove(i);
        self.add_tempo_change(to, bpm)
    }

    pub fn remove_tempo_change(&mut self, beat: f64) -> bool {
        let before = self.tempo_changes.len();
        self.tempo_changes.retain(|t| (t.beat - beat).abs() >= 1e-6);
        self.tempo_changes.len() != before
    }

    /// Sets the tempo of the section the position is in (what the tempo
    /// display and tap tempo change).
    pub fn set_tempo_at(&mut self, beat: f64, bpm: f64) -> bool {
        let at = self.tempo_changes.iter().rev().find(|t| t.beat <= beat + 1e-9).map(|t| t.beat);
        self.add_tempo_change(at.unwrap_or(0.0), bpm)
    }

    /// Adds a time-signature change at the start of `bar` (or changes the one
    /// there). Bar 1 sets the opening signature.
    pub fn add_signature_change(&mut self, bar: u32, numerator: u32, denominator: u32) -> bool {
        let numerator = numerator.clamp(1, 32);
        let denominator = if matches!(denominator, 2 | 4 | 8 | 16) { denominator } else { 4 };
        if bar <= 1 {
            self.numerator = numerator;
            self.denominator = denominator;
            return true;
        }
        match self.signature_changes.iter_mut().find(|c| c.bar == bar) {
            Some(c) => {
                c.numerator = numerator;
                c.denominator = denominator;
            }
            None => self.signature_changes.push(SignatureChange { bar, numerator, denominator }),
        }
        self.signature_changes.sort_by_key(|c| c.bar);
        true
    }

    pub fn remove_signature_change(&mut self, bar: u32) -> bool {
        let before = self.signature_changes.len();
        self.signature_changes.retain(|c| c.bar != bar);
        self.signature_changes.len() != before
    }

    /// Sets the time signature of the section the position is in.
    pub fn set_signature_at(&mut self, beat: f64, numerator: u32, denominator: u32) -> bool {
        let map = self.tempo_map();
        let bar = map.sections().iter().rev().find(|s| s.0 <= beat + 1e-9).map_or(1, |s| s.1) as u32;
        self.add_signature_change(bar, numerator, denominator)
    }

    pub fn set_meter(&mut self, meter: Meter) {
        self.tempo = meter.bpm;
        self.numerator = meter.numerator;
        self.denominator = meter.denominator;
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn bar_beats(&self) -> f64 {
        f64::from(self.numerator.max(1))
    }

    /// Rounds a position to the grid when snap is on.
    pub fn snap(&self, beats: f64) -> f64 {
        if !self.snap {
            return beats.max(0.0);
        }
        if self.snap_value == Snap::Bar && !self.signature_changes.is_empty() {
            // Bars change length: snap to the nearer bar line.
            let map = self.tempo_map();
            let bar = map.bar_at(beats);
            let (a, b) = (map.bar_start(bar), map.bar_start(bar + 1));
            return if beats - a <= b - beats { a } else { b }.max(0.0);
        }
        let step = self.snap_value.step(&self.meter());
        ((beats / step).round() * step).max(0.0)
    }

    /// Sets the cycle region, snapped and in order, at least one grid step long.
    pub fn set_cycle(&mut self, start: f64, end: f64) {
        let (a, b) = if start <= end { (start, end) } else { (end, start) };
        let (a, mut b) = (self.snap(a), self.snap(b));
        let min = if self.snap { self.snap_value.step(&self.meter()) } else { 0.25 };
        if b - a < min {
            b = a + min;
        }
        self.cycle_start = a;
        self.cycle_end = b;
    }

    /// The tempo a region's seconds are measured at: the song's, or the
    /// region's own when it follows the tempo.
    ///
    /// A region's audio is measured at the tempo where it starts (with tempo
    /// changes, audio crossing one keeps its own speed).
    pub fn region_meter(&self, region: &Region) -> Meter {
        let mut m = self.meter();
        if let Some(bpm) = region.tempo.filter(|_| region.midi.is_none()) {
            m.bpm = bpm;
        } else if !self.tempo_changes.is_empty() {
            m.bpm = self.tempo_map().bpm_at(region.start);
        }
        m
    }

    /// How much a following region's audio is stretched to fit (1 = not at all).
    pub fn stretch_ratio(&self, region: &Region) -> f64 {
        let mut here = self.meter();
        here.bpm = self.tempo_map().bpm_at(region.start);
        here.beat_secs() / self.region_meter(region).beat_secs()
    }

    /// Turns the cycle on around these regions: from the earliest start to
    /// the latest end (loops included), exactly, not snapped.
    pub fn cycle_regions(&mut self, ids: &[u64]) -> bool {
        let spans: Vec<(f64, f64)> = ids.iter().filter_map(|&id| self.find_region(id)).map(|(_, r)| (r.start, r.start + self.region_span(r))).collect();
        if spans.is_empty() {
            return false;
        }
        let start = spans.iter().map(|s| s.0).fold(f64::INFINITY, f64::min);
        let end = spans.iter().map(|s| s.1).fold(0.0, f64::max);
        self.cycle_start = start.max(0.0);
        self.cycle_end = end.max(start + 0.25);
        self.cycle = true;
        true
    }

    /// Length of a region on the timeline, in beats, at the current tempo.
    pub fn region_beats(&self, region: &Region) -> f64 {
        match &region.midi {
            Some(m) => m.length,
            // Across tempo changes: where its audio ends on the song's clock.
            None if region.tempo.is_none() && !self.tempo_changes.is_empty() => {
                let map = self.tempo_map();
                map.secs_to_beats(map.beats_to_secs(region.start) + region.length) - region.start
            }
            None => self.region_meter(region).secs_to_beats(region.length),
        }
    }

    /// Makes an audio region follow the song's tempo, its audio being at
    /// `bpm` (or stop following with `None`).
    pub fn set_region_tempo(&mut self, id: u64, bpm: Option<f64>) -> bool {
        let Some(r) = self.region_mut(id).filter(|r| r.midi.is_none()) else { return false };
        r.tempo = bpm.map(|b| b.clamp(Meter::MIN_BPM, Meter::MAX_BPM));
        true
    }

    /// How far a region reaches on the timeline, in beats, including its loop.
    pub fn region_span(&self, region: &Region) -> f64 {
        self.region_beats(region).max(region.loop_to)
    }

    /// Timeline length in beats: whole bars, with room after the last region.
    pub fn length_beats(&self) -> f64 {
        let bar = self.bar_beats();
        let end = self
            .tracks
            .iter()
            .flat_map(|t| &t.regions)
            .map(|r| r.start + self.region_span(r))
            .fold(0.0, f64::max);
        let content = if end > 0.0 { (end / bar).ceil() * bar + TAIL_BARS * bar } else { 0.0 };
        content.max(MIN_BARS * bar).max(self.cycle_end)
    }

    // ------------------------------------------------------------- tracks

    /// Adds an empty audio track at the end and returns its id.
    pub fn add_track(&mut self, name: Option<&str>) -> u64 {
        let id = self.take_id();
        let number = self.tracks.len() + 1;
        self.tracks.push(Track {
            id,
            name: name.map_or_else(|| format!("Audio {number}"), str::to_owned),
            color: TRACK_COLORS[(number - 1) % TRACK_COLORS.len()].into(),
            volume_db: 0.0,
            pan: 0.0,
            mute: false,
            solo: false,
            armed: false,
            monitor: false,
            sends: Vec::new(),
            inserts: Vec::new(),
            automation: Vec::new(),
            automation_view: None,
            instrument: None,
            frozen: None,
            regions: Vec::new(),
        });
        id
    }

    pub fn track(&self, id: u64) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    pub fn track_mut(&mut self, id: u64) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == id)
    }

    pub fn remove_track(&mut self, id: u64) -> bool {
        let before = self.tracks.len();
        self.tracks.retain(|t| t.id != id);
        self.tracks.len() != before
    }

    /// Arms one track for recording (or disarms it); only one is armed at a time.
    pub fn set_armed(&mut self, id: u64, armed: bool) -> bool {
        if self.track(id).is_none() {
            return false;
        }
        for t in &mut self.tracks {
            t.armed = armed && t.id == id;
            if !t.armed {
                t.monitor = false;
            }
        }
        true
    }

    pub fn armed_track(&self) -> Option<&Track> {
        self.tracks.iter().find(|t| t.armed)
    }

    /// Whether a track should be heard, taking every track's solo into account.
    pub fn audible(&self, track: &Track) -> bool {
        let any_solo = self.tracks.iter().any(|t| t.solo);
        !track.mute && (!any_solo || track.solo)
    }

    // ------------------------------------------------- instruments and MIDI

    /// Adds an instrument track at the end and returns its id.
    pub fn add_instrument_track(&mut self, kind: InstrumentKind) -> u64 {
        let number = self.tracks.iter().filter(|t| t.instrument.is_some()).count() + 1;
        let id = self.add_track(Some(&format!("{} {number}", kind.label())));
        let instrument_id = self.take_id();
        let params = kind.params().iter().map(|d| (d.key.to_owned(), d.default)).collect();
        if let Some(t) = self.track_mut(id) {
            t.instrument = Some(Instrument { id: instrument_id, kind: kind.key().into(), params, sample: None, slices: Vec::new(), plugin: None });
        }
        id
    }

    /// Sets one instrument parameter, kept within its range.
    pub fn set_instrument_param(&mut self, track: u64, key: &str, value: f32) -> bool {
        let Some(inst) = self.track_mut(track).and_then(|t| t.instrument.as_mut()) else { return false };
        let Some(def) = inst.instrument_kind().and_then(|k| k.params().iter().find(|d| d.key == key)) else { return false };
        inst.params.insert(key.to_owned(), value.clamp(def.min, def.max));
        true
    }

    /// Loads an audio file into a track's sampler (forgetting old slices).
    pub fn set_instrument_sample(&mut self, track: u64, file: Option<&str>) -> bool {
        let Some(inst) = self.track_mut(track).and_then(|t| t.instrument.as_mut()) else { return false };
        inst.sample = file.map(str::to_owned);
        inst.slices.clear();
        true
    }

    /// Sets where the sampler's slices start (seconds; sorted, duplicates dropped).
    pub fn set_slices(&mut self, track: u64, slices: &[f64]) -> bool {
        let Some(inst) = self.track_mut(track).and_then(|t| t.instrument.as_mut()) else { return false };
        let mut s: Vec<f64> = slices.iter().copied().filter(|x| x.is_finite() && *x >= 0.0).collect();
        s.sort_by(f64::total_cmp);
        s.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
        inst.slices = s;
        true
    }

    /// Adds an empty MIDI region to an instrument track. Returns its id.
    pub fn add_midi_region(&mut self, track: u64, start: f64, length: f64) -> Option<u64> {
        self.track(track)?.instrument.as_ref()?;
        let id = self.take_id();
        let region = Region {
            id,
            name: "MIDI".into(),
            file: String::new(),
            start: self.snap(start).max(0.0),
            offset: 0.0,
            length: 0.0,
            fade_in: 0.0,
            fade_out: 0.0,
            source_length: 0.0,
            midi: Some(MidiData { length: length.max(Self::MIN_MIDI_BEATS), offset: 0.0, notes: Vec::new() }),
            gain_db: 0.0,
            reverse: false,
            loop_to: 0.0,
            tempo: None,
        };
        self.track_mut(track)?.regions.push(region);
        Some(id)
    }

    fn midi_mut(&mut self, region: u64) -> Option<&mut MidiData> {
        self.region_mut(region)?.midi.as_mut()
    }

    /// Adds a note (start in beats from the region's start). Grows the region
    /// to the next bar if the note runs past its end. Returns the note's index.
    pub fn add_note(&mut self, region: u64, start: f64, length: f64, key: u8, velocity: u8) -> Option<usize> {
        let bar = self.bar_beats();
        let m = self.midi_mut(region)?;
        let note = Note {
            start: (start + m.offset).max(m.offset),
            length: length.max(1.0 / 64.0),
            key: key.min(127),
            velocity: velocity.clamp(1, 127),
        };
        let end = note.start + note.length - m.offset;
        if end > m.length {
            m.length = (end / bar).ceil() * bar;
        }
        let i = m.notes.partition_point(|n| (n.start, n.key) < (note.start, note.key));
        m.notes.insert(i, note);
        Some(i)
    }

    /// Replaces a note (moved, resized, transposed or re-velocitied).
    /// Returns its new index.
    pub fn update_note(&mut self, region: u64, index: usize, start: f64, length: f64, key: u8, velocity: u8) -> Option<usize> {
        let m = self.midi_mut(region)?;
        if index >= m.notes.len() {
            return None;
        }
        m.notes.remove(index);
        self.add_note(region, start, length, key, velocity)
    }

    pub fn remove_note(&mut self, region: u64, index: usize) -> bool {
        let Some(m) = self.midi_mut(region) else { return false };
        if index >= m.notes.len() {
            return false;
        }
        m.notes.remove(index);
        true
    }

    /// Moves, transposes and/or resizes several notes together by the same
    /// amounts (one edit). The group stays inside the region's start and the
    /// key range. Returns the notes' new indices.
    pub fn transform_notes(&mut self, region: u64, indices: &[usize], d_start: f64, d_key: i32, d_length: f64) -> Option<Vec<usize>> {
        let bar = self.bar_beats();
        let m = self.midi_mut(region)?;
        let picked: Vec<usize> = {
            let mut v: Vec<usize> = indices.iter().copied().filter(|&i| i < m.notes.len()).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        if picked.is_empty() {
            return None;
        }
        // Keep the whole group in range rather than squashing its edge notes.
        let earliest = picked.iter().map(|&i| m.notes[i].start).fold(f64::INFINITY, f64::min);
        let d_start = d_start.max(m.offset - earliest);
        let (low, high) = picked.iter().fold((127u8, 0u8), |(lo, hi), &i| (lo.min(m.notes[i].key), hi.max(m.notes[i].key)));
        let d_key = d_key.clamp(-i32::from(low), 127 - i32::from(high));
        let mut tagged: Vec<(Note, bool)> = m.notes.iter().map(|&n| (n, false)).collect();
        for &i in &picked {
            let (n, moved) = &mut tagged[i];
            n.start += d_start;
            n.key = (i32::from(n.key) + d_key) as u8;
            n.length = (n.length + d_length).max(1.0 / 64.0);
            *moved = true;
        }
        tagged.sort_by(|(a, _), (b, _)| (a.start, a.key).partial_cmp(&(b.start, b.key)).unwrap_or(std::cmp::Ordering::Equal));
        let end = tagged.iter().map(|(n, _)| n.start + n.length - m.offset).fold(0.0, f64::max);
        if end > m.length {
            m.length = (end / bar).ceil() * bar;
        }
        let out = tagged.iter().enumerate().filter(|(_, (_, moved))| *moved).map(|(i, _)| i).collect();
        m.notes = tagged.into_iter().map(|(n, _)| n).collect();
        Some(out)
    }

    /// Deletes several notes (one edit).
    pub fn remove_notes(&mut self, region: u64, indices: &[usize]) -> bool {
        let Some(m) = self.midi_mut(region) else { return false };
        let before = m.notes.len();
        let mut i = 0;
        m.notes.retain(|_| {
            i += 1;
            !indices.contains(&(i - 1))
        });
        m.notes.len() != before
    }

    /// Moves notes toward the song's `grid` (in beats). `strength` 0..=1 is
    /// how far (1 = exactly on the grid); `ends` also quantizes note ends.
    /// `swing` 0..=0.5 delays every second grid step by that share of a step
    /// (1/3 is a triplet feel). `indices` picks notes, or every note when
    /// empty. The notes stay sorted, so indices change.
    pub fn quantize_notes(&mut self, region: u64, indices: &[usize], grid: f64, strength: f64, ends: bool, swing: f64) -> bool {
        let Some(start) = self.find_region(region).map(|(_, r)| r.start) else { return false };
        let Some(m) = self.midi_mut(region) else { return false };
        if grid <= 0.0 {
            return false;
        }
        let strength = strength.clamp(0.0, 1.0);
        let swing = swing.clamp(0.0, 0.5) * grid;
        // Notes are timed from `start - offset` on the song's timeline.
        let origin = start - m.offset;
        // The nearest grid position, with every odd step pushed late by the swing.
        let nearest = |at: f64| {
            let k = (at / grid).floor() as i64;
            (k - 1..=k + 2)
                .map(|k| k as f64 * grid + if k.rem_euclid(2) == 1 { swing } else { 0.0 })
                .min_by(|a, b| (a - at).abs().total_cmp(&(b - at).abs()))
                .unwrap_or(at)
        };
        let q = |beats: f64| {
            let target = nearest(origin + beats) - origin;
            if strength >= 1.0 { target } else { beats + (target - beats) * strength }
        };
        for (i, n) in m.notes.iter_mut().enumerate() {
            if !indices.is_empty() && !indices.contains(&i) {
                continue;
            }
            let end = n.start + n.length;
            n.start = q(n.start).max(0.0);
            let end = if ends { q(end) } else { end };
            // A note never quantizes to nothing: at least one grid step (or its own length).
            n.length = if end - n.start > 1e-6 { end - n.start } else if ends { grid } else { n.length };
        }
        m.notes.sort_by(|a, b| (a.start, a.key).partial_cmp(&(b.start, b.key)).unwrap_or(std::cmp::Ordering::Equal));
        true
    }

    /// Adds a region holding recorded notes (`start` in song beats). The region
    /// spans whole bars around them. Returns its id.
    pub fn add_recorded_notes(&mut self, track: u64, notes: &[Note]) -> Option<u64> {
        let bar = self.bar_beats();
        let first = notes.iter().map(|n| n.start).fold(f64::INFINITY, f64::min);
        let last = notes.iter().map(|n| n.start + n.length).fold(0.0, f64::max);
        if !first.is_finite() {
            return None;
        }
        let start = (first.max(0.0) / bar).floor() * bar;
        let end = ((last / bar).ceil() * bar).max(start + bar);
        let id = self.add_midi_region(track, start, end - start)?;
        // The region keeps its bar position whatever the snap setting is.
        if let Some(r) = self.region_mut(id) {
            r.start = start;
            r.name = "Recording".into();
        }
        let m = self.midi_mut(id)?;
        m.notes = notes
            .iter()
            .map(|n| Note { start: (n.start - start).max(0.0), length: n.length.max(1.0 / 64.0), key: n.key.min(127), velocity: n.velocity.clamp(1, 127) })
            .collect();
        m.notes.sort_by(|a, b| (a.start, a.key).partial_cmp(&(b.start, b.key)).unwrap_or(std::cmp::Ordering::Equal));
        Some(id)
    }

    /// Adds several notes at once (a chord stamp, a pattern). Starts are in
    /// beats from the region's start. Returns the new notes' indices.
    pub fn add_notes(&mut self, region: u64, notes: &[Note]) -> Option<Vec<usize>> {
        let bar = self.bar_beats();
        let m = self.midi_mut(region)?;
        if notes.is_empty() {
            return None;
        }
        let offset = m.offset;
        let added = notes.iter().map(|n| Note {
            start: (n.start + offset).max(offset),
            length: n.length.max(1.0 / 64.0),
            key: n.key.min(127),
            velocity: n.velocity.clamp(1, 127),
        });
        Some(Self::merge_notes(m, Vec::new(), added.collect(), bar))
    }

    /// Replaces the notes at `remove` with `add` (positions within the notes,
    /// not the region), keeps them sorted and the region long enough, and
    /// returns the indices of the added notes.
    fn merge_notes(m: &mut MidiData, remove: Vec<usize>, add: Vec<Note>, bar: f64) -> Vec<usize> {
        let mut tagged: Vec<(Note, bool)> =
            m.notes.iter().enumerate().filter(|(i, _)| !remove.contains(i)).map(|(_, &n)| (n, false)).collect();
        tagged.extend(add.into_iter().map(|n| (n, true)));
        tagged.sort_by(|(a, _), (b, _)| (a.start, a.key).partial_cmp(&(b.start, b.key)).unwrap_or(std::cmp::Ordering::Equal));
        let end = tagged.iter().map(|(n, _)| n.start + n.length - m.offset).fold(0.0, f64::max);
        if end > m.length {
            m.length = (end / bar).ceil() * bar;
        }
        let out = tagged.iter().enumerate().filter(|(_, (_, new))| *new).map(|(i, _)| i).collect();
        m.notes = tagged.into_iter().map(|(n, _)| n).collect();
        out
    }

    /// The picked notes (every note when `indices` is empty) grouped into
    /// chords: notes starting together, each group sorted from low to high.
    fn chords(m: &MidiData, indices: &[usize]) -> Vec<Vec<usize>> {
        let mut picked: Vec<usize> = if indices.is_empty() {
            (0..m.notes.len()).collect()
        } else {
            indices.iter().copied().filter(|&i| i < m.notes.len()).collect()
        };
        picked.sort_by(|&a, &b| (m.notes[a].start, m.notes[a].key).partial_cmp(&(m.notes[b].start, m.notes[b].key)).unwrap_or(std::cmp::Ordering::Equal));
        picked.dedup();
        let mut groups: Vec<Vec<usize>> = Vec::new();
        for i in picked {
            match groups.last_mut() {
                Some(g) if (m.notes[g[0]].start - m.notes[i].start).abs() < 1.0 / 64.0 => g.push(i),
                _ => groups.push(vec![i]),
            }
        }
        groups
    }

    /// Strums chords: each note of a chord starts `spread` beats after the
    /// one below it (above it when `down`), ending where it did. Returns the
    /// notes' new indices.
    pub fn strum_notes(&mut self, region: u64, indices: &[usize], spread: f64, down: bool) -> Option<Vec<usize>> {
        let bar = self.bar_beats();
        let m = self.midi_mut(region)?;
        let mut remove = Vec::new();
        let mut add = Vec::new();
        for mut chord in Self::chords(m, indices).into_iter().filter(|g| g.len() > 1) {
            if down {
                chord.reverse();
            }
            for (k, &i) in chord.iter().enumerate() {
                let mut n = m.notes[i];
                let shift = (spread * k as f64).min(n.length - 1.0 / 64.0).max(0.0);
                n.start += shift;
                n.length -= shift;
                remove.push(i);
                add.push(n);
            }
        }
        if add.is_empty() {
            return None;
        }
        Some(Self::merge_notes(m, remove, add, bar))
    }

    /// Turns chords into arpeggios: their notes one after another, `step`
    /// beats each, for as long as the chord lasted. `pattern` is "up",
    /// "down" or "updown". Returns the new notes' indices.
    pub fn arpeggiate_notes(&mut self, region: u64, indices: &[usize], step: f64, pattern: &str) -> Option<Vec<usize>> {
        if step <= 0.0 {
            return None;
        }
        let bar = self.bar_beats();
        let m = self.midi_mut(region)?;
        let mut remove = Vec::new();
        let mut add = Vec::new();
        for chord in Self::chords(m, indices).into_iter().filter(|g| g.len() > 1) {
            let notes: Vec<Note> = chord.iter().map(|&i| m.notes[i]).collect();
            let order: Vec<usize> = match pattern {
                "down" => (0..notes.len()).rev().collect(),
                "updown" => (0..notes.len()).chain((1..notes.len() - 1).rev()).collect(),
                _ => (0..notes.len()).collect(),
            };
            let start = notes[0].start;
            let span = notes.iter().map(|n| n.length).fold(0.0, f64::max);
            let count = ((span / step) - 1e-6).ceil().max(1.0) as usize;
            for k in 0..count {
                let n = notes[order[k % order.len()]];
                let at = start + step * k as f64;
                add.push(Note { start: at, length: step.min(start + span - at).max(1.0 / 64.0), ..n });
            }
            remove.extend(chord);
        }
        if add.is_empty() {
            return None;
        }
        Some(Self::merge_notes(m, remove, add, bar))
    }

    // ------------------------------------------------------------- markers

    /// Names for new markers, in the order a song usually goes.
    const MARKER_NAMES: [&'static str; 8] = ["Intro", "Verse", "Pre-Chorus", "Chorus", "Verse 2", "Bridge", "Chorus 2", "Outro"];

    /// Adds a marker at `beats` (snapped), named by how many there are unless
    /// `name` is given. Returns its id (an existing marker there is kept).
    pub fn add_marker(&mut self, beats: f64, name: Option<&str>) -> u64 {
        let beat = self.snap(beats);
        if let Some(m) = self.markers.iter().find(|m| (m.beat - beat).abs() < 1e-6) {
            return m.id;
        }
        let id = self.take_id();
        let count = self.markers.len();
        let name = match name {
            Some(n) if !n.trim().is_empty() => n.trim().to_owned(),
            _ => Self::MARKER_NAMES.get(count).map_or_else(|| format!("Marker {}", count + 1), |n| (*n).to_owned()),
        };
        self.markers.push(Marker { id, beat, name });
        self.markers.sort_by(|a, b| a.beat.total_cmp(&b.beat));
        id
    }

    pub fn move_marker(&mut self, id: u64, beats: f64) -> bool {
        let beat = self.snap(beats);
        let Some(m) = self.markers.iter_mut().find(|m| m.id == id) else { return false };
        m.beat = beat;
        self.markers.sort_by(|a, b| a.beat.total_cmp(&b.beat));
        true
    }

    pub fn rename_marker(&mut self, id: u64, name: &str) -> bool {
        let name = name.trim();
        let Some(m) = self.markers.iter_mut().find(|m| m.id == id).filter(|_| !name.is_empty()) else { return false };
        m.name = name.to_owned();
        true
    }

    pub fn remove_marker(&mut self, id: u64) -> bool {
        let before = self.markers.len();
        self.markers.retain(|m| m.id != id);
        self.markers.len() != before
    }

    /// The next marker after `beats`, if any.
    pub fn marker_after(&self, beats: f64) -> Option<f64> {
        self.markers.iter().map(|m| m.beat).find(|&b| b > beats + 1e-6)
    }

    /// The marker before `beats`, or the start of the song.
    pub fn marker_before(&self, beats: f64) -> f64 {
        self.markers.iter().rev().map(|m| m.beat).find(|&b| b < beats - 1e-6).unwrap_or(0.0)
    }

    /// The section a marker starts: from it to the next marker, or to the
    /// end of the last region (whole bars) for the last marker.
    pub fn marker_section(&self, id: u64) -> Option<(f64, f64)> {
        let m = self.markers.iter().find(|m| m.id == id)?;
        let bar = self.bar_beats();
        let end = self.marker_after(m.beat).unwrap_or_else(|| {
            let content = self.tracks.iter().flat_map(|t| &t.regions).map(|r| r.start + self.region_span(r)).fold(0.0, f64::max);
            ((content / bar).ceil() * bar).max(m.beat + bar)
        });
        Some((m.beat, end))
    }

    // ------------------------------------------------------------- buses

    /// Adds a bus at the end and returns its id.
    pub fn add_bus(&mut self, name: Option<&str>) -> u64 {
        let id = self.take_id();
        let number = self.buses.len() + 1;
        self.buses.push(Bus {
            id,
            name: name.map_or_else(|| format!("Bus {number}"), str::to_owned),
            color: "teal".into(),
            volume_db: 0.0,
            pan: 0.0,
            mute: false,
            inserts: Vec::new(),
        });
        id
    }

    pub fn bus(&self, id: u64) -> Option<&Bus> {
        self.buses.iter().find(|b| b.id == id)
    }

    pub fn bus_mut(&mut self, id: u64) -> Option<&mut Bus> {
        self.buses.iter_mut().find(|b| b.id == id)
    }

    /// Removes a bus and every send to it.
    pub fn remove_bus(&mut self, id: u64) -> bool {
        let before = self.buses.len();
        self.buses.retain(|b| b.id != id);
        let lane = format!("send:{id}");
        for t in &mut self.tracks {
            t.sends.retain(|s| s.bus != id);
            t.automation.retain(|l| l.target != lane);
        }
        self.buses.len() != before
    }

    /// Sets how much of a track goes to a bus, adding the send if needed.
    pub fn set_send(&mut self, track: u64, bus: u64, level_db: f32) -> bool {
        if self.bus(bus).is_none() {
            return false;
        }
        let level_db = level_db.clamp(Track::MIN_DB, Track::MAX_DB);
        let Some(t) = self.track_mut(track) else { return false };
        match t.sends.iter_mut().find(|s| s.bus == bus) {
            Some(s) => s.level_db = level_db,
            None => t.sends.push(Send { bus, level_db }),
        }
        true
    }

    /// A track's send level to a bus, or `Track::MIN_DB` when it has none.
    pub fn send_level(&self, track: &Track, bus: u64) -> f32 {
        track.sends.iter().find(|s| s.bus == bus).map_or(Track::MIN_DB, |s| s.level_db)
    }

    // ------------------------------------------------------------ effects

    pub fn chain(&self, chain: Chain) -> Option<&Vec<Insert>> {
        match chain {
            Chain::Track(id) => self.track(id).map(|t| &t.inserts),
            Chain::Bus(id) => self.bus(id).map(|b| &b.inserts),
            Chain::Master => Some(&self.master_inserts),
        }
    }

    fn chain_mut(&mut self, chain: Chain) -> Option<&mut Vec<Insert>> {
        match chain {
            Chain::Track(id) => self.track_mut(id).map(|t| &mut t.inserts),
            Chain::Bus(id) => self.bus_mut(id).map(|b| &mut b.inserts),
            Chain::Master => Some(&mut self.master_inserts),
        }
    }

    /// Every chain, with what it belongs to.
    fn chains_mut(&mut self) -> impl Iterator<Item = &mut Vec<Insert>> {
        self.tracks
            .iter_mut()
            .map(|t| &mut t.inserts)
            .chain(self.buses.iter_mut().map(|b| &mut b.inserts))
            .chain(std::iter::once(&mut self.master_inserts))
    }

    /// Adds an effect at the end of a chain with its default settings.
    /// Reverb and delay on a bus start fully wet, since the dry sound
    /// already reaches the master from the tracks.
    pub fn add_insert(&mut self, chain: Chain, kind: EffectKind) -> Option<u64> {
        let id = self.next_id;
        let mut params: BTreeMap<String, f32> = kind.params().iter().map(|d| (d.key.to_owned(), d.default)).collect();
        if matches!(chain, Chain::Bus(_)) && matches!(kind, EffectKind::Reverb | EffectKind::Delay) {
            params.insert("mix".into(), 100.0);
        }
        // Pitch correction starts in the song's key.
        if kind == EffectKind::PitchCorrect
            && let Some((root, minor)) = parse_key(&self.key)
        {
            params.insert("key".into(), f32::from(root));
            params.insert("scale".into(), if minor { 2.0 } else { 1.0 });
        }
        self.chain_mut(chain)?.push(Insert { id, kind: kind.key().into(), enabled: true, params, plugin: None });
        self.next_id += 1;
        Some(id)
    }

    /// Adds a plugin to the end of a chain.
    pub fn add_plugin_insert(&mut self, chain: Chain, plugin: PluginSlot) -> Option<u64> {
        let id = self.next_id;
        let insert = Insert { id, kind: EffectKind::Plugin.key().into(), enabled: true, params: BTreeMap::new(), plugin: Some(plugin) };
        self.chain_mut(chain)?.push(insert);
        self.next_id += 1;
        Some(id)
    }

    /// Adds a track playing a plugin instrument; returns the track id.
    pub fn add_plugin_track(&mut self, plugin: PluginSlot) -> u64 {
        let id = self.add_track(Some(&plugin.name));
        let instrument_id = self.take_id();
        if let Some(t) = self.track_mut(id) {
            t.instrument = Some(Instrument {
                id: instrument_id,
                kind: InstrumentKind::Plugin.key().into(),
                params: BTreeMap::new(),
                sample: None,
                slices: Vec::new(),
                plugin: Some(plugin),
            });
        }
        id
    }

    /// Every plugin in the song, by its insert or instrument id.
    pub fn plugins(&self) -> Vec<(u64, &PluginSlot)> {
        let chains = self.tracks.iter().map(|t| &t.inserts).chain(self.buses.iter().map(|b| &b.inserts)).chain(std::iter::once(&self.master_inserts));
        let mut out: Vec<(u64, &PluginSlot)> = chains.flatten().filter_map(|i| Some((i.id, i.plugin.as_ref()?))).collect();
        out.extend(self.tracks.iter().filter_map(|t| t.instrument.as_ref()).filter_map(|i| Some((i.id, i.plugin.as_ref()?))));
        out
    }

    /// Stores a plugin's state (from its insert or instrument id) for saving.
    pub fn set_plugin_state(&mut self, id: u64, state: String) -> bool {
        for chain in self.chains_mut() {
            if let Some(slot) = chain.iter_mut().find(|i| i.id == id).and_then(|i| i.plugin.as_mut()) {
                slot.state = Some(state);
                return true;
            }
        }
        if let Some(slot) = self.tracks.iter_mut().filter_map(|t| t.instrument.as_mut()).find(|i| i.id == id).and_then(|i| i.plugin.as_mut()) {
            slot.state = Some(state);
            return true;
        }
        false
    }

    pub fn insert_mut(&mut self, id: u64) -> Option<&mut Insert> {
        self.chains_mut().flat_map(|c| c.iter_mut()).find(|i| i.id == id)
    }

    pub fn remove_insert(&mut self, id: u64) -> bool {
        let prefix = format!("fx:{id}:");
        for t in &mut self.tracks {
            t.automation.retain(|l| !l.target.starts_with(&prefix));
        }
        self.chains_mut().any(|c| {
            let before = c.len();
            c.retain(|i| i.id != id);
            c.len() != before
        })
    }

    /// Sets one parameter, kept within its range. False for an unknown key.
    pub fn set_insert_param(&mut self, id: u64, key: &str, value: f32) -> bool {
        let Some(insert) = self.insert_mut(id) else { return false };
        let Some(def) = insert.effect_kind().and_then(|k| k.params().iter().find(|d| d.key == key)) else { return false };
        insert.params.insert(key.to_owned(), value.clamp(def.min, def.max));
        true
    }

    /// Moves an effect to `to` within its chain (0 = first).
    pub fn move_insert(&mut self, id: u64, to: usize) -> bool {
        for chain in self.chains_mut() {
            if let Some(from) = chain.iter().position(|i| i.id == id) {
                let to = to.min(chain.len() - 1);
                if from == to {
                    return false;
                }
                let insert = chain.remove(from);
                chain.insert(to, insert);
                return true;
            }
        }
        false
    }

    // ---------------------------------------------------------- automation

    /// The range and current (unautomated) value of an automation target on
    /// a track: (min, max, value). `None` if the target doesn't exist.
    pub fn target_range(&self, track: &Track, target: &str) -> Option<(f32, f32, f32)> {
        let mut parts = target.split(':');
        match (parts.next()?, parts.next(), parts.next()) {
            ("volume", None, _) => Some((Track::MIN_DB, Track::MAX_DB, track.volume_db)),
            ("pan", None, _) => Some((-1.0, 1.0, track.pan)),
            ("send", Some(bus), None) => {
                let bus: u64 = bus.parse().ok()?;
                self.bus(bus)?;
                Some((Track::MIN_DB, Track::MAX_DB, self.send_level(track, bus)))
            }
            ("fx", Some(insert), Some(key)) => {
                let insert: u64 = insert.parse().ok()?;
                let i = track.inserts.iter().find(|i| i.id == insert)?;
                let kind = i.effect_kind()?;
                let index = kind.params().iter().position(|d| d.key == key)?;
                let def = &kind.params()[index];
                Some((def.min, def.max, i.values()[index]))
            }
            _ => None,
        }
    }

    fn lane_mut(&mut self, track: u64, target: &str, create: bool) -> Option<&mut Lane> {
        let t = self.track_mut(track)?;
        let i = match t.automation.iter().position(|l| l.target == target) {
            Some(i) => i,
            None if create => {
                t.automation.push(Lane { target: target.to_owned(), points: Vec::new() });
                t.automation.len() - 1
            }
            None => return None,
        };
        Some(&mut t.automation[i])
    }

    /// Adds a point (or replaces one at the same beat). Returns its index.
    pub fn add_auto_point(&mut self, track: u64, target: &str, beat: f64, value: f32) -> Option<usize> {
        let (min, max, _) = self.target_range(self.track(track)?, target)?;
        let point = AutoPoint { beat: beat.max(0.0), value: value.clamp(min, max) };
        let lane = self.lane_mut(track, target, true)?;
        if let Some(i) = lane.points.iter().position(|p| (p.beat - point.beat).abs() < 1e-6) {
            lane.points[i] = point;
            return Some(i);
        }
        let i = lane.points.partition_point(|p| p.beat < point.beat);
        lane.points.insert(i, point);
        Some(i)
    }

    /// Moves a point; returns its new index (points stay sorted by beat).
    pub fn move_auto_point(&mut self, track: u64, target: &str, index: usize, beat: f64, value: f32) -> Option<usize> {
        let (min, max, _) = self.target_range(self.track(track)?, target)?;
        let lane = self.lane_mut(track, target, false)?;
        if index >= lane.points.len() {
            return None;
        }
        lane.points.remove(index);
        let point = AutoPoint { beat: beat.max(0.0), value: value.clamp(min, max) };
        let i = lane.points.partition_point(|p| p.beat < point.beat);
        lane.points.insert(i, point);
        Some(i)
    }

    pub fn remove_auto_point(&mut self, track: u64, target: &str, index: usize) -> bool {
        let Some(lane) = self.lane_mut(track, target, false) else { return false };
        if index >= lane.points.len() {
            return false;
        }
        lane.points.remove(index);
        true
    }

    /// Removes every point of a lane, so the parameter stops changing.
    pub fn clear_lane(&mut self, track: u64, target: &str) -> bool {
        let Some(t) = self.track_mut(track) else { return false };
        let before = t.automation.len();
        t.automation.retain(|l| l.target != target);
        t.automation.len() != before
    }

    // ------------------------------------------------------------ regions

    /// Places a whole audio file on a track. Returns the region id.
    pub fn add_region(&mut self, track: u64, file: &str, name: &str, start: f64, length_secs: f64) -> Option<u64> {
        let id = self.take_id();
        let region = Region {
            id,
            name: name.into(),
            file: file.into(),
            start: start.max(0.0),
            offset: 0.0,
            length: length_secs,
            fade_in: 0.0,
            fade_out: 0.0,
            source_length: length_secs,
            midi: None,
            gain_db: 0.0,
            reverse: false,
            loop_to: 0.0,
            tempo: None,
        };
        self.track_mut(track)?.regions.push(region);
        Some(id)
    }

    /// The track holding a region, and the region.
    pub fn find_region(&self, id: u64) -> Option<(&Track, &Region)> {
        self.tracks
            .iter()
            .find_map(|t| t.regions.iter().find(|r| r.id == id).map(|r| (t, r)))
    }

    /// Moves a region to a new start (snapped) and, optionally, another track.
    pub fn move_region(&mut self, id: u64, start: f64, to_track: Option<u64>) -> bool {
        let start = self.snap(start);
        let Some(from) = self.find_region(id).map(|(t, _)| t.id) else { return false };
        let to = to_track.filter(|t| self.track(*t).is_some()).unwrap_or(from);
        let Some(src) = self.track_mut(from) else { return false };
        let index = src.regions.iter().position(|r| r.id == id).expect("found above");
        let mut region = src.regions.remove(index);
        region.start = start;
        self.track_mut(to).expect("checked above").regions.push(region);
        true
    }

    fn region_mut(&mut self, id: u64) -> Option<&mut Region> {
        self.tracks.iter_mut().flat_map(|t| t.regions.iter_mut()).find(|r| r.id == id)
    }

    /// Shortest a MIDI region can be, in beats.
    pub const MIN_MIDI_BEATS: f64 = 0.25;

    /// Shortest a region can be trimmed to, in seconds.
    pub const MIN_REGION_SECS: f64 = 0.01;

    /// Moves a region's left or right edge to `beats` (snapped), keeping the
    /// audio where it is in time: like dragging the edge of a region in a DAW.
    pub fn trim_region(&mut self, id: u64, left: bool, beats: f64) -> bool {
        let beats = self.snap(beats);
        let Some(meter) = self.find_region(id).map(|(_, r)| self.region_meter(r)) else { return false };
        let Some(r) = self.region_mut(id) else { return false };
        if let Some(m) = &mut r.midi {
            // MIDI: the notes stay where they are; the region shows more or less of them.
            if left {
                let delta = (beats - r.start).max(-m.offset).max(-r.start).min(m.length - Self::MIN_MIDI_BEATS);
                r.start += delta;
                m.offset += delta;
                m.length -= delta;
                r.loop_to -= delta;
            } else {
                m.length = (beats - r.start).max(Self::MIN_MIDI_BEATS);
            }
            if r.loop_to <= m.length + 1e-6 {
                r.loop_to = 0.0;
            }
            return true;
        }
        if left {
            let mut delta = meter.beats_to_secs(beats - r.start);
            // Can't start before the beginning of the file, or past the end of the region.
            delta = delta.max(-r.offset).min(r.length - Self::MIN_REGION_SECS);
            r.offset += delta;
            r.length -= delta;
            r.start += meter.secs_to_beats(delta);
            r.loop_to -= meter.secs_to_beats(delta);
        } else {
            let mut length = meter.beats_to_secs(beats - r.start).max(Self::MIN_REGION_SECS);
            if r.source_length > 0.0 {
                length = length.min(r.source_length - r.offset);
            }
            r.length = length;
        }
        r.fade_in = r.fade_in.min(r.length);
        r.fade_out = r.fade_out.min(r.length - r.fade_in);
        if r.loop_to <= meter.secs_to_beats(r.length) + 1e-6 {
            r.loop_to = 0.0;
        }
        true
    }

    /// Splits a region in two at `beats`. Returns the new right-hand region's id.
    pub fn split_region(&mut self, id: u64, beats: f64) -> Option<u64> {
        let meter = self.find_region(id).map(|(_, r)| self.region_meter(r))?;
        let new_id = self.next_id;
        let track = self.tracks.iter_mut().find(|t| t.regions.iter().any(|r| r.id == id))?;
        let index = track.regions.iter().position(|r| r.id == id)?;
        let r = &mut track.regions[index];
        // Only the region's own length splits; the pieces stop looping.
        r.loop_to = 0.0;
        if let Some(length) = r.midi.as_ref().map(|m| m.length) {
            let cut = beats - r.start;
            if cut < Self::MIN_MIDI_BEATS || cut > length - Self::MIN_MIDI_BEATS {
                return None;
            }
            let mut right = r.clone();
            right.id = new_id;
            right.start = beats;
            if let Some(rm) = &mut right.midi {
                rm.offset += cut;
                rm.length -= cut;
            }
            if let Some(m) = &mut r.midi {
                m.length = cut;
            }
            track.regions.insert(index + 1, right);
            self.next_id += 1;
            return Some(new_id);
        }
        let cut = meter.beats_to_secs(beats - r.start);
        if cut < Self::MIN_REGION_SECS || cut > r.length - Self::MIN_REGION_SECS {
            return None;
        }
        let mut right = r.clone();
        right.id = new_id;
        right.start = beats;
        right.offset += cut;
        right.length -= cut;
        right.fade_in = 0.0;
        r.length = cut;
        r.fade_out = 0.0;
        r.fade_in = r.fade_in.min(cut);
        right.fade_out = right.fade_out.min(right.length);
        track.regions.insert(index + 1, right);
        self.next_id += 1;
        Some(new_id)
    }

    /// Sets fade lengths in seconds, kept within the region.
    pub fn set_fades(&mut self, id: u64, fade_in: f64, fade_out: f64) -> bool {
        let Some(r) = self.region_mut(id).filter(|r| r.midi.is_none()) else { return false };
        r.fade_in = fade_in.clamp(0.0, r.length);
        r.fade_out = fade_out.clamp(0.0, r.length - r.fade_in);
        true
    }

    /// Sets a region's volume change in dB (−48 to +24). Audio regions only.
    pub fn set_region_gain(&mut self, id: u64, db: f32) -> bool {
        let Some(r) = self.region_mut(id).filter(|r| r.midi.is_none()) else { return false };
        r.gain_db = db.clamp(-48.0, 24.0);
        true
    }

    /// Plays a region backwards (or forwards again). Audio regions only.
    pub fn set_region_reverse(&mut self, id: u64, on: bool) -> bool {
        let Some(r) = self.region_mut(id).filter(|r| r.midi.is_none()) else { return false };
        r.reverse = on;
        true
    }

    /// Repeats a region's contents until `end` beats on the timeline
    /// (snapped). An end at or before the region's own end stops the loop.
    pub fn set_region_loop(&mut self, id: u64, end: f64) -> bool {
        let end = self.snap(end);
        let Some((start, len)) = self.find_region(id).map(|(_, r)| (r.start, self.region_beats(r))) else { return false };
        let Some(r) = self.region_mut(id) else { return false };
        r.loop_to = if end - start > len + 1e-6 { end - start } else { 0.0 };
        true
    }

    /// Joins regions on one track into one. MIDI regions merge their notes
    /// (loops written out); audio regions join when they are neighbouring
    /// pieces of one recording, e.g. after a split. Returns the joined
    /// region's id (the earliest one's).
    pub fn join_regions(&mut self, ids: &[u64]) -> Option<u64> {
        let row = self.row_of(*ids.first()?)?;
        if ids.len() < 2 || ids.iter().any(|&id| self.row_of(id) != Some(row)) {
            return None;
        }
        let mut picked: Vec<Region> = self.tracks[row].regions.iter().filter(|r| ids.contains(&r.id)).cloned().collect();
        if picked.len() < 2 {
            return None;
        }
        picked.sort_by(|a, b| a.start.total_cmp(&b.start));
        let first = picked[0].clone();
        let joined = if first.midi.is_some() {
            let start = first.start;
            let end = picked.iter().map(|r| r.start + self.region_span(r)).fold(start, f64::max);
            let mut notes = Vec::new();
            for r in &picked {
                let m = r.midi.as_ref()?;
                let span = self.region_span(r);
                // Every pass of a loop, as plain notes.
                let mut pass = 0.0;
                while pass < span - 1e-9 {
                    for n in &m.notes {
                        let rel = n.start - m.offset;
                        if rel < 0.0 || rel >= m.length || pass + rel >= span {
                            continue;
                        }
                        let length = n.length.min(m.length - rel).min(span - pass - rel);
                        notes.push(Note { start: r.start - start + pass + rel, length, ..*n });
                    }
                    pass += m.length.max(Self::MIN_MIDI_BEATS);
                }
            }
            notes.sort_by(|a, b| (a.start, a.key).partial_cmp(&(b.start, b.key)).unwrap_or(std::cmp::Ordering::Equal));
            Region { midi: Some(MidiData { length: end - start, offset: 0.0, notes }), loop_to: 0.0, ..first }
        } else {
            for w in picked.windows(2) {
                let (a, b) = (&w[0], &w[1]);
                let a_end = a.start + self.region_meter(a).secs_to_beats(a.length);
                let neighbours = b.midi.is_none()
                    && a.file == b.file
                    && !a.reverse
                    && !b.reverse
                    && (a.gain_db - b.gain_db).abs() < 1e-3
                    && a.loop_to == 0.0
                    && b.loop_to == 0.0
                    && a.tempo == b.tempo
                    && (b.start - a_end).abs() < 1e-3
                    && (b.offset - (a.offset + a.length)).abs() < 1e-3;
                if !neighbours {
                    return None;
                }
            }
            let last = picked.last()?;
            Region { length: last.offset + last.length - first.offset, fade_out: last.fade_out, ..first }
        };
        let id = joined.id;
        self.tracks[row].regions.retain(|r| !ids.contains(&r.id));
        self.tracks[row].regions.push(joined);
        Some(id)
    }

    /// Moves a track to a new position in the list (0 = top).
    pub fn move_track(&mut self, id: u64, to: usize) -> bool {
        let Some(from) = self.tracks.iter().position(|t| t.id == id) else { return false };
        let to = to.min(self.tracks.len() - 1);
        if from == to {
            return false;
        }
        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);
        true
    }

    pub fn remove_region(&mut self, id: u64) -> bool {
        for track in &mut self.tracks {
            let before = track.regions.len();
            track.regions.retain(|r| r.id != id);
            if track.regions.len() != before {
                return true;
            }
        }
        false
    }

    // ------------------------------------------------ many regions at once

    /// Row (0 = top) of the track holding `region`.
    fn row_of(&self, region: u64) -> Option<usize> {
        self.tracks.iter().position(|t| t.regions.iter().any(|r| r.id == region))
    }

    /// Regions overlapping beats `from..to` on track rows `top..=bottom`
    /// (a marquee selection), in timeline order.
    pub fn regions_in(&self, from: f64, to: f64, top: usize, bottom: usize) -> Vec<u64> {
        let (from, to) = (from.min(to), from.max(to));
        let mut found: Vec<(f64, u64)> = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(row, _)| (top.min(bottom)..=top.max(bottom)).contains(row))
            .flat_map(|(_, t)| &t.regions)
            .filter(|r| r.start < to && r.start + self.region_span(r) > from)
            .map(|r| (r.start, r.id))
            .collect();
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        found.into_iter().map(|(_, id)| id).collect()
    }

    /// Every region id, in timeline order.
    pub fn all_regions(&self) -> Vec<u64> {
        self.regions_in(0.0, f64::MAX, 0, self.tracks.len().saturating_sub(1))
    }

    /// Copies regions, keeping their places relative to each other.
    pub fn copy_regions(&self, ids: &[u64]) -> Option<Clipboard> {
        let picked: Vec<(usize, &Region)> = ids
            .iter()
            .filter_map(|&id| Some((self.row_of(id)?, self.find_region(id)?.1)))
            .collect();
        let top = picked.iter().map(|(row, _)| *row).min()?;
        let first = picked.iter().map(|(_, r)| r.start).fold(f64::MAX, f64::min);
        let last = picked.iter().map(|(_, r)| r.start + self.region_span(r)).fold(0.0, f64::max);
        let items = picked
            .into_iter()
            .map(|(row, r)| {
                let mut r = r.clone();
                r.start -= first;
                (row - top, r)
            })
            .collect();
        Some(Clipboard { items, span: last - first })
    }

    /// Places copied regions with the earliest at `at` beats and the topmost on
    /// track `row`, adding tracks if they run off the bottom. Returns the new ids.
    pub fn paste(&mut self, clip: &Clipboard, at: f64, row: usize) -> Vec<u64> {
        let mut ids = Vec::with_capacity(clip.items.len());
        for (offset, region) in &clip.items {
            let target = row + offset;
            while self.tracks.len() <= target {
                self.add_track(None);
            }
            let mut region = region.clone();
            region.id = self.take_id();
            region.start = (region.start + at).max(0.0);
            ids.push(region.id);
            self.tracks[target].regions.push(region);
        }
        ids
    }

    /// Copies regions to right after the end of the group (⌘D), rounded up to
    /// the snap grid so repeats stay in time. Returns the new ids.
    pub fn duplicate_regions(&mut self, ids: &[u64]) -> Vec<u64> {
        let Some(clip) = self.copy_regions(ids) else { return Vec::new() };
        let top = ids.iter().filter_map(|&id| self.row_of(id)).min().unwrap_or(0);
        let first = ids.iter().filter_map(|&id| self.find_region(id)).map(|(_, r)| r.start).fold(f64::MAX, f64::min);
        let mut span = clip.span;
        if self.snap {
            let step = self.snap_value.step(&self.meter());
            span = ((span / step) - 1e-6).ceil().max(1.0) * step;
        }
        self.paste(&clip, first + span, top)
    }

    /// Moves regions together by `beats` and `rows` tracks, keeping their
    /// spacing. Stops at the start of the song and the first/last track.
    pub fn move_regions(&mut self, ids: &[u64], beats: f64, rows: i64) -> bool {
        let found: Vec<(usize, f64)> = ids
            .iter()
            .filter_map(|&id| Some((self.row_of(id)?, self.find_region(id)?.1.start)))
            .collect();
        if found.is_empty() {
            return false;
        }
        let earliest = found.iter().map(|f| f.1).fold(f64::MAX, f64::min);
        let beats = beats.max(-earliest);
        let (top, bottom) = found.iter().fold((usize::MAX, 0), |(t, b), f| (t.min(f.0), b.max(f.0)));
        let last = self.tracks.len() as i64 - 1;
        let rows = rows.clamp(-(top as i64), last - bottom as i64);
        if beats == 0.0 && rows == 0 {
            return false;
        }
        for &id in ids {
            let Some(row) = self.row_of(id) else { continue };
            let index = self.tracks[row].regions.iter().position(|r| r.id == id).expect("found above");
            let mut region = self.tracks[row].regions.remove(index);
            region.start += beats;
            self.tracks[(row as i64 + rows) as usize].regions.push(region);
        }
        true
    }

    /// Deletes regions; true if any existed.
    pub fn remove_regions(&mut self, ids: &[u64]) -> bool {
        // Not `any`: that would stop after the first one it removes.
        let mut removed = false;
        for &id in ids {
            removed |= self.remove_region(id);
        }
        removed
    }

    /// Every audio file the project uses, without duplicates.
    pub fn audio_files(&self) -> Vec<String> {
        let mut files: Vec<String> =
            self.tracks.iter().flat_map(|t| &t.regions).filter(|r| r.midi.is_none()).map(|r| r.file.clone()).collect();
        files.extend(self.tracks.iter().filter_map(|t| t.instrument.as_ref()?.sample.clone()));
        files.extend(self.tracks.iter().filter_map(|t| t.frozen.clone()));
        files.sort();
        files.dedup();
        files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        Project::new("Test", Meter::default(), "C major")
    }

    #[test]
    fn new_projects_have_a_name_and_a_four_bar_cycle() {
        let p = Project::new("  ", Meter::default(), "C major");
        assert_eq!(p.name, "Untitled");
        assert_eq!((p.cycle_start, p.cycle_end), (0.0, 16.0));
    }

    #[test]
    fn snapping_follows_the_grid() {
        let mut p = project();
        assert_eq!(p.snap(5.4), 5.0);
        p.snap_value = Snap::Bar;
        assert_eq!(p.snap(5.4), 4.0);
        assert_eq!(p.snap(6.1), 8.0);
        p.snap = false;
        assert_eq!(p.snap(5.4), 5.4);
    }

    #[test]
    fn cycle_is_ordered_snapped_and_never_empty() {
        let mut p = project();
        p.set_cycle(9.2, 3.9);
        assert_eq!((p.cycle_start, p.cycle_end), (4.0, 9.0));
        p.set_cycle(2.0, 2.1);
        assert_eq!((p.cycle_start, p.cycle_end), (2.0, 3.0));
    }

    #[test]
    fn timeline_length_leaves_room_after_the_last_region() {
        let mut p = project();
        assert_eq!(p.length_beats(), 128.0);
        let t = p.add_track(None);
        // 75 s at 120 BPM is 150 beats → 38 bars rounded up, plus 4 bars of room.
        p.add_region(t, "a.wav", "a", 0.0, 75.0);
        assert_eq!(p.length_beats(), 168.0);
    }

    #[test]
    fn tracks_get_names_colours_and_unique_ids() {
        let mut p = project();
        let a = p.add_track(None);
        let b = p.add_track(Some("Vocals"));
        assert_ne!(a, b);
        assert_eq!(p.track(a).unwrap().name, "Audio 1");
        assert_eq!(p.track(b).unwrap().name, "Vocals");
        assert_ne!(p.track(a).unwrap().color, p.track(b).unwrap().color);
    }

    #[test]
    fn regions_move_between_tracks_and_snap() {
        let mut p = project();
        let a = p.add_track(None);
        let b = p.add_track(None);
        let r = p.add_region(a, "a.wav", "a", 0.0, 2.0).unwrap();
        assert!(p.move_region(r, 4.3, Some(b)));
        let (track, region) = p.find_region(r).unwrap();
        assert_eq!((track.id, region.start), (b, 4.0));
        assert!(p.track(a).unwrap().regions.is_empty());
        assert!(p.remove_region(r));
        assert!(p.find_region(r).is_none());
    }

    #[test]
    fn trimming_keeps_audio_in_place() {
        let mut p = project(); // 120 BPM: 1 beat = 0.5 s
        let t = p.add_track(None);
        let r = p.add_region(t, "a.wav", "a", 4.0, 4.0).unwrap(); // beats 4..12
        assert!(p.trim_region(r, true, 6.0));
        let region = p.find_region(r).unwrap().1.clone();
        assert_eq!((region.start, region.offset, region.length), (6.0, 1.0, 3.0));
        // The left edge can't go before the start of the file.
        p.trim_region(r, true, 0.0);
        let region = p.find_region(r).unwrap().1.clone();
        assert_eq!((region.start, region.offset, region.length), (4.0, 0.0, 4.0));
        // Nor the right edge past its end.
        p.trim_region(r, false, 40.0);
        assert_eq!(p.find_region(r).unwrap().1.length, 4.0);
        p.trim_region(r, false, 8.0);
        assert_eq!(p.find_region(r).unwrap().1.length, 2.0);
    }

    #[test]
    fn splitting_makes_two_contiguous_regions() {
        let mut p = project();
        let t = p.add_track(None);
        let r = p.add_region(t, "a.wav", "a", 0.0, 4.0).unwrap();
        p.set_fades(r, 0.5, 0.5);
        let right = p.split_region(r, 2.0).unwrap();
        let (a, b) = (p.find_region(r).unwrap().1.clone(), p.find_region(right).unwrap().1.clone());
        assert_eq!((a.start, a.offset, a.length, a.fade_in, a.fade_out), (0.0, 0.0, 1.0, 0.5, 0.0));
        assert_eq!((b.start, b.offset, b.length, b.fade_in, b.fade_out), (2.0, 1.0, 3.0, 0.0, 0.5));
        assert_eq!(p.split_region(r, 0.0), None, "nothing to split at the very edge");
    }

    #[test]
    fn fades_stay_inside_the_region() {
        let mut p = project();
        let t = p.add_track(None);
        let r = p.add_region(t, "a.wav", "a", 0.0, 2.0).unwrap();
        p.set_fades(r, 1.5, 1.5);
        let region = p.find_region(r).unwrap().1;
        assert_eq!((region.fade_in, region.fade_out), (1.5, 0.5));
    }

    #[test]
    fn tracks_reorder() {
        let mut p = project();
        let a = p.add_track(None);
        let b = p.add_track(None);
        let c = p.add_track(None);
        assert!(p.move_track(c, 0));
        assert_eq!(p.tracks.iter().map(|t| t.id).collect::<Vec<_>>(), vec![c, a, b]);
        assert!(!p.move_track(c, 0), "already there");
    }

    #[test]
    fn only_one_track_is_armed() {
        let mut p = project();
        let a = p.add_track(None);
        let b = p.add_track(None);
        p.set_armed(a, true);
        p.track_mut(a).unwrap().monitor = true;
        p.set_armed(b, true);
        assert!(!p.track(a).unwrap().armed && !p.track(a).unwrap().monitor);
        assert_eq!(p.armed_track().map(|t| t.id), Some(b));
        p.set_armed(b, false);
        assert!(p.armed_track().is_none());
    }

    #[test]
    fn solo_silences_other_tracks() {
        let mut p = project();
        let a = p.add_track(None);
        let b = p.add_track(None);
        assert!(p.audible(p.track(a).unwrap()));
        p.track_mut(b).unwrap().solo = true;
        assert!(!p.audible(p.track(a).unwrap()));
        assert!(p.audible(p.track(b).unwrap()));
        p.track_mut(b).unwrap().mute = true;
        assert!(!p.audible(p.track(b).unwrap()), "mute wins over solo");
    }

    #[test]
    fn fader_gain() {
        let mut p = project();
        let a = p.add_track(None);
        let t = p.track_mut(a).unwrap();
        assert_eq!(t.gain(), 1.0);
        t.volume_db = Track::MIN_DB;
        assert_eq!(t.gain(), 0.0);
        t.volume_db = -6.0;
        assert!((t.gain() - 0.501).abs() < 0.001);
    }

    /// Two tracks: regions 1 s (2 beats) long at beats 0 and 4 on the first,
    /// and at beat 2 on the second.
    fn three_regions() -> (Project, [u64; 3]) {
        let mut p = project();
        let a = p.add_track(None);
        let b = p.add_track(None);
        let r1 = p.add_region(a, "a.wav", "a", 0.0, 1.0).unwrap();
        let r2 = p.add_region(a, "a.wav", "a", 4.0, 1.0).unwrap();
        let r3 = p.add_region(b, "b.wav", "b", 2.0, 1.0).unwrap();
        (p, [r1, r2, r3])
    }

    #[test]
    fn marquee_finds_overlapping_regions_on_the_rows() {
        let (p, [r1, r2, r3]) = three_regions();
        assert_eq!(p.regions_in(1.0, 3.0, 0, 1), vec![r1, r3]);
        assert_eq!(p.regions_in(1.0, 5.0, 0, 0), vec![r1, r2]);
        assert_eq!(p.regions_in(2.0, 4.0, 0, 0), Vec::<u64>::new(), "touching edges don't count");
        assert_eq!(p.all_regions(), vec![r1, r3, r2]);
    }

    #[test]
    fn paste_keeps_layout_and_adds_tracks_when_needed() {
        let (mut p, [r1, _, r3]) = three_regions();
        let clip = p.copy_regions(&[r3, r1]).unwrap();
        assert_eq!(clip.span, 4.0);
        let new = p.paste(&clip, 8.0, 1);
        assert_eq!(new.len(), 2);
        assert_eq!(p.tracks.len(), 3, "the lower region needed a new track");
        let (t, r) = p.find_region(new[1]).unwrap();
        assert_eq!((t.id, r.start), (p.tracks[1].id, 8.0));
        let (t, r) = p.find_region(new[0]).unwrap();
        assert_eq!((t.id, r.start), (p.tracks[2].id, 10.0));
    }

    #[test]
    fn duplicate_lands_after_the_group_on_the_grid() {
        let (mut p, [r1, r2, _]) = three_regions();
        p.snap_value = Snap::Bar;
        let new = p.duplicate_regions(&[r1, r2]);
        let starts: Vec<f64> = new.iter().map(|&id| p.find_region(id).unwrap().1.start).collect();
        assert_eq!(starts, vec![8.0, 12.0], "6 beats rounds up to 2 bars");
        p.snap = false;
        let new = p.duplicate_regions(&[r1]);
        assert_eq!(p.find_region(new[0]).unwrap().1.start, 2.0);
    }

    #[test]
    fn moving_many_keeps_spacing_and_stays_in_bounds() {
        let (mut p, [r1, r2, r3]) = three_regions();
        assert!(p.move_regions(&[r2, r3], -10.0, 5));
        // Clamped: r3 (beat 2) can only go back 2 beats, and is on the last track already.
        assert_eq!(p.find_region(r3).unwrap().1.start, 0.0);
        assert_eq!(p.find_region(r2).unwrap().1.start, 2.0);
        assert_eq!(p.find_region(r2).unwrap().0.id, p.tracks[0].id);
        assert!(p.move_regions(&[r1, r2], 1.0, 1));
        assert_eq!(p.find_region(r1).unwrap().0.id, p.tracks[1].id);
        assert!(!p.move_regions(&[r1], 0.0, 0));
        assert!(p.remove_regions(&[r1, r2, 999]));
        assert_eq!(p.all_regions(), vec![r3]);
    }

    #[test]
    fn sends_go_to_buses_and_vanish_with_them() {
        let mut p = project();
        let t = p.add_track(None);
        let b = p.add_bus(None);
        assert_eq!(p.bus(b).unwrap().name, "Bus 1");
        assert!(!p.set_send(t, 999, 0.0), "no such bus");
        assert!(p.set_send(t, b, -6.0));
        assert!(p.set_send(t, b, -3.0));
        assert_eq!(p.track(t).unwrap().sends, vec![Send { bus: b, level_db: -3.0 }]);
        assert_eq!(p.send_level(p.track(t).unwrap(), b), -3.0);
        assert!(p.remove_bus(b));
        assert!(p.track(t).unwrap().sends.is_empty());
        assert_eq!(db_to_gain(-60.0), 0.0);
        assert!((db_to_gain(-6.0) - 0.501).abs() < 0.001);
    }

    #[test]
    fn effects_live_in_chains_with_clamped_settings() {
        let mut p = project();
        let t = p.add_track(None);
        let b = p.add_bus(None);
        let eq = p.add_insert(Chain::Track(t), EffectKind::Eq).unwrap();
        let comp = p.add_insert(Chain::Track(t), EffectKind::Compressor).unwrap();
        let verb = p.add_insert(Chain::Bus(b), EffectKind::Reverb).unwrap();
        let lim = p.add_insert(Chain::Master, EffectKind::Limiter).unwrap();
        assert!(p.add_insert(Chain::Track(999), EffectKind::Eq).is_none());
        assert_eq!(p.insert_mut(verb).unwrap().params["mix"], 100.0, "bus reverbs start fully wet");

        assert!(p.set_insert_param(eq, "b1_gain", 99.0));
        assert_eq!(p.insert_mut(eq).unwrap().values()[9], 24.0, "clamped to the range");
        assert!(!p.set_insert_param(eq, "nope", 1.0));

        assert!(p.move_insert(comp, 0));
        let ids: Vec<u64> = p.chain(Chain::Track(t)).unwrap().iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![comp, eq]);

        assert!(p.remove_insert(lim) && p.master_inserts.is_empty());
        // Old projects (no "inserts") still load.
        let mut old = p.insert_mut(eq).unwrap().clone();
        old.params.clear();
        assert_eq!(old.values(), EffectKind::Eq.defaults());
    }

    #[test]
    fn automation_points_stay_sorted_clamped_and_tidy() {
        let mut p = project();
        let t = p.add_track(None);
        assert_eq!(p.add_auto_point(t, "volume", 4.0, -6.0), Some(0));
        assert_eq!(p.add_auto_point(t, "volume", 0.0, 99.0), Some(0), "goes first");
        assert_eq!(p.track(t).unwrap().automation[0].points[0].value, Track::MAX_DB, "clamped");
        assert_eq!(p.add_auto_point(t, "volume", 4.0, -3.0), Some(1), "same beat replaces");
        assert_eq!(p.track(t).unwrap().automation[0].points.len(), 2);
        assert_eq!(p.move_auto_point(t, "volume", 0, 8.0, 0.0), Some(1), "moving past re-sorts");
        assert!(p.remove_auto_point(t, "volume", 0));
        assert!(p.add_auto_point(t, "wobble", 0.0, 0.0).is_none(), "unknown target");

        // Lanes for a bus send and an effect knob go away with the bus or effect.
        let b = p.add_bus(None);
        let fx = p.add_insert(Chain::Track(t), EffectKind::Eq).unwrap();
        assert!(p.add_auto_point(t, &format!("send:{b}"), 0.0, -10.0).is_some());
        assert!(p.add_auto_point(t, &format!("fx:{fx}:b1_gain"), 0.0, 4.0).is_some());
        assert_eq!(p.track(t).unwrap().automation.len(), 3);
        p.remove_bus(b);
        p.remove_insert(fx);
        assert_eq!(p.track(t).unwrap().automation.len(), 1);
        assert!(p.clear_lane(t, "volume"));
        assert!(p.track(t).unwrap().automation.is_empty());
    }

    #[test]
    fn midi_regions_hold_notes_and_edit_like_audio_regions() {
        let mut p = project();
        let audio = p.add_track(None);
        assert!(p.add_midi_region(audio, 0.0, 4.0).is_none(), "only on instrument tracks");
        let t = p.add_instrument_track(InstrumentKind::Synth);
        assert_eq!(p.track(t).unwrap().name, "Isai Synth 1");
        let r = p.add_midi_region(t, 4.0, 4.0).unwrap();
        assert_eq!(p.add_note(r, 1.0, 1.0, 64, 100), Some(0));
        assert_eq!(p.add_note(r, 0.0, 0.5, 60, 100), Some(0), "sorted by start");
        // A note past the end grows the region to the next bar.
        p.add_note(r, 5.0, 1.0, 67, 90);
        assert_eq!(p.region_beats(p.find_region(r).unwrap().1), 8.0);
        assert_eq!(p.update_note(r, 0, 2.0, 1.0, 62, 80), Some(1));
        assert!(p.remove_note(r, 2));

        // Trimming the left edge keeps the notes where they are in time.
        assert!(p.trim_region(r, true, 5.0));
        let (_, region) = p.find_region(r).unwrap();
        let m = region.midi.as_ref().unwrap();
        assert_eq!((region.start, m.offset, m.length), (5.0, 1.0, 7.0));
        // Splitting shares the notes between both halves.
        let right = p.split_region(r, 7.0).unwrap();
        let rm = p.find_region(right).unwrap().1.midi.clone().unwrap();
        assert_eq!((rm.offset, rm.length), (3.0, 5.0));
        assert!(p.audio_files().is_empty(), "MIDI has no audio files");
        assert!(!p.set_fades(r, 1.0, 1.0));
        assert!(p.set_instrument_param(t, "cutoff", 99999.0));
        assert_eq!(p.track(t).unwrap().instrument.as_ref().unwrap().values()[2], 18000.0);
    }

    #[test]
    fn notes_move_and_delete_as_a_group() {
        let mut p = project();
        let t = p.add_instrument_track(InstrumentKind::Synth);
        let r = p.add_midi_region(t, 0.0, 4.0).unwrap();
        p.add_note(r, 0.0, 1.0, 60, 100);
        p.add_note(r, 1.0, 1.0, 127, 100);
        p.add_note(r, 2.0, 1.0, 64, 100);
        let notes = |p: &Project| p.find_region(r).unwrap().1.midi.as_ref().unwrap().notes.clone();
        // Notes 0 and 1 move right two beats and up: the top note limits the transpose.
        let moved = p.transform_notes(r, &[0, 1], 2.5, 5, 0.0).unwrap();
        let n = notes(&p);
        assert_eq!(moved, vec![1, 2], "followed through the re-sort");
        assert_eq!((n[1].start, n[1].key, n[2].key), (2.5, 60, 127), "keys unchanged at the top");
        // Moving left stops at the region start; lengths change together.
        let moved = p.transform_notes(r, &moved, -10.0, 0, 0.5).unwrap();
        let n = notes(&p);
        assert_eq!(n.iter().map(|n| n.start).collect::<Vec<_>>(), vec![0.0, 1.0, 2.0]);
        assert!(moved.iter().all(|&i| n[i].length == 1.5));
        assert!(p.transform_notes(r, &[9], 1.0, 0, 0.0).is_none());
        assert!(p.remove_notes(r, &[0, 2]));
        assert_eq!(notes(&p).len(), 1);
    }

    #[test]
    fn quantize_moves_notes_to_the_grid() {
        let mut p = project();
        let t = p.add_instrument_track(InstrumentKind::Synth);
        let r = p.add_midi_region(t, 4.0, 4.0).unwrap();
        p.add_note(r, 0.1, 0.3, 60, 100);
        p.add_note(r, 1.2, 0.5, 62, 100);
        p.add_note(r, 2.9, 0.5, 64, 100);
        let notes = |p: &Project| p.find_region(r).unwrap().1.midi.as_ref().unwrap().notes.clone();
        // Half strength, only the second note.
        assert!(p.quantize_notes(r, &[1], 0.5, 0.5, false, 0.0));
        assert!((notes(&p)[1].start - 1.1).abs() < 1e-9);
        // Full strength on everything, ends too.
        assert!(p.quantize_notes(r, &[], 0.5, 1.0, true, 0.0));
        let n = notes(&p);
        assert_eq!(n.iter().map(|n| n.start).collect::<Vec<_>>(), vec![0.0, 1.0, 3.0]);
        assert_eq!(n[0].length, 0.5, "a short note keeps a grid step");
        assert_eq!(n[1].length, 0.5);
        // The grid is the song's, even when the region's notes are offset.
        assert!(p.trim_region(r, true, 4.25));
        assert!(p.quantize_notes(r, &[], 1.0, 1.0, false, 0.0));
        assert_eq!(notes(&p)[1].start, 1.0, "already on a song beat");
    }

    #[test]
    fn recorded_notes_become_a_bar_aligned_region() {
        let mut p = project();
        let t = p.add_instrument_track(InstrumentKind::Synth);
        let rec = [Note { start: 5.5, length: 1.0, key: 60, velocity: 90 }, Note { start: 9.0, length: 0.5, key: 64, velocity: 0 }];
        let id = p.add_recorded_notes(t, &rec).unwrap();
        let region = p.find_region(id).unwrap().1;
        let m = region.midi.as_ref().unwrap();
        assert_eq!((region.start, m.length), (4.0, 8.0));
        assert_eq!(m.notes[0].start, 1.5);
        assert_eq!(m.notes[1].velocity, 1);
        assert!(p.add_recorded_notes(t, &[]).is_none());
    }

    #[test]
    fn keys_parse_and_pitch_correction_follows_the_song() {
        assert_eq!(parse_key("C major"), Some((0, false)));
        assert_eq!(parse_key("F# minor"), Some((6, true)));
        assert_eq!(parse_key("B♭ major"), Some((10, false)));
        assert_eq!(parse_key("nope"), None);
        let mut p = project();
        p.key = "A minor".into();
        let t = p.add_track(None);
        let id = p.add_insert(Chain::Track(t), EffectKind::PitchCorrect).unwrap();
        let i = p.insert_mut(id).unwrap();
        assert_eq!((i.params["key"], i.params["scale"]), (9.0, 2.0));
    }

    #[test]
    fn following_regions_keep_their_beats_when_the_tempo_changes() {
        let mut p = project(); // 120 BPM: a second is two beats
        let t = p.add_track(None);
        let r = p.add_region(t, "loop.wav", "loop", 0.0, 2.0).unwrap();
        let beats = |p: &Project| p.region_beats(p.find_region(r).unwrap().1);
        assert_eq!(beats(&p), 4.0);
        p.tempo = 60.0;
        assert_eq!(beats(&p), 2.0, "not following: the audio keeps its seconds");
        p.tempo = 120.0;
        assert!(p.set_region_tempo(r, Some(120.0)));
        p.tempo = 60.0;
        assert_eq!(beats(&p), 4.0, "following: it keeps its beats");
        assert_eq!(p.stretch_ratio(p.find_region(r).unwrap().1), 2.0, "twice as long at half the tempo");
        // Splitting a following region cuts in its own beats.
        let right = p.split_region(r, 2.0).unwrap();
        assert_eq!(p.find_region(right).unwrap().1.offset, 1.0, "one second into the audio");
        assert!(p.set_region_tempo(r, None));
    }

    #[test]
    fn tempo_changes_are_added_moved_and_removed() {
        let mut p = Project::default();
        assert!(p.add_tempo_change(16.0, 90.0));
        assert!(p.add_tempo_change(8.0, 100.04));
        assert_eq!(p.tempo_changes, vec![TempoChange { beat: 8.0, bpm: 100.0 }, TempoChange { beat: 16.0, bpm: 90.0 }]);
        // The tempo display changes the section the playhead is in.
        assert!(p.set_tempo_at(10.0, 110.0));
        assert_eq!(p.tempo_changes[0].bpm, 110.0);
        assert!(p.set_tempo_at(2.0, 130.0));
        assert_eq!(p.tempo, 130.0);
        assert!(p.move_tempo_change(16.0, 20.0, 80.0));
        assert_eq!(p.tempo_changes[1], TempoChange { beat: 20.0, bpm: 80.0 });
        assert_eq!(p.tempo_map().bpm_at(19.0), 110.0);
        assert!(p.remove_tempo_change(8.0));
        assert_eq!(p.tempo_map().bpm_at(10.0), 130.0);
        // Saved and loaded with the project; old projects without them still load.
        let text = ron::ser::to_string(&p).unwrap();
        let back: Project = ron::from_str(&text).unwrap();
        assert_eq!(back.tempo_changes, p.tempo_changes);
        let old: Project = ron::from_str("(name: \"Old\", tempo: 100.0)").unwrap();
        assert!(old.tempo_changes.is_empty() && old.signature_changes.is_empty());
    }

    #[test]
    fn audio_keeps_its_speed_through_a_tempo_change() {
        // 24 s of audio from the start: 8 s at 120 (16 beats), then 16 s at 90 (24 beats).
        let mut p = Project::default();
        let track = p.add_track(Some("Loop"));
        let id = p.add_region(track, "loop.wav", "Loop", 0.0, 24.0).unwrap();
        let r = p.find_region(id).unwrap().1.clone();
        assert_eq!(p.region_beats(&r), 48.0);
        p.add_tempo_change(16.0, 90.0);
        assert!((p.region_beats(&r) - 40.0).abs() < 1e-9);
    }

    #[test]
    fn signature_changes_move_the_bar_lines() {
        let mut p = Project::default();
        assert!(p.add_signature_change(3, 3, 4));
        assert!(p.add_signature_change(1, 4, 4), "bar 1 sets the opening signature");
        let map = p.tempo_map();
        assert_eq!(map.bar_start(4), 11.0);
        assert_eq!(p.meter_at(12.0).numerator, 3);
        // Snapping to bars follows the shorter bars.
        p.snap = true;
        p.snap_value = Snap::Bar;
        assert_eq!(p.snap(10.4), 11.0);
        assert_eq!(p.snap(9.0), 8.0);
        assert!(p.set_signature_at(12.0, 6, 8));
        assert_eq!(p.signature_changes, vec![SignatureChange { bar: 3, numerator: 6, denominator: 8 }]);
        assert!(p.remove_signature_change(3));
        assert!(p.signature_changes.is_empty());
    }

    #[test]
    fn cycle_wraps_the_chosen_regions() {
        let (mut p, [r1, r2, r3]) = three_regions();
        assert!(p.cycle_regions(&[r2, r3]));
        assert_eq!((p.cycle, p.cycle_start, p.cycle_end), (true, 2.0, 6.0), "from the earliest start to the latest end");
        p.set_region_loop(r1, 10.0);
        p.cycle_regions(&[r1]);
        assert_eq!((p.cycle_start, p.cycle_end), (0.0, 10.0), "a loop counts");
        assert!(!p.cycle_regions(&[999]));
    }

    #[test]
    fn swing_pushes_every_second_grid_step_late() {
        let mut p = project();
        let t = p.add_instrument_track(InstrumentKind::Synth);
        let r = p.add_midi_region(t, 0.0, 4.0).unwrap();
        for i in 0..4 {
            p.add_note(r, f64::from(i) * 0.5, 0.25, 60, 100);
        }
        assert!(p.quantize_notes(r, &[], 0.5, 1.0, false, 1.0 / 3.0));
        let starts: Vec<f64> = p.find_region(r).unwrap().1.midi.as_ref().unwrap().notes.iter().map(|n| n.start).collect();
        let want = [0.0, 0.5 + 0.5 / 3.0, 1.0, 1.5 + 0.5 / 3.0];
        assert!(starts.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-9), "{starts:?}");
    }

    #[test]
    fn chords_strum_and_arpeggiate() {
        let mut p = project();
        let t = p.add_instrument_track(InstrumentKind::Synth);
        let r = p.add_midi_region(t, 0.0, 4.0).unwrap();
        let chord = [60, 64, 67].map(|key| Note { start: 0.0, length: 2.0, key, velocity: 100 });
        assert_eq!(p.add_notes(r, &chord).unwrap(), vec![0, 1, 2]);
        let notes = |p: &Project| p.find_region(r).unwrap().1.midi.as_ref().unwrap().notes.clone();

        // Strummed up: each higher note a little later, all ending together.
        assert!(p.strum_notes(r, &[], 0.125, false).is_some());
        let n = notes(&p);
        assert_eq!(n.iter().map(|n| (n.start, n.key)).collect::<Vec<_>>(), vec![(0.0, 60), (0.125, 64), (0.25, 67)]);
        assert!(n.iter().all(|n| (n.start + n.length - 2.0).abs() < 1e-9));
        assert!(p.strum_notes(r, &[0], 0.1, false).is_none(), "a single note isn't a chord");

        // Arpeggiate a fresh chord up-down in eighths over its two beats.
        p.remove_notes(r, &[0, 1, 2]);
        p.add_notes(r, &chord);
        let new = p.arpeggiate_notes(r, &[], 0.5, "updown").unwrap();
        let n = notes(&p);
        assert_eq!(new.len(), 4);
        assert_eq!(n.iter().map(|n| n.key).collect::<Vec<_>>(), vec![60, 64, 67, 64]);
        assert_eq!(n.iter().map(|n| n.start).collect::<Vec<_>>(), vec![0.0, 0.5, 1.0, 1.5]);
        assert!(p.arpeggiate_notes(r, &[], 0.0, "up").is_none());
    }

    #[test]
    fn markers_name_themselves_and_mark_sections() {
        let mut p = project();
        let intro = p.add_marker(0.0, None);
        let verse = p.add_marker(16.2, None);
        assert_eq!(p.markers.iter().map(|m| (m.beat, m.name.as_str())).collect::<Vec<_>>(), vec![(0.0, "Intro"), (16.0, "Verse")]);
        assert_eq!(p.add_marker(16.0, Some("Again")), verse, "one marker per place");
        assert!(p.move_marker(intro, 32.0));
        assert_eq!(p.markers[0].id, verse, "kept in order");
        assert!(p.rename_marker(verse, " Hook "));
        assert!(!p.rename_marker(verse, "  "));
        assert_eq!(p.marker_after(16.0), Some(32.0));
        assert_eq!(p.marker_before(16.0), 0.0);
        assert_eq!(p.marker_section(verse), Some((16.0, 32.0)));
        assert_eq!(p.marker_section(intro), Some((32.0, 36.0)), "the last one runs a bar");
        assert!(p.remove_marker(intro) && !p.remove_marker(intro));
    }

    #[test]
    fn regions_loop_reverse_and_change_gain() {
        let mut p = project();
        let t = p.add_track(None);
        let r = p.add_region(t, "a.wav", "a", 0.0, 2.0).unwrap(); // 4 beats
        assert!(p.set_region_loop(r, 13.0));
        let region = p.find_region(r).unwrap().1.clone();
        assert_eq!((region.loop_to, p.region_span(&region)), (13.0, 13.0));
        assert_eq!(p.regions_in(10.0, 11.0, 0, 0), vec![r], "the loop counts as the region");
        p.set_region_loop(r, 3.0);
        assert_eq!(p.find_region(r).unwrap().1.loop_to, 0.0, "shorter than the region: no loop");
        p.set_region_loop(r, 12.0);
        p.trim_region(r, true, 2.0);
        assert_eq!(p.find_region(r).unwrap().1.loop_to, 10.0, "the loop end stays put");

        assert!(p.set_region_gain(r, 40.0) && p.find_region(r).unwrap().1.gain_db == 24.0);
        assert!(p.set_region_reverse(r, true) && p.find_region(r).unwrap().1.reverse);
        let m = p.add_instrument_track(InstrumentKind::Synth);
        let mr = p.add_midi_region(m, 0.0, 4.0).unwrap();
        assert!(!p.set_region_gain(mr, 3.0) && !p.set_region_reverse(mr, true), "audio only");
    }

    #[test]
    fn joining_merges_midi_and_rejoins_split_audio() {
        let mut p = project();
        let t = p.add_instrument_track(InstrumentKind::Synth);
        let a = p.add_midi_region(t, 0.0, 4.0).unwrap();
        let b = p.add_midi_region(t, 8.0, 2.0).unwrap();
        p.add_note(a, 1.0, 1.0, 60, 100);
        p.add_note(b, 0.0, 1.0, 62, 100);
        p.set_region_loop(b, 12.0); // two passes of b
        let joined = p.join_regions(&[b, a]).unwrap();
        assert_eq!(joined, a);
        let region = p.find_region(a).unwrap().1;
        let m = region.midi.as_ref().unwrap();
        assert_eq!(m.length, 12.0);
        assert_eq!(m.notes.iter().map(|n| (n.start, n.key)).collect::<Vec<_>>(), vec![(1.0, 60), (8.0, 62), (10.0, 62)]);
        assert!(p.find_region(b).is_none());

        let audio = p.add_track(None);
        let r = p.add_region(audio, "a.wav", "a", 0.0, 4.0).unwrap();
        let right = p.split_region(r, 4.0).unwrap();
        let other = p.add_region(audio, "b.wav", "b", 20.0, 1.0).unwrap();
        assert!(p.join_regions(&[r, other]).is_none(), "different recordings");
        assert_eq!(p.join_regions(&[r, right]), Some(r));
        assert_eq!(p.find_region(r).unwrap().1.length, 4.0);
        assert!(p.join_regions(&[r, a]).is_none(), "different tracks");
    }
}
