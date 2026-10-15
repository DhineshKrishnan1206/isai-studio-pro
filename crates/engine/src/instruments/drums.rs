//! "Isai Drums": a drum machine that synthesises every sound, so it needs
//! no sample files and runs on any computer. Five kits (808, Club, Acoustic,
//! Lo-fi, Trap) shape ten pieces, played by the usual General MIDI keys:
//! kick 36, rim 37, snare 38, clap 39, low tom 43, closed hat 42, open hat 46,
//! high tom 48, crash 49, cowbell 56. A closed hat chokes an open one.
//! Every piece has its own level, tuning, length and pan.

use std::f32::consts::{PI, TAU};

use super::{LiveNote, MAX_INST_PARAMS, PlayNote};
use crate::effects::flush;
use crate::mixer::SILENT;

/// The pieces, in parameter order, with the MIDI key each answers to.
pub const PIECES: [(&str, u8); 10] = [
    ("Kick", 36),
    ("Snare", 38),
    ("Clap", 39),
    ("Rim", 37),
    ("Closed hat", 42),
    ("Open hat", 46),
    ("Low tom", 43),
    ("High tom", 48),
    ("Crash", 49),
    ("Cowbell", 56),
];
pub const KITS: &[&str] = &["808", "Club", "Acoustic", "Lo-fi", "Trap"];

/// Parameters: the kit, volume and drive, then level, tune, decay and pan for
/// each piece in `PIECES` order.
pub mod param {
    pub const KIT: usize = 0;
    pub const VOLUME: usize = 1;
    pub const DRIVE: usize = 2;
    pub const FIRST_PIECE: usize = 3;
    pub const PER_PIECE: usize = 4;
    pub const LEVEL: usize = 0;
    pub const TUNE: usize = 1;
    pub const DECAY: usize = 2;
    pub const PAN: usize = 3;
}

const VOICES: usize = 24;
const KICK: usize = 0;
const SNARE: usize = 1;
const CLAP: usize = 2;
const RIM: usize = 3;
const CLOSED: usize = 4;
const OPEN: usize = 5;
const LOW_TOM: usize = 6;
const HIGH_TOM: usize = 7;
const CRASH: usize = 8;
const COWBELL: usize = 9;
const KIT_808: usize = 0;
const KIT_CLUB: usize = 1;
const KIT_ACOUSTIC: usize = 2;
const KIT_LOFI: usize = 3;
const KIT_TRAP: usize = 4;

/// The 808's six square waves for cymbals and hats, in Hz.
const METAL: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];

/// A state-variable filter (band and high outputs), set once per hit.
#[derive(Clone, Copy, Default)]
struct Svf {
    g: f32,
    k: f32,
    ic1: f32,
    ic2: f32,
}

impl Svf {
    fn new(freq: f32, q: f32, rate: f32) -> Self {
        Self { g: (PI * freq.min(rate * 0.45) / rate).tan(), k: 1.0 / q.max(0.1), ic1: 0.0, ic2: 0.0 }
    }

    /// (band-pass, high-pass)
    #[inline]
    fn run(&mut self, x: f32) -> (f32, f32) {
        let a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + self.g * a1 * v3;
        let v2 = self.ic2 + self.g * v1;
        self.ic1 = flush(2.0 * v1 - self.ic1);
        self.ic2 = flush(2.0 * v2 - self.ic2);
        (v1 * self.k, x - self.k * v1 - v2)
    }
}

/// One drum hit sounding.
#[derive(Clone, Copy)]
struct Voice {
    piece: usize,
    kit: usize,
    /// Samples since the hit.
    t: u32,
    vel: f32,
    /// Tuning ratio and length multiplier from the piece's knobs.
    tune: f32,
    decay: f32,
    gain_l: f32,
    gain_r: f32,
    phase: [f32; 6],
    noise: u32,
    f1: Svf,
    f2: Svf,
    /// 1 normally; falls to 0 when a closed hat chokes an open one.
    choke: f32,
    choking: bool,
    /// Sample hold for the Lo-fi kit's crunch.
    held: f32,
}

impl Voice {
    const OFF: Voice = Voice {
        piece: usize::MAX,
        kit: 0,
        t: 0,
        vel: 0.0,
        tune: 1.0,
        decay: 1.0,
        gain_l: 0.0,
        gain_r: 0.0,
        phase: [0.0; 6],
        noise: 1,
        f1: Svf { g: 0.0, k: 1.0, ic1: 0.0, ic2: 0.0 },
        f2: Svf { g: 0.0, k: 1.0, ic1: 0.0, ic2: 0.0 },
        choke: 1.0,
        choking: false,
        held: 0.0,
    };

    fn active(&self) -> bool {
        self.piece != usize::MAX
    }

    #[inline]
    fn noise(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        (self.noise as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// A sine at `hz` on oscillator `i`.
    #[inline]
    fn sine(&mut self, i: usize, hz: f32, rate: f32) -> f32 {
        self.phase[i] = (self.phase[i] + hz / rate).fract();
        (self.phase[i] * TAU).sin()
    }

    /// The 808's metallic cluster of square waves.
    #[inline]
    fn metal(&mut self, rate: f32) -> f32 {
        let mut s = 0.0;
        for (i, f) in METAL.iter().enumerate() {
            self.phase[i] = (self.phase[i] + f * self.tune / rate).fract();
            s += if self.phase[i] < 0.5 { 1.0 } else { -1.0 };
        }
        s / 6.0
    }

    /// The next sample, or `None` once the hit has died away.
    fn next(&mut self, rate: f32) -> Option<f32> {
        let t = self.t as f32 / rate;
        self.t += 1;
        let d = self.decay;
        // Falls 60 dB over `len` seconds (times the piece's length knob).
        let env = |len: f32| (-6.9 * t / (len * d)).exp();
        let kit = self.kit;
        let s = match self.piece {
            KICK => {
                let (start, end, sweep, len, click) = match kit {
                    KIT_CLUB => (240.0, 55.0, 0.025, 0.5, 0.5),
                    KIT_ACOUSTIC => (130.0, 62.0, 0.012, 0.4, 0.6),
                    KIT_LOFI => (150.0, 50.0, 0.04, 0.8, 0.15),
                    KIT_TRAP => (110.0, 40.0, 0.06, 2.2, 0.1),
                    _ => (140.0, 48.0, 0.045, 1.2, 0.15),
                };
                let hz = (end + (start - end) * (-t / sweep).exp()) * self.tune;
                let body = self.sine(0, hz, rate) * env(len);
                let n = self.noise();
                let beater = n * click * (-t / 0.004).exp();
                if t > len * d {
                    return None;
                }
                body + beater
            }
            SNARE => {
                let (tone, tone_len, rattle_len, rattle): (f32, f32, f32, f32) = match kit {
                    KIT_CLUB => (200.0, 0.2, 0.35, 0.7),
                    KIT_ACOUSTIC => (180.0, 0.18, 0.4, 0.75),
                    KIT_TRAP => (220.0, 0.15, 0.25, 0.7),
                    _ => (185.0, 0.25, 0.3, 0.55),
                };
                let body = (self.sine(0, tone * self.tune, rate) + 0.6 * self.sine(1, tone * 1.8 * self.tune, rate)) * 0.6 * env(tone_len);
                let n = self.noise();
                let (_, high) = self.f1.run(n);
                if t > rattle_len.max(tone_len) * d {
                    return None;
                }
                body * (1.0 - rattle * 0.5) + high * rattle * env(rattle_len)
            }
            CLAP => {
                // Three quick slaps, then a short tail of noise.
                let n = self.noise();
                let (band, _) = self.f1.run(n);
                let bursts = [0.0, 0.011, 0.022].iter().map(|&b| if t >= b { (-(t - b) / 0.006).exp() } else { 0.0 }).fold(0.0, f32::max);
                let tail = if t >= 0.03 { 0.7 * (-6.9 * (t - 0.03) / (0.35 * d)).exp() } else { 0.0 };
                if t > 0.03 + 0.35 * d {
                    return None;
                }
                band * 2.2 * bursts.max(tail)
            }
            RIM => {
                if t > 0.06 * d {
                    return None;
                }
                (self.sine(0, 1700.0 * self.tune, rate) * 0.6 + self.sine(1, 480.0 * self.tune, rate) * 0.5) * env(0.06)
            }
            CLOSED | OPEN | CRASH => {
                let len = match self.piece {
                    CLOSED if kit == KIT_TRAP => 0.06,
                    CLOSED => 0.09,
                    OPEN => 0.6,
                    _ => 2.0,
                };
                // 808 and Trap use the metallic squares; the others, brighter noise.
                let src = if kit == KIT_808 || kit == KIT_TRAP {
                    let m = self.metal(rate);
                    m + 0.15 * self.noise()
                } else {
                    self.noise() * 0.8 + self.metal(rate) * 0.2
                };
                let (band, _) = self.f1.run(src);
                let (_, high) = self.f2.run(band);
                if t > len * d {
                    return None;
                }
                high * 1.6 * env(len)
            }
            LOW_TOM | HIGH_TOM => {
                let (start, end) = if self.piece == LOW_TOM { (110.0, 85.0) } else { (190.0, 150.0) };
                let hz = (end + (start - end) * (-t / 0.05).exp()) * self.tune;
                let len = if kit == KIT_ACOUSTIC { 0.45 } else { 0.7 };
                if t > len * d {
                    return None;
                }
                let n = self.noise();
                self.sine(0, hz, rate) * env(len) + n * 0.08 * (-t / 0.02).exp()
            }
            COWBELL => {
                let sq = |p: f32| if p < 0.5 { 1.0 } else { -1.0 };
                self.phase[0] = (self.phase[0] + 540.0 * self.tune / rate).fract();
                self.phase[1] = (self.phase[1] + 800.0 * self.tune / rate).fract();
                let (band, _) = self.f1.run((sq(self.phase[0]) + sq(self.phase[1])) * 0.5);
                let len = if t < 0.02 { 0.05 } else { 0.4 };
                if t > 0.4 * d {
                    return None;
                }
                band * 1.4 * env(len)
            }
            _ => return None,
        };
        if self.choking {
            self.choke *= 0.995;
            if self.choke < 1e-3 {
                return None;
            }
        }
        // Lo-fi: fewer bits and a lower sample rate.
        let s = if kit == KIT_LOFI {
            if self.t.is_multiple_of(4) {
                self.held = (s * 24.0).round() / 24.0;
            }
            self.held
        } else {
            s
        };
        Some(s * self.vel * self.choke)
    }
}

pub struct Drums {
    voices: [Voice; VOICES],
    expected: u64,
    cursor: usize,
    clock: u32,
    struck: u128,
    held: u128,
    velocities: [u8; 128],
}

impl Default for Drums {
    fn default() -> Self {
        Self::new()
    }
}

impl Drums {
    pub fn new() -> Self {
        Self { voices: [Voice::OFF; VOICES], expected: SILENT, cursor: 0, clock: 1, struck: 0, held: 0, velocities: [0; 128] }
    }

    /// Which piece a MIDI key plays.
    pub fn piece_for(key: u8) -> Option<usize> {
        PIECES.iter().position(|&(_, k)| k == key)
    }

    pub fn keys(&mut self) -> (u128, u128) {
        (std::mem::take(&mut self.held), std::mem::take(&mut self.struck))
    }

    pub fn velocities(&self) -> &[u8; 128] {
        &self.velocities
    }

    pub fn is_active(&self) -> bool {
        self.voices.iter().any(Voice::active)
    }

    pub fn reset(&mut self) {
        self.voices = [Voice::OFF; VOICES];
        self.expected = SILENT;
    }

    fn hit(&mut self, p: &[f32; MAX_INST_PARAMS], key: u8, velocity: u8, rate: f32) {
        let Some(piece) = Self::piece_for(key) else { return };
        self.struck |= 1 << key;
        self.held |= 1 << key;
        self.velocities[usize::from(key)] = velocity.clamp(1, 127);
        let base = param::FIRST_PIECE + piece * param::PER_PIECE;
        let level = 10f32.powf(p[base + param::LEVEL].clamp(-60.0, 6.0) / 20.0);
        let pan = p[base + param::PAN].clamp(-1.0, 1.0);
        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        // A closed hat cuts off an open one, as on a real hi-hat.
        if piece == CLOSED {
            for v in self.voices.iter_mut().filter(|v| v.piece == OPEN) {
                v.choking = true;
            }
        }
        // The same piece again restarts on a fresh voice; the oldest one goes if all are busy.
        let slot = self.voices.iter().position(|v| !v.active()).unwrap_or_else(|| {
            (0..VOICES).max_by_key(|&i| self.voices[i].t).unwrap_or(0)
        });
        let kit = (p[param::KIT].round() as usize).min(KITS.len() - 1);
        let tune = 2f32.powf(p[base + param::TUNE].clamp(-12.0, 12.0) / 12.0);
        let vel = (f32::from(velocity.clamp(1, 127)) / 127.0).powf(1.3);
        self.clock = self.clock.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let (f1, f2) = match piece {
            SNARE => (Svf::new(if kit == KIT_CLUB { 2500.0 } else { 1500.0 } * tune, 0.7, rate), Svf::default()),
            CLAP => (Svf::new(1200.0 * tune, 1.2, rate), Svf::default()),
            CLOSED | OPEN | CRASH => (
                Svf::new(if piece == CRASH { 6000.0 } else { 10_000.0 } * tune, 0.8, rate),
                Svf::new(if piece == CRASH { 4000.0 } else { 7000.0 } * tune, 0.7, rate),
            ),
            COWBELL => (Svf::new(700.0 * tune, 2.0, rate), Svf::default()),
            _ => (Svf::default(), Svf::default()),
        };
        self.voices[slot] = Voice {
            piece,
            kit,
            vel,
            tune,
            decay: p[base + param::DECAY].clamp(0.1, 4.0),
            gain_l: level * angle.cos() * std::f32::consts::SQRT_2,
            gain_r: level * angle.sin() * std::f32::consts::SQRT_2,
            noise: self.clock | 1,
            f1,
            f2,
            ..Voice::OFF
        };
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        p: &[f32; MAX_INST_PARAMS],
        notes: &[PlayNote],
        positions: &[u64],
        live: &[LiveNote],
        rate: f32,
        l: &mut [f32],
        r: &mut [f32],
    ) {
        for n in live.iter().filter(|n| n.on && n.velocity > 0 && n.key < 128) {
            self.hit(p, n.key, n.velocity, rate);
        }
        let volume = 10f32.powf(p[param::VOLUME].clamp(-24.0, 6.0) / 20.0) * 0.5;
        let drive = (p[param::DRIVE] / 100.0).clamp(0.0, 1.0);
        let drive_gain = 1.0 + 8.0 * drive;
        let drive_norm = 1.0 / drive_gain.sqrt();

        for (k, &pos) in positions.iter().enumerate() {
            if pos != SILENT {
                if pos != self.expected {
                    self.cursor = notes.partition_point(|n| n.start < pos);
                }
                while let Some(&n) = notes.get(self.cursor).filter(|n| n.start <= pos) {
                    if n.start == pos {
                        self.hit(p, n.key, n.velocity, rate);
                    }
                    self.cursor += 1;
                }
                self.expected = pos + 1;
            } else {
                self.expected = SILENT;
            }
            let (mut out_l, mut out_r) = (0.0, 0.0);
            for v in &mut self.voices {
                if !v.active() {
                    continue;
                }
                match v.next(rate) {
                    Some(s) => {
                        out_l += s * v.gain_l;
                        out_r += s * v.gain_r;
                    }
                    None => *v = Voice::OFF,
                }
            }
            if drive > 0.0 {
                out_l = (out_l * drive_gain).tanh() * drive_norm;
                out_r = (out_r * drive_gain).tanh() * drive_norm;
            }
            l[k] += out_l * volume;
            r[k] += out_r * volume;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::InstrumentKind;
    use super::*;

    fn run(p: &[f32; MAX_INST_PARAMS], notes: &[PlayNote], frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut d = Drums::new();
        let positions: Vec<u64> = (0..frames as u64).collect();
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        d.render(p, notes, &positions, &[], 48_000.0, &mut l, &mut r);
        (l, r)
    }

    fn peak(s: &[f32]) -> f32 {
        s.iter().fold(0.0, |m, x| m.max(x.abs()))
    }

    #[test]
    fn every_piece_of_every_kit_sounds_and_dies_away() {
        for kit in 0..KITS.len() {
            for &(name, key) in &PIECES {
                let mut p = InstrumentKind::Drums.defaults();
                p[param::KIT] = kit as f32;
                let (l, r) = run(&p, &[PlayNote { start: 0, end: 4800, key, velocity: 120 }], 48_000 * 4);
                assert!(peak(&l[..9600]) > 0.01, "{name} in kit {kit} is silent");
                assert!(l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 4.0), "{name} kit {kit}");
                assert!(peak(&l[48_000 * 3..]) < 1e-3, "{name} in kit {kit} never stops");
            }
        }
    }

    #[test]
    fn other_keys_are_ignored_and_pan_moves_pieces() {
        let mut p = InstrumentKind::Drums.defaults();
        let (l, _) = run(&p, &[PlayNote { start: 0, end: 100, key: 61, velocity: 100 }], 4800);
        assert_eq!(peak(&l), 0.0);
        p[param::FIRST_PIECE + param::PAN] = -1.0; // kick hard left
        let (l, r) = run(&p, &[PlayNote { start: 0, end: 100, key: 36, velocity: 100 }], 4800);
        assert!(peak(&l) > 0.1 && peak(&r) < 1e-3);
    }

    #[test]
    fn a_closed_hat_chokes_the_open_one() {
        let p = InstrumentKind::Drums.defaults();
        let open = [PlayNote { start: 0, end: 100, key: 46, velocity: 120 }];
        let choked = [open[0], PlayNote { start: 4800, end: 4900, key: 42, velocity: 120 }];
        let (ring, _) = run(&p, &open, 24_000);
        let (cut, _) = run(&p, &choked, 24_000);
        assert!(peak(&cut[14_400..]) < peak(&ring[14_400..]) * 0.2);
    }
}
