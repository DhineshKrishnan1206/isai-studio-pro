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

/// A tempo change: from `beat` on, the song plays at `bpm` (quarter notes per minute).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TempoPoint {
    pub beat: f64,
    pub bpm: f64,
}

/// A time-signature change at the start of `bar` (counted from 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SigChange {
    pub bar: u32,
    pub numerator: u32,
    pub denominator: u32,
}

/// The song's tempo and time signature over time.
///
/// Positions are in the song's beats: notes of its first time signature's
/// denominator, as everywhere else. The tempo changes in steps at each
/// `TempoPoint`; a time signature applies from its bar to the next change.
/// Converting between beats and seconds adds up the time of each tempo
/// section, so everything after a tempo change moves with it.
#[derive(Clone, Debug, PartialEq)]
pub struct TempoMap {
    /// The beat unit: the first time signature's denominator.
    base_den: u32,
    /// (first beat, bpm, seconds at that beat), starting at beat 0.
    tempos: Vec<(f64, f64, f64)>,
    /// (first beat, first bar, numerator, denominator), starting at bar 1.
    sigs: Vec<(f64, i64, u32, u32)>,
}

impl Default for TempoMap {
    fn default() -> Self {
        Self::constant(Meter::default())
    }
}

impl TempoMap {
    /// One tempo and time signature for the whole song.
    pub fn constant(meter: Meter) -> Self {
        Self::new(meter, &[], &[])
    }

    /// `base` is the song's opening tempo and time signature.
    pub fn new(base: Meter, tempo_changes: &[TempoPoint], sig_changes: &[SigChange]) -> Self {
        let base_den = base.denominator.max(1);
        let clamp = |bpm: f64| bpm.clamp(Meter::MIN_BPM, Meter::MAX_BPM);
        let beat_secs = |bpm: f64| 60.0 / bpm * 4.0 / f64::from(base_den);

        let mut changes: Vec<TempoPoint> = tempo_changes.iter().copied().filter(|t| t.beat > 1e-9 && t.bpm.is_finite()).collect();
        changes.sort_by(|a, b| a.beat.total_cmp(&b.beat));
        let mut tempos: Vec<(f64, f64, f64)> = vec![(0.0, clamp(base.bpm), 0.0)];
        for t in changes {
            let &(beat, bpm, secs) = tempos.last().unwrap_or(&(0.0, 120.0, 0.0));
            if (t.beat - beat).abs() < 1e-9 {
                if let Some(last) = tempos.last_mut() {
                    last.1 = clamp(t.bpm);
                }
                continue;
            }
            tempos.push((t.beat, clamp(t.bpm), secs + (t.beat - beat) * beat_secs(bpm)));
        }

        let mut bars: Vec<SigChange> = sig_changes.iter().copied().filter(|c| c.bar > 1 && c.numerator > 0 && c.denominator > 0).collect();
        bars.sort_by_key(|c| c.bar);
        let mut sigs: Vec<(f64, i64, u32, u32)> = vec![(0.0, 1, base.numerator.max(1), base_den)];
        for c in bars {
            let &(beat, bar, num, den) = sigs.last().unwrap_or(&(0.0, 1, 4, 4));
            let at = i64::from(c.bar);
            if at == bar {
                if let Some(last) = sigs.last_mut() {
                    last.2 = c.numerator;
                    last.3 = c.denominator;
                }
                continue;
            }
            let bar_len = f64::from(num) * f64::from(base_den) / f64::from(den);
            sigs.push((beat + (at - bar) as f64 * bar_len, at, c.numerator, c.denominator));
        }
        Self { base_den, tempos, sigs }
    }

    fn beat_secs(&self, bpm: f64) -> f64 {
        60.0 / bpm * 4.0 / f64::from(self.base_den)
    }

    fn tempo_index(&self, beat: f64) -> usize {
        self.tempos.partition_point(|t| t.0 <= beat).saturating_sub(1)
    }

    fn sig_index(&self, beat: f64) -> usize {
        self.sigs.partition_point(|s| s.0 <= beat + 1e-9).saturating_sub(1)
    }

    /// The tempo at a position.
    pub fn bpm_at(&self, beat: f64) -> f64 {
        self.tempos[self.tempo_index(beat)].1
    }

    /// The tempo and time signature in force at a position.
    pub fn meter_at(&self, beat: f64) -> Meter {
        let s = self.sigs[self.sig_index(beat)];
        Meter { bpm: self.bpm_at(beat), numerator: s.2, denominator: s.3 }
    }

    /// The opening tempo and time signature.
    pub fn base(&self) -> Meter {
        Meter { bpm: self.tempos[0].1, numerator: self.sigs[0].2, denominator: self.base_den }
    }

    pub fn has_changes(&self) -> bool {
        self.tempos.len() > 1 || self.sigs.len() > 1
    }

    pub fn beats_to_secs(&self, beats: f64) -> f64 {
        if beats <= 0.0 {
            return beats * self.beat_secs(self.tempos[0].1);
        }
        let (b, bpm, secs) = self.tempos[self.tempo_index(beats)];
        secs + (beats - b) * self.beat_secs(bpm)
    }

    pub fn secs_to_beats(&self, secs: f64) -> f64 {
        if secs <= 0.0 {
            return secs / self.beat_secs(self.tempos[0].1);
        }
        let i = self.tempos.partition_point(|t| t.2 <= secs).saturating_sub(1);
        let (b, bpm, s) = self.tempos[i];
        b + (secs - s) / self.beat_secs(bpm)
    }

    pub fn beats_to_samples(&self, beats: f64, sample_rate: u32) -> u64 {
        (self.beats_to_secs(beats) * f64::from(sample_rate)).round().max(0.0) as u64
    }

    pub fn samples_to_beats(&self, samples: u64, sample_rate: u32) -> f64 {
        self.secs_to_beats(samples as f64 / f64::from(sample_rate.max(1)))
    }

    /// Where a bar (counted from 1) starts, in beats.
    pub fn bar_start(&self, bar: i64) -> f64 {
        let i = self.sigs.partition_point(|s| s.1 <= bar).saturating_sub(1);
        let (beat, first, num, den) = self.sigs[i];
        beat + (bar - first) as f64 * f64::from(num) * f64::from(self.base_den) / f64::from(den)
    }

    /// The bar (counted from 1) a position is in.
    pub fn bar_at(&self, beats: f64) -> i64 {
        self.bar_beat(beats).bar
    }

    /// Splits a position into bar.beat.division.tick, following time
    /// signature changes. Beats count the section's own note value.
    pub fn bar_beat(&self, beats: f64) -> BarBeat {
        let beats = beats.max(0.0);
        let (start, first, num, den) = self.sigs[self.sig_index(beats)];
        let unit = f64::from(self.base_den) / f64::from(den);
        let local = Meter { bpm: 120.0, numerator: num, denominator: den };
        let mut bb = local.bar_beat((beats - start).max(0.0) / unit);
        bb.bar += first - 1;
        bb
    }

    /// The first metronome beat at or after `beat`, and whether it starts a bar.
    pub fn next_click(&self, beat: f64) -> (f64, bool) {
        let i = self.sig_index(beat.max(0.0));
        let (start, _, num, den) = self.sigs[i];
        let unit = f64::from(self.base_den) / f64::from(den);
        // A hair of tolerance, so a beat landing on `beat` after rounding still counts.
        let k = ((beat - start) / unit - 1e-6).ceil().max(0.0);
        let at = start + k * unit;
        if let Some(next) = self.sigs.get(i + 1)
            && at >= next.0 - 1e-9
        {
            return (next.0, true);
        }
        (at, (k as i64).rem_euclid(i64::from(num.max(1))) == 0)
    }

    /// Every tempo section: (first beat, bpm).
    pub fn tempos(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.tempos.iter().map(|t| (t.0, t.1))
    }

    /// Every time-signature section: (first beat, first bar, numerator, denominator).
    pub fn sections(&self) -> &[(f64, i64, u32, u32)] {
        &self.sigs
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
    fn tempo_changes_move_everything_after_them() {
        // 120 BPM for 4 bars, then 60 BPM: bar 5 starts at 8 s, and a beat after it lasts 1 s.
        let map = TempoMap::new(Meter::default(), &[TempoPoint { beat: 16.0, bpm: 60.0 }], &[]);
        assert_eq!(map.beats_to_secs(16.0), 8.0);
        assert_eq!(map.beats_to_secs(17.0), 9.0);
        assert_eq!(map.secs_to_beats(9.0), 17.0);
        assert_eq!(map.secs_to_beats(4.0), 8.0);
        assert_eq!(map.bpm_at(15.9), 120.0);
        assert_eq!(map.bpm_at(16.0), 60.0);
        assert_eq!(map.beats_to_samples(17.0, 48_000), 9 * 48_000);
        assert_eq!(map.samples_to_beats(9 * 48_000, 48_000), 17.0);
        // No changes: the same as a plain Meter.
        let plain = TempoMap::constant(Meter::default());
        assert_eq!(plain.beats_to_secs(10.0), Meter::default().beats_to_secs(10.0));
    }

    #[test]
    fn time_signature_changes_shift_the_bars() {
        // Two bars of 4/4, then 3/4 from bar 3, then 6/8 from bar 5.
        let sigs = [SigChange { bar: 3, numerator: 3, denominator: 4 }, SigChange { bar: 5, numerator: 6, denominator: 8 }];
        let map = TempoMap::new(Meter::default(), &[], &sigs);
        assert_eq!(map.bar_start(3), 8.0);
        assert_eq!(map.bar_start(4), 11.0);
        assert_eq!(map.bar_start(5), 14.0);
        // A 6/8 bar is six eighths: three quarter-note beats.
        assert_eq!(map.bar_start(6), 17.0);
        assert_eq!(map.bar_beat(11.0), BarBeat { bar: 4, beat: 1, division: 1, tick: 0 });
        assert_eq!(map.bar_beat(15.5), BarBeat { bar: 5, beat: 4, division: 1, tick: 0 });
        assert_eq!(map.meter_at(15.0).numerator, 6);
        // Clicks: every eighth in 6/8, with the downbeat on each bar.
        assert_eq!(map.next_click(14.0), (14.0, true));
        assert_eq!(map.next_click(14.1), (14.5, false));
        assert_eq!(map.next_click(16.9), (17.0, true));
        // The last beat of the 4/4 section clicks, then bar 3 starts with a downbeat.
        assert_eq!(map.next_click(7.0), (7.0, false));
        assert_eq!(map.next_click(7.5), (8.0, true));
    }

    #[test]
    fn samples_round_trip() {
        let m = Meter::default();
        assert_eq!(m.beats_to_samples(4.0, 48_000), 96_000);
        assert_eq!(m.samples_per_beat(48_000), 24_000.0);
    }
}
