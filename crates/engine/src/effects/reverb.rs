//! A studio reverb built on a feedback delay network (FDN): the input is
//! diffused through a chain of all-passes, then circulates through eight
//! delay lines mixed by an energy-preserving Hadamard matrix. Each line loses
//! level so the tail decays in exactly the chosen time, and loses treble so
//! it darkens naturally like a real room. Slowly modulated line lengths keep
//! the tail smooth instead of metallic.

use super::biquad::{Biquad, Coeffs};
use super::{Context, MAX_PARAMS, flush};

const LINES: usize = 8;
/// Delay line lengths in ms at size 100%; chosen to share no common factors.
const LINE_MS: [f32; LINES] = [37.3, 43.1, 49.7, 56.9, 61.3, 71.9, 79.1, 87.7];
/// Input diffusers (ms) and their gains.
const DIFFUSERS: [(f32, f32); 4] = [(4.71, 0.72), (3.59, 0.72), (12.73, 0.62), (9.31, 0.62)];
const MAX_PREDELAY_MS: f32 = 200.0;
/// Modulation depth at 100%, in ms.
const MOD_MS: f32 = 1.2;

/// Parameter indices (see `REVERB_PARAMS`).
pub mod param {
    pub const DECAY: usize = 0;
    pub const SIZE: usize = 1;
    pub const PREDELAY: usize = 2;
    pub const DAMPING: usize = 3;
    pub const LOW_CUT: usize = 4;
    pub const MODULATION: usize = 5;
    pub const WIDTH: usize = 6;
    pub const MIX: usize = 7;
}

struct Line {
    buf: Vec<f32>,
    pos: usize,
    /// One-pole low-pass memory (treble loss per trip).
    lp: f32,
}

struct AllPass {
    buf: Vec<f32>,
    pos: usize,
    gain: f32,
}

impl AllPass {
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let d = self.buf[self.pos];
        let y = d - self.gain * x;
        self.buf[self.pos] = flush(x + self.gain * y);
        self.pos = (self.pos + 1) % self.buf.len();
        y
    }
}

pub struct Reverb {
    lines: [Line; LINES],
    diffusers: Vec<AllPass>,
    predelay: Vec<f32>,
    pre_pos: usize,
    low_cut: Biquad,
    low_cut_freq: f32,
    lfo: f32,
}

/// Hadamard mixing of 8 values, scaled so energy is preserved.
#[inline]
fn hadamard(x: &mut [f32; LINES]) {
    let mut h = 1;
    while h < LINES {
        for i in (0..LINES).step_by(h * 2) {
            for j in i..i + h {
                let (a, b) = (x[j], x[j + h]);
                x[j] = a + b;
                x[j + h] = a - b;
            }
        }
        h *= 2;
    }
    let norm = 1.0 / (LINES as f32).sqrt();
    x.iter_mut().for_each(|v| *v *= norm);
}

impl Reverb {
    pub fn new(rate: f32) -> Self {
        let samples = |ms: f32| ((ms * 0.001 * rate) as usize).max(1);
        Self {
            lines: LINE_MS.map(|ms| Line { buf: vec![0.0; samples(ms + MOD_MS * 2.0) + 4], pos: 0, lp: 0.0 }),
            diffusers: DIFFUSERS.iter().map(|&(ms, gain)| AllPass { buf: vec![0.0; samples(ms)], pos: 0, gain }).collect(),
            predelay: vec![0.0; samples(MAX_PREDELAY_MS) + 1],
            pre_pos: 0,
            low_cut: Biquad::new(),
            low_cut_freq: 0.0,
            lfo: 0.0,
        }
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        use param::*;
        let rate = ctx.sample_rate;
        let decay = p[DECAY].max(0.05);
        let size = 0.25 + 0.75 * (p[SIZE] / 100.0).clamp(0.0, 1.0);
        let wet = (p[MIX] / 100.0).clamp(0.0, 1.0);
        let width = (p[WIDTH] / 100.0).clamp(0.0, 1.0);
        let mod_depth = (p[MODULATION] / 100.0).clamp(0.0, 1.0) * MOD_MS * 0.001 * rate;
        // Treble loss per trip: a one-pole low-pass at the damping frequency.
        let damp = (-std::f32::consts::TAU * p[DAMPING].clamp(500.0, 20_000.0) / rate).exp();
        let pre_len = self.predelay.len();
        let pre = ((p[PREDELAY].clamp(0.0, MAX_PREDELAY_MS) * 0.001 * rate) as usize).min(pre_len - 1);
        let cut = p[LOW_CUT];
        if (cut - self.low_cut_freq).abs() > 0.01 {
            self.low_cut.c = Coeffs::highpass(cut.max(10.0), std::f32::consts::FRAC_1_SQRT_2, rate);
            self.low_cut_freq = cut;
        }

        // Per-line length and loss: each trip loses 60 dB × (length / decay time).
        let mut len = [0f32; LINES];
        let mut gain = [0f32; LINES];
        for i in 0..LINES {
            len[i] = (LINE_MS[i] * size * 0.001 * rate).max(2.0);
            gain[i] = 10f32.powf(-3.0 * len[i] / (decay * rate));
        }
        let lfo_step = std::f32::consts::TAU * 0.7 / rate;
        // Scaled so a long reverb sits at a similar loudness to the dry sound.
        let out_gain = 0.6;

        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            // Pre-delay, then diffusion.
            let input = (*a + *b) * 0.5;
            self.predelay[self.pre_pos] = input;
            let mut x = self.predelay[(self.pre_pos + pre_len - pre) % pre_len];
            self.pre_pos = (self.pre_pos + 1) % pre_len;
            for d in &mut self.diffusers {
                x = d.tick(x);
            }

            // Read every line (fractional, modulated), lose treble and level.
            self.lfo = (self.lfo + lfo_step) % std::f32::consts::TAU;
            let mut taps = [0f32; LINES];
            for (i, line) in self.lines.iter_mut().enumerate() {
                let phase = self.lfo + i as f32 * 0.785;
                let delay = len[i] + mod_depth * (1.0 + phase.sin());
                let n = line.buf.len();
                let back = delay.min((n - 2) as f32);
                let k = back.floor() as usize;
                let frac = back - k as f32;
                let s0 = line.buf[(line.pos + n - k) % n];
                let s1 = line.buf[(line.pos + n - k - 1) % n];
                let s = s0 + (s1 - s0) * frac;
                line.lp = flush(s + (line.lp - s) * damp);
                taps[i] = line.lp * gain[i];
            }

            // Two decorrelated outputs from the lines.
            let mut ol = 0.0;
            let mut or = 0.0;
            for (i, t) in taps.iter().enumerate() {
                if i % 2 == 0 { ol += t } else { or += t }
                if i % 4 < 2 { or += t * 0.3 } else { ol += t * 0.3 }
            }

            // Mix and feed back, with the new input added to every line.
            hadamard(&mut taps);
            for (line, t) in self.lines.iter_mut().zip(taps) {
                let n = line.buf.len();
                line.buf[line.pos] = flush(t + x);
                line.pos = (line.pos + 1) % n;
            }

            let (mut wl, mut wr) = (ol * out_gain, or * out_gain);
            wl = self.low_cut.tick(0, wl);
            wr = self.low_cut.tick(1, wr);
            let mid = (wl + wr) * 0.5;
            let side = (wl - wr) * 0.5 * width;
            *a = *a * (1.0 - wet) + (mid + side) * wet;
            *b = *b * (1.0 - wet) + (mid - side) * wet;
        }
    }

    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.buf.fill(0.0);
            line.lp = 0.0;
        }
        for d in &mut self.diffusers {
            d.buf.fill(0.0);
        }
        self.predelay.fill(0.0);
        self.low_cut.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::super::{EffectKind, tests::CTX};
    use super::param::*;
    use super::*;

    fn tail_energy(decay: f32) -> (f32, f32) {
        let mut rev = Reverb::new(48_000.0);
        let mut p = EffectKind::Reverb.defaults();
        p[MIX] = 100.0;
        p[DECAY] = decay;
        p[PREDELAY] = 0.0;
        let mut energy = |first: bool| {
            let (mut l, mut r) = (vec![0.0f32; 4800], vec![0.0f32; 4800]);
            if first {
                l[0] = 1.0;
                r[0] = 1.0;
            }
            rev.process(&p, &CTX, &mut l, &mut r);
            l.iter().chain(&r).map(|s| s * s).sum::<f32>()
        };
        let early = energy(true) + energy(false);
        // Skip ahead to one decay time after the click.
        let blocks = (decay * 10.0) as usize;
        let mut late = 0.0;
        for _ in 2..blocks {
            late = energy(false);
        }
        (early, late)
    }

    #[test]
    fn the_tail_falls_about_60_db_in_the_decay_time() {
        let (early, late) = tail_energy(1.0);
        assert!(early > 1e-3, "a tail starts");
        let drop_db = 10.0 * (late / early).log10();
        assert!(drop_db < -40.0 && drop_db > -80.0, "{drop_db} dB");
    }

    #[test]
    fn longer_decay_rings_longer() {
        let (e1, l1) = tail_energy(1.0);
        let (e2, l2) = tail_energy(1.0 * 1.0);
        assert!((l1 / e1 - l2 / e2).abs() < 1e-9, "deterministic");
        let mut rev = Reverb::new(48_000.0);
        let mut p = EffectKind::Reverb.defaults();
        p[MIX] = 100.0;
        let mut ring = |decay: f32| {
            rev.reset();
            p[DECAY] = decay;
            let (mut l, mut r) = (vec![0.0f32; 48_000], vec![0.0f32; 48_000]);
            l[0] = 1.0;
            rev.process(&p, &CTX, &mut l, &mut r);
            l[40_000..].iter().map(|s| s * s).sum::<f32>()
        };
        assert!(ring(4.0) > ring(0.5) * 100.0);
    }
}
