//! Musical time: converting between samples, seconds and bars/beats.
//!
//! Tempo is always in quarter notes per minute, as in other DAWs. A "beat" is
//! one note of the time signature's denominator, so 6/8 counts six eighths per
//! bar and 4/4 counts four quarters.

/// Subdivisions shown in a bar.beat.division.tick position, as in pro DAWs:
/// four sixteenth-note divisions per quarter note, 240 ticks per division.
pub const TICKS_PER_DIVISION: u32 = 240;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Meter {
    /// Quarter notes per minute.
    pub bpm: f64,
    /// Beats per bar, e.g. 3 in 3/4.
    pub numerator: u32,
    /// The note value of one beat: 4 = quarter, 8 = eighth.
    pub denominator: u32,
}

impl Default for Meter {
    fn default() -> Self {
        Self { bpm: 120.0, numerator: 4, denominator: 4 }
    }
}

/// A position as bar, beat, division and tick, all counted from 1 except ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarBeat {
    pub bar: i64,
    pub beat: u32,
    pub division: u32,
    pub tick: u32,
}

impl Meter {
    pub const MIN_BPM: f64 = 20.0;
    pub const MAX_BPM: f64 = 400.0;

    /// Length of one beat (one denominator note) in seconds.
    pub fn beat_secs(&self) -> f64 {
        60.0 / self.bpm.clamp(Self::MIN_BPM, Self::MAX_BPM) * 4.0 / f64::from(self.denominator.max(1))
    }

    pub fn samples_per_beat(&self, sample_rate: u32) -> f64 {
        self.beat_secs() * f64::from(sample_rate)
    }

    pub fn bar_secs(&self) -> f64 {
        self.beat_secs() * f64::from(self.numerator.max(1))
    }

    pub fn secs_to_beats(&self, secs: f64) -> f64 {
        secs / self.beat_secs()
    }

    pub fn beats_to_secs(&self, beats: f64) -> f64 {
        beats * self.beat_secs()
    }

    pub fn beats_to_samples(&self, beats: f64, sample_rate: u32) -> u64 {
        (beats * self.samples_per_beat(sample_rate)).round().max(0.0) as u64
    }

    /// Splits a position in beats into bar.beat.division.tick. Divisions are
    /// sixteenths of a quarter note, scaled to the beat length.
    pub fn bar_beat(&self, beats: f64) -> BarBeat {
        let num = f64::from(self.numerator.max(1));
        let beats = beats.max(0.0);
        let bar = (beats / num).floor();
        let in_bar = beats - bar * num;
        let beat = in_bar.floor();
        // Divisions per beat: four per quarter note, so two per eighth-note beat.
        let divisions = (16.0 / f64::from(self.denominator.max(1))).max(1.0);
        let in_beat = (in_bar - beat) * divisions;
        let division = in_beat.floor();
        let tick = ((in_beat - division) * f64::from(TICKS_PER_DIVISION)).floor();
        BarBeat {
            bar: bar as i64 + 1,
            beat: beat as u32 + 1,
            division: division as u32 + 1,
            tick: (tick as u32).min(TICKS_PER_DIVISION - 1),
        }
    }
}

impl std::fmt::Display for BarBeat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {} {:>3}", self.bar, self.beat, self.division, self.tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beat_lengths() {
        let m = Meter::default();
        assert_eq!(m.beat_secs(), 0.5);
        assert_eq!(m.bar_secs(), 2.0);
        // In 6/8 at 120 quarter-note BPM, an eighth lasts a quarter second.
        let m = Meter { bpm: 120.0, numerator: 6, denominator: 8 };
        assert_eq!(m.beat_secs(), 0.25);
        assert_eq!(m.bar_secs(), 1.5);
    }

    #[test]
    fn bar_beat_positions() {
        let m = Meter::default();
        assert_eq!(m.bar_beat(0.0), BarBeat { bar: 1, beat: 1, division: 1, tick: 0 });
        assert_eq!(m.bar_beat(4.0), BarBeat { bar: 2, beat: 1, division: 1, tick: 0 });
        assert_eq!(m.bar_beat(5.25), BarBeat { bar: 2, beat: 2, division: 2, tick: 0 });
        // Half a sixteenth (0.125 beats) is tick 120 of 240.
        assert_eq!(m.bar_beat(5.25 + 0.125).tick, 120);
        let three = Meter { numerator: 3, ..m };
        assert_eq!(three.bar_beat(3.0).bar, 2);
    }

    #[test]
    fn samples_round_trip() {
        let m = Meter::default();
        assert_eq!(m.beats_to_samples(4.0, 48_000), 96_000);
        assert_eq!(m.samples_per_beat(48_000), 24_000.0);
    }
}
