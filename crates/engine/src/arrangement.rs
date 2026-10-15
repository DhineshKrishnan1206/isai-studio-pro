//! What the audio thread plays: tracks of regions, already resolved to audio
//! and sample positions.
//!
//! The UI builds a new `Arrangement` after every edit and sends it to the
//! engine whole. The audio thread never edits it, so no locking is needed, and
//! the old one is handed back to the UI thread to free.

use std::sync::Arc;

use crate::clip::Clip;
use crate::effects::PlayInsert;
use crate::instruments::{PlayInstrument, PlayNote};

/// Most tracks the mixer keeps smoothing state for. Tracks beyond this play
/// without gain smoothing (still correct, but fader moves may click).
pub const MAX_TRACKS: usize = 256;
/// Most buses the mixer has room for.
pub const MAX_BUSES: usize = 32;

pub struct Arrangement {
    pub tracks: Vec<PlayTrack>,
    /// Buses (aux channels) that tracks send to; they play into the master.
    pub buses: Vec<PlayBus>,
    /// Effects on the master, after everything is mixed.
    pub master: Vec<PlayInsert>,
    /// Song tempo, for tempo-synced effects.
    pub bpm: f32,
}

impl Default for Arrangement {
    fn default() -> Self {
        Self { tracks: Vec::new(), buses: Vec::new(), master: Vec::new(), bpm: 120.0 }
    }
}

impl Arrangement {
    /// Every instrument in the arrangement.
    pub fn instruments(&self) -> impl Iterator<Item = &PlayInstrument> {
        self.tracks.iter().filter_map(|t| t.instrument.as_ref())
    }

    /// Every effect in the arrangement: tracks, buses, then the master.
    pub fn inserts(&self) -> impl Iterator<Item = &PlayInsert> {
        self.tracks
            .iter()
            .flat_map(|t| &t.inserts)
            .chain(self.buses.iter().flat_map(|b| &b.inserts))
            .chain(&self.master)
    }
}

/// A bus: like a track, but its sound comes from other tracks' sends.
#[derive(Clone)]
pub struct PlayBus {
    pub gain: f32,
    pub pan: f32,
    pub audible: bool,
    /// Effects, before the fader.
    pub inserts: Vec<PlayInsert>,
}

impl PlayBus {
    pub fn target_gains(&self) -> (f32, f32) {
        pan_gains(self.gain, self.pan, self.audible)
    }
}

/// What an automation lane changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AutoTarget {
    /// Fader level in dB.
    Volume,
    /// -1 left .. 1 right.
    Pan,
    /// Send level in dB to the bus at this index.
    Send(usize),
    /// An effect parameter: the insert's id and the parameter's index.
    Param { insert: u64, index: usize },
}

/// A track parameter changing over time: points at timeline positions
/// (samples), with straight lines between them.
#[derive(Clone, Debug)]
pub struct PlayLane {
    pub target: AutoTarget,
    /// (position, value), sorted by position.
    pub points: Vec<(u64, f32)>,
}

impl PlayLane {
    /// The value at `pos`: the first point's before it, the last one's after it.
    pub fn value_at(&self, pos: u64) -> Option<f32> {
        let first = self.points.first()?;
        let i = self.points.partition_point(|p| p.0 <= pos);
        if i == 0 {
            return Some(first.1);
        }
        let (p0, v0) = self.points[i - 1];
        match self.points.get(i) {
            Some(&(p1, v1)) if p1 > p0 => Some(v0 + (v1 - v0) * ((pos - p0) as f32 / (p1 - p0) as f32)),
            _ => Some(v0),
        }
    }
}

/// Decibels to a linear gain; silent at -60 dB and below.
pub fn db_gain(db: f32) -> f32 {
    if db <= -60.0 { 0.0 } else { 10f32.powf(db.min(6.0) / 20.0) }
}

/// Fader times a constant-power pan law, scaled so centre is unity.
pub(crate) fn pan_gains(gain: f32, pan: f32, audible: bool) -> (f32, f32) {
    if !audible {
        return (0.0, 0.0);
    }
    let angle = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    let norm = std::f32::consts::SQRT_2;
    (gain * angle.cos() * norm, gain * angle.sin() * norm)
}

pub struct PlayTrack {
    /// Linear fader gain.
    pub gain: f32,
    /// -1 left .. 1 right.
    pub pan: f32,
    /// False when muted, or when another track is soloed.
    pub audible: bool,
    /// (bus index, linear level): post-fader copies to buses.
    pub sends: Vec<(usize, f32)>,
    /// Effects, before the fader.
    pub inserts: Vec<PlayInsert>,
    /// Parameters that change over time; they override the values above.
    pub automation: Vec<PlayLane>,
    /// An instrument track's instrument, and the notes it plays (sorted by start).
    pub instrument: Option<PlayInstrument>,
    pub notes: Vec<PlayNote>,
    pub regions: Vec<PlayRegion>,
}

pub struct PlayRegion {
    pub audio: Arc<Clip>,
    /// Timeline position where the region starts, in samples.
    pub start: u64,
    /// Where in the audio it starts playing, in samples.
    pub offset: u64,
    /// How many samples it plays.
    pub len: u64,
    /// Fade lengths in samples. Equal-power curves, so a crossfade between two
    /// overlapping regions keeps a steady loudness.
    pub fade_in: u64,
    pub fade_out: u64,
    /// Linear gain (the region's own volume change).
    pub gain: f32,
    /// Plays its slice of the audio backwards.
    pub reverse: bool,
}

impl PlayTrack {
    /// Left and right gains: the fader times a constant-power pan law, so a
    /// sound panned across the stereo field keeps the same loudness.
    pub fn target_gains(&self) -> (f32, f32) {
        pan_gains(self.gain, self.pan, self.audible)
    }

    /// Fader and pan gains at timeline position `pos`, following automation.
    pub fn gains_at(&self, pos: Option<u64>) -> (f32, f32) {
        let Some(pos) = pos.filter(|_| !self.automation.is_empty()) else { return self.target_gains() };
        let (mut gain, mut pan) = (self.gain, self.pan);
        for lane in &self.automation {
            match lane.target {
                AutoTarget::Volume => gain = lane.value_at(pos).map_or(gain, db_gain),
                AutoTarget::Pan => pan = lane.value_at(pos).unwrap_or(pan),
                _ => {}
            }
        }
        pan_gains(gain, pan, self.audible)
    }

    /// The send level to bus `bus` at `pos`, following automation.
    pub fn send_at(&self, bus: usize, level: f32, pos: Option<u64>) -> f32 {
        pos.and_then(|pos| self.automation.iter().find(|l| l.target == AutoTarget::Send(bus))?.value_at(pos))
            .map_or(level, db_gain)
    }
}

impl PlayRegion {
    /// The (left, right) sample at timeline position `pos`, if the region covers it.
    #[inline]
    pub fn sample_at(&self, pos: u64) -> Option<(f32, f32)> {
        if pos < self.start || pos >= self.start + self.len {
            return None;
        }
        let t = pos - self.start;
        let i = (self.offset + if self.reverse { self.len - 1 - t } else { t }) as usize;
        let (l, r) = (*self.audio.left.get(i)?, *self.audio.right.get(i)?);
        let mut g = self.gain;
        if t < self.fade_in {
            g *= (t as f32 / self.fade_in as f32 * std::f32::consts::FRAC_PI_2).sin();
        }
        let left = self.len - t;
        if left <= self.fade_out {
            g *= (left as f32 / self.fade_out as f32 * std::f32::consts::FRAC_PI_2).sin();
        }
        Some((l * g, r * g))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centre_pan_is_unity_and_hard_pan_is_one_side() {
        let mut t = PlayTrack { gain: 1.0, pan: 0.0, audible: true, sends: vec![], inserts: vec![], automation: vec![], instrument: None, notes: vec![], regions: vec![] };
        let (l, r) = t.target_gains();
        assert!((l - 1.0).abs() < 1e-6 && (r - 1.0).abs() < 1e-6);
        t.pan = -1.0;
        let (l, r) = t.target_gains();
        assert!(l > 1.4 && r.abs() < 1e-6);
        t.audible = false;
        assert_eq!(t.target_gains(), (0.0, 0.0));
    }

    #[test]
    fn regions_play_their_slice_of_the_audio() {
        let audio = Arc::new(Clip {
            name: String::new(),
            sample_rate: 10,
            left: (0..10).map(|i| i as f32).collect(),
            right: vec![0.0; 10],
        });
        let r = PlayRegion { audio, start: 100, offset: 3, len: 4, fade_in: 0, fade_out: 0, gain: 1.0, reverse: false };
        assert_eq!(r.sample_at(99), None);
        assert_eq!(r.sample_at(100), Some((3.0, 0.0)));
        assert_eq!(r.sample_at(103), Some((6.0, 0.0)));
        assert_eq!(r.sample_at(104), None);
    }

    #[test]
    fn reversed_regions_play_their_slice_backwards_with_their_gain() {
        let audio = Arc::new(Clip {
            name: String::new(),
            sample_rate: 10,
            left: (0..10).map(|i| i as f32).collect(),
            right: vec![0.0; 10],
        });
        let r = PlayRegion { audio, start: 0, offset: 3, len: 4, fade_in: 0, fade_out: 0, gain: 0.5, reverse: true };
        let played: Vec<f32> = (0..4).map(|p| r.sample_at(p).unwrap().0).collect();
        assert_eq!(played, vec![3.0, 2.5, 2.0, 1.5]);
    }

    #[test]
    fn fades_rise_and_fall() {
        let audio = Arc::new(Clip { name: String::new(), sample_rate: 10, left: vec![1.0; 100], right: vec![1.0; 100] });
        let r = PlayRegion { audio, start: 0, offset: 0, len: 100, fade_in: 10, fade_out: 10, gain: 1.0, reverse: false };
        assert_eq!(r.sample_at(0).unwrap().0, 0.0);
        assert!((r.sample_at(5).unwrap().0 - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5);
        assert_eq!(r.sample_at(50).unwrap().0, 1.0);
        assert!(r.sample_at(99).unwrap().0 < 0.2);
    }

    #[test]
    fn automation_draws_straight_lines_between_points() {
        let lane = PlayLane { target: AutoTarget::Volume, points: vec![(100, -12.0), (200, 0.0)] };
        assert_eq!(lane.value_at(0), Some(-12.0), "before the first point");
        assert_eq!(lane.value_at(150), Some(-6.0));
        assert_eq!(lane.value_at(500), Some(0.0), "after the last point");
        assert_eq!(PlayLane { target: AutoTarget::Pan, points: vec![] }.value_at(5), None);

        let mut t = PlayTrack { gain: 1.0, pan: 0.0, audible: true, sends: vec![], inserts: vec![], automation: vec![lane], instrument: None, notes: vec![], regions: vec![] };
        let (l, _) = t.gains_at(Some(0));
        assert!((l - db_gain(-12.0)).abs() < 1e-5);
        assert_eq!(t.gains_at(None), t.target_gains(), "stopped: the fader's own value");
        t.automation.push(PlayLane { target: AutoTarget::Send(0), points: vec![(0, -6.0)] });
        assert!((t.send_at(0, 1.0, Some(10)) - db_gain(-6.0)).abs() < 1e-5);
        assert_eq!(t.send_at(1, 0.7, Some(10)), 0.7);
    }
}
