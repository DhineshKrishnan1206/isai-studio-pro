//! Mixes an arrangement one block at a time.
//!
//! Playback and export both go through `Mixer`, so an export sounds exactly
//! like what you hear. Working in blocks (rather than one sample at a time)
//! is what lets each track run its own effects later.

use crate::arrangement::{Arrangement, MAX_BUSES, MAX_TRACKS};
use crate::effects::{Context, EffectBank};
use crate::instruments::{InstrumentBank, LiveNote};

/// Most frames mixed in one go. Buffers are allocated once, up front.
pub const BLOCK: usize = 512;

/// A timeline position with nothing to play: the transport is counting in,
/// or stopped and silent.
pub const SILENT: u64 = u64::MAX;

/// How quickly fader, pan, mute and solo changes glide to their new value
/// (per sample): a few milliseconds, so changes never click.
const GLIDE: f32 = 0.003;

/// What one block plays.
pub struct Block<'a> {
    /// The timeline position of each frame, or `SILENT`.
    pub positions: &'a [u64],
    /// Start/stop fade per frame, applied to the song's regions only, so live
    /// notes and effect tails keep sounding when the transport stops.
    pub fades: &'a [f32],
    /// Notes played live this block, for the track at `live_track`.
    pub live: &'a [LiveNote],
    pub live_track: Option<usize>,
}

/// How long effects keep running after everything goes quiet, so reverb and
/// delay tails ring out.
const TAIL_SECONDS: f32 = 3.0;

pub struct Mixer {
    /// Blocks in a row with nothing playing.
    quiet_blocks: usize,
    /// Smoothed (left, right) gain per track, then per bus.
    gains: Vec<(f32, f32)>,
    bus_gains: Vec<(f32, f32)>,
    /// Last send level per track and bus (`track * MAX_BUSES + bus`), so
    /// send changes ramp over a block instead of clicking.
    send_gains: Vec<f32>,
    /// Post-fader peak per track (`0..MAX_TRACKS`), then per bus, since the last `take_peaks`.
    peaks: Vec<(f32, f32)>,
    track_l: Vec<f32>,
    track_r: Vec<f32>,
    /// One block per bus, back to back.
    bus_l: Vec<f32>,
    bus_r: Vec<f32>,
    /// The live track's instrument: keys held in the last block, and keys
    /// struck since `take_keys` (bit `n` = MIDI note `n`).
    keys_held: u128,
    keys_struck: u128,
    /// How hard the live track's keys were last played.
    key_velocities: [u8; 128],
}

impl Default for Mixer {
    fn default() -> Self {
        Self::new()
    }
}

impl Mixer {
    pub fn new() -> Self {
        Self {
            quiet_blocks: 0,
            gains: vec![(0.0, 0.0); MAX_TRACKS],
            bus_gains: vec![(0.0, 0.0); MAX_BUSES],
            send_gains: vec![0.0; MAX_TRACKS * MAX_BUSES],
            peaks: vec![(0.0, 0.0); MAX_TRACKS + MAX_BUSES],
            track_l: vec![0.0; BLOCK],
            track_r: vec![0.0; BLOCK],
            bus_l: vec![0.0; BLOCK * MAX_BUSES],
            bus_r: vec![0.0; BLOCK * MAX_BUSES],
            keys_held: 0,
            keys_struck: 0,
            key_velocities: [0; 128],
        }
    }

    /// The live track's keys: held now, struck since the last call, and
    /// how hard each was last played.
    pub fn take_keys(&mut self) -> (u128, u128, &[u8; 128]) {
        (self.keys_held, std::mem::take(&mut self.keys_struck), &self.key_velocities)
    }

    /// Jumps every track straight to its gain, with no glide (for export).
    pub fn snap_gains(&mut self, arrangement: &Arrangement) {
        for (g, t) in self.gains.iter_mut().zip(&arrangement.tracks) {
            *g = t.target_gains();
        }
        for (g, b) in self.bus_gains.iter_mut().zip(&arrangement.buses) {
            *g = b.target_gains();
        }
        for (i, t) in arrangement.tracks.iter().enumerate().take(MAX_TRACKS) {
            for &(bus, level) in &t.sends {
                if bus < MAX_BUSES {
                    self.send_gains[i * MAX_BUSES + bus] = level;
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn set_all_gains(&mut self, gain: f32) {
        self.gains.fill((gain, gain));
        self.bus_gains.fill((gain, gain));
    }

    /// Mixes the tracks at `positions` (one timeline position per frame, or
    /// `SILENT`) into `out_l` / `out_r`, which are overwritten. At most `BLOCK` frames.
    ///
    /// Effects run from `bank`, at `sample_rate`. Each track and bus runs its
    /// effects before its fader; the master's run last, on the whole mix.
    #[allow(clippy::too_many_arguments)]
    pub fn mix(
        &mut self,
        arrangement: &Arrangement,
        bank: &mut EffectBank,
        instruments: &mut InstrumentBank,
        sample_rate: f32,
        block: &Block,
        out_l: &mut [f32],
        out_r: &mut [f32],
    ) {
        // Tempo-synced effects follow the tempo where this block plays.
        let bpm = block.positions.iter().find(|&&p| p != SILENT).map_or(arrangement.bpm, |&p| arrangement.bpm_at(p));
        let ctx = Context { sample_rate, bpm };
        let positions = block.positions;
        let n = positions.len().min(BLOCK).min(out_l.len()).min(out_r.len());
        out_l[..n].fill(0.0);
        out_r[..n].fill(0.0);
        self.keys_held = 0;
        // The block's span on the timeline, to skip regions it doesn't touch.
        let (lo, hi) = positions[..n]
            .iter()
            .filter(|&&p| p != SILENT)
            .fold((u64::MAX, 0), |(lo, hi), &p| (lo.min(p), hi.max(p)));
        // Nothing playing (no song, no notes) for a few seconds: clear the
        // effects once and skip them.
        let silent = lo > hi && block.live.is_empty() && !instruments.any_active();
        self.quiet_blocks = if silent { self.quiet_blocks.saturating_add(1) } else { 0 };
        let quiet = (self.quiet_blocks * BLOCK) as f32 > TAIL_SECONDS * sample_rate;
        // Where automation is read for this block.
        let block_pos = positions[..n].iter().copied().find(|&p| p != SILENT);
        bank.set_quiet(quiet);
        let buses = arrangement.buses.len().min(MAX_BUSES);
        self.bus_l[..buses * BLOCK].fill(0.0);
        self.bus_r[..buses * BLOCK].fill(0.0);

        for (i, track) in arrangement.tracks.iter().enumerate() {
            let target = track.gains_at(block_pos);
            let mut gain = self.gains.get(i).copied().unwrap_or(target);
            if gain == (0.0, 0.0) && target == (0.0, 0.0) {
                continue;
            }

            // 1. The track's own audio.
            let (tl, tr) = (&mut self.track_l[..n], &mut self.track_r[..n]);
            tl.fill(0.0);
            tr.fill(0.0);
            if lo <= hi {
                for region in &track.regions {
                    if region.start > hi || region.start + region.len <= lo {
                        continue;
                    }
                    for (k, &pos) in positions[..n].iter().enumerate() {
                        if let Some((a, b)) = region.sample_at(pos) {
                            let f = block.fades.get(k).copied().unwrap_or(1.0);
                            tl[k] += a * f;
                            tr[k] += b * f;
                        }
                    }
                }
            }

            if let Some(inst) = &track.instrument
                && let Some(instrument) = instruments.get_mut(inst.id, inst.kind)
            {
                let live: &[LiveNote] = if block.live_track == Some(i) { block.live } else { &[] };
                instrument.render(inst, &track.notes, &positions[..n], live, sample_rate, tl, tr);
                if block.live_track == Some(i) {
                    let (held, struck) = instrument.keys();
                    self.keys_held = held;
                    self.keys_struck |= struck;
                    if struck != 0 {
                        self.key_velocities = *instrument.velocities();
                    }
                }
            }

            if !quiet && !track.inserts.is_empty() {
                bank.run_automated(&track.inserts, &track.automation, block_pos, &ctx, tl, tr);
            }

            // 2. Fader and pan (gliding), metering, and into the mix.
            let (mut pl, mut pr) = (0.0f32, 0.0f32);
            for k in 0..n {
                gain.0 += (target.0 - gain.0) * GLIDE;
                gain.1 += (target.1 - gain.1) * GLIDE;
                let (a, b) = (tl[k] * gain.0, tr[k] * gain.1);
                pl = pl.max(a.abs());
                pr = pr.max(b.abs());
                out_l[k] += a;
                out_r[k] += b;
                // Keep the post-fader signal for the sends.
                tl[k] = a;
                tr[k] = b;
            }
            // 3. Sends: post-fader copies into the buses, ramped to their new level.
            for &(bus, level) in &track.sends {
                if bus >= buses {
                    continue;
                }
                let level = track.send_at(bus, level, block_pos);
                let slot = (i < MAX_TRACKS).then(|| i * MAX_BUSES + bus);
                let from = slot.map_or(level, |s| self.send_gains[s]);
                if from == 0.0 && level == 0.0 {
                    continue;
                }
                let step = (level - from) / n as f32;
                let (bl, br) = (&mut self.bus_l[bus * BLOCK..bus * BLOCK + n], &mut self.bus_r[bus * BLOCK..bus * BLOCK + n]);
                for k in 0..n {
                    let g = from + step * (k + 1) as f32;
                    bl[k] += tl[k] * g;
                    br[k] += tr[k] * g;
                }
                if let Some(s) = slot {
                    self.send_gains[s] = level;
                }
            }
            if let Some(g) = self.gains.get_mut(i) {
                *g = gain;
            }
            if let Some(p) = self.peaks.get_mut(i).filter(|_| i < MAX_TRACKS) {
                p.0 = p.0.max(pl);
                p.1 = p.1.max(pr);
            }
        }

        // 4. Buses into the mix.
        for (j, bus) in arrangement.buses.iter().take(buses).enumerate() {
            let target = bus.target_gains();
            let mut gain = self.bus_gains[j];
            let (bl, br) = (&mut self.bus_l[j * BLOCK..j * BLOCK + n], &mut self.bus_r[j * BLOCK..j * BLOCK + n]);
            if !quiet && !bus.inserts.is_empty() {
                bank.run(&bus.inserts, &ctx, bl, br);
            }
            let (mut pl, mut pr) = (0.0f32, 0.0f32);
            for k in 0..n {
                gain.0 += (target.0 - gain.0) * GLIDE;
                gain.1 += (target.1 - gain.1) * GLIDE;
                let (a, b) = (bl[k] * gain.0, br[k] * gain.1);
                pl = pl.max(a.abs());
                pr = pr.max(b.abs());
                out_l[k] += a;
                out_r[k] += b;
            }
            self.bus_gains[j] = gain;
            let p = &mut self.peaks[MAX_TRACKS + j];
            p.0 = p.0.max(pl);
            p.1 = p.1.max(pr);
        }

        // 5. Master effects.
        if !quiet && !arrangement.master.is_empty() {
            bank.run(&arrangement.master, &ctx, &mut out_l[..n], &mut out_r[..n]);
        }
    }

    /// Calls `f(index, left, right)` with each track's peak since the last
    /// call, then each bus's (numbered after the tracks), and resets them.
    pub fn take_peaks(&mut self, tracks: usize, buses: usize, mut f: impl FnMut(usize, f32, f32)) {
        let tracks = tracks.min(MAX_TRACKS);
        for (i, p) in self.peaks.iter_mut().take(tracks).enumerate() {
            f(i, p.0, p.1);
            *p = (0.0, 0.0);
        }
        for (j, p) in self.peaks[MAX_TRACKS..].iter_mut().take(buses.min(MAX_BUSES)).enumerate() {
            f(tracks + j, p.0, p.1);
            *p = (0.0, 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::arrangement::{PlayRegion, PlayTrack};
    use crate::clip::Clip;

    fn block(positions: &[u64]) -> Block<'_> {
        Block { positions, fades: &[], live: &[], live_track: None }
    }

    fn track(level: f32, start: u64, gain: f32) -> PlayTrack {
        let audio = Arc::new(Clip { name: String::new(), sample_rate: 1000, left: vec![level; 100], right: vec![level; 100] });
        PlayTrack { gain, pan: 0.0, audible: true, sends: vec![], inserts: vec![], automation: vec![], instrument: None, notes: vec![], regions: vec![PlayRegion { audio, start, offset: 0, len: 100, fade_in: 0, fade_out: 0, gain: 1.0, reverse: false }] }
    }

    #[test]
    fn mixes_tracks_and_skips_silent_positions() {
        let a = Arrangement { tracks: vec![track(0.25, 0, 1.0), track(0.5, 10, 0.5)], ..Default::default() };
        let mut m = Mixer::new();
        m.snap_gains(&a);
        let positions = [0, 10, SILENT, 200];
        let (mut l, mut r) = ([9.0; 4], [9.0; 4]);
        m.mix(&a, &mut EffectBank::default(), &mut InstrumentBank::default(), 1000.0, &block(&positions), &mut l, &mut r);
        let want = [0.25, 0.5, 0.0, 0.0];
        assert!(l.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-6), "{l:?}");
        assert_eq!(r, l);
    }

    #[test]
    fn reports_post_fader_peaks_per_track() {
        let a = Arrangement { tracks: vec![track(0.5, 0, 0.5), track(0.5, 1000, 1.0)], ..Default::default() };
        let mut m = Mixer::new();
        m.snap_gains(&a);
        let positions: Vec<u64> = (0..64).collect();
        let (mut l, mut r) = (vec![0.0; 64], vec![0.0; 64]);
        m.mix(&a, &mut EffectBank::default(), &mut InstrumentBank::default(), 1000.0, &block(&positions), &mut l, &mut r);
        let mut seen = vec![];
        m.take_peaks(2, 0, |i, l, _| seen.push((i, l)));
        assert_eq!(seen.len(), 2);
        assert!((seen[0].1 - 0.25).abs() < 1e-6 && seen[1].1 == 0.0, "{seen:?}");
        m.take_peaks(1, 0, |_, l, _| assert_eq!(l, 0.0, "reset after reading"));
    }

    #[test]
    fn gain_changes_glide() {
        let a = Arrangement { tracks: vec![track(1.0, 0, 1.0)], ..Default::default() };
        let mut m = Mixer::new();
        let positions: Vec<u64> = (0..8).collect();
        let (mut l, mut r) = (vec![0.0; 8], vec![0.0; 8]);
        m.mix(&a, &mut EffectBank::default(), &mut InstrumentBank::default(), 1000.0, &block(&positions), &mut l, &mut r);
        assert!(l[0] > 0.0 && l[0] < 0.01 && l[7] > l[0], "rises from silence: {l:?}");
    }

    #[test]
    fn sends_feed_buses_which_play_into_the_mix() {
        use crate::arrangement::PlayBus;
        let mut t = track(0.5, 0, 1.0);
        t.sends = vec![(0, 0.5)];
        let mut muted = track(0.5, 0, 1.0);
        muted.audible = false;
        muted.sends = vec![(0, 1.0)];
        let a = Arrangement { tracks: vec![t, muted], buses: vec![PlayBus { gain: 1.0, pan: 0.0, audible: true, inserts: vec![] }], ..Default::default() };
        let mut m = Mixer::new();
        m.snap_gains(&a);
        let positions: Vec<u64> = (0..4).collect();
        let (mut l, mut r) = (vec![0.0; 4], vec![0.0; 4]);
        m.mix(&a, &mut EffectBank::default(), &mut InstrumentBank::default(), 1000.0, &block(&positions), &mut l, &mut r);
        // 0.5 dry + 0.5 * 0.5 through the bus; the muted track sends nothing (post-fader).
        assert!((l[3] - 0.75).abs() < 1e-5, "{l:?}");
        let mut seen = vec![];
        m.take_peaks(2, 1, |i, l, _| seen.push((i, l)));
        assert_eq!(seen.len(), 3);
        assert!((seen[2].1 - 0.25).abs() < 1e-5, "bus meter: {seen:?}");
    }

    #[test]
    fn automation_moves_the_fader_over_time() {
        use crate::arrangement::{AutoTarget, PlayLane};
        let mut t = track(1.0, 0, 1.0);
        t.automation = vec![PlayLane { target: AutoTarget::Volume, points: vec![(0, -60.0), (50, 0.0)] }];
        let a = Arrangement { tracks: vec![t], ..Default::default() };
        let mut m = Mixer::new();
        m.set_all_gains(0.0);
        let (mut l, mut r) = (vec![0.0; 1], vec![0.0; 1]);
        // At position 0 the lane says silence; at 60 it says 0 dB.
        m.mix(&a, &mut EffectBank::default(), &mut InstrumentBank::default(), 1000.0, &block(&[0]), &mut l, &mut r);
        assert!(l[0].abs() < 1e-6);
        let mut last = 0.0;
        for _ in 0..3000 {
            m.mix(&a, &mut EffectBank::default(), &mut InstrumentBank::default(), 1000.0, &block(&[60]), &mut l, &mut r);
            last = l[0];
        }
        assert!((last - 1.0).abs() < 0.01, "{last}");
    }
}
