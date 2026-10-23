//! Moving effects: chorus, flanger and phaser. Each sweeps a short delay (or
//! a chain of all-pass filters) with a slow LFO; left and right sweep out of
//! step, which spreads the sound across the stereo field.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::{Context, MAX_PARAMS, flush};

/// A short stereo delay line read at a fractional position.
struct Line {
    buf: [Vec<f32>; 2],
    pos: usize,
}

impl Line {
    fn new(seconds: f32, rate: f32) -> Self {
        let len = (seconds * rate) as usize + 4;
        Self {
            buf: [vec![0.0; len], vec![0.0; len]],
            pos: 0,
        }
    }

    /// The sample `delay` samples ago on channel `ch` (linear interpolation).
    #[inline]
    fn read(&self, ch: usize, delay: f32) -> f32 {
        let buf = &self.buf[ch];
        let len = buf.len();
        let back = delay.clamp(1.0, (len - 3) as f32);
        let i = back as usize;
        let frac = back - i as f32;
        let a = buf[(self.pos + len - i) % len];
        let b = buf[(self.pos + len - i - 1) % len];
        a + (b - a) * frac
    }

    #[inline]
    fn write(&mut self, l: f32, r: f32) {
        self.buf[0][self.pos] = flush(l);
        self.buf[1][self.pos] = flush(r);
        self.pos = (self.pos + 1) % self.buf[0].len();
    }

    fn reset(&mut self) {
        self.buf.iter_mut().for_each(|b| b.fill(0.0));
    }
}

/// Thickens a sound with slightly detuned, delayed copies of itself.
pub struct Chorus {
    line: Line,
    phase: f32,
}

impl Chorus {
    pub fn new(rate: f32) -> Self {
        Self {
            line: Line::new(0.06, rate),
            phase: 0.0,
        }
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let sr = ctx.sample_rate;
        let step = TAU * p[0].clamp(0.01, 10.0) / sr;
        let depth = (p[1] / 100.0).clamp(0.0, 1.0) * 0.006 * sr;
        let base = p[2].clamp(1.0, 40.0) * 0.001 * sr;
        let width = (p[3] / 100.0).clamp(0.0, 1.0);
        let mix = (p[4] / 100.0).clamp(0.0, 1.0);
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            // Two taps per side, a quarter-cycle apart; the sides opposite each other.
            let tap = |offset: f32| base + depth * (0.5 + 0.5 * (self.phase + offset).sin());
            let wl = 0.5 * (self.line.read(0, tap(0.0)) + self.line.read(0, tap(FRAC_PI_2)));
            let wr = 0.5
                * (self.line.read(1, tap(PI * width))
                    + self.line.read(1, tap(PI * width + FRAC_PI_2)));
            self.line.write(*a, *b);
            *a += (wl - *a) * mix;
            *b += (wr - *b) * mix;
            self.phase = (self.phase + step) % TAU;
        }
    }

    pub fn reset(&mut self) {
        self.line.reset();
    }
}

/// A sweeping comb filter: the jet-plane whoosh.
pub struct Flanger {
    line: Line,
    phase: f32,
}

impl Flanger {
    pub fn new(rate: f32) -> Self {
        Self {
            line: Line::new(0.03, rate),
            phase: 0.0,
        }
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let sr = ctx.sample_rate;
        let step = TAU * p[0].clamp(0.01, 10.0) / sr;
        let depth = (p[1] / 100.0).clamp(0.0, 1.0);
        let base = p[2].clamp(0.1, 15.0) * 0.001 * sr;
        let feedback = (p[3] / 100.0).clamp(-0.95, 0.95);
        let mix = (p[4] / 100.0).clamp(0.0, 1.0);
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let sweep = |offset: f32| {
                (base * (1.0 - depth * 0.5 * (1.0 + (self.phase + offset).sin()))).max(1.0)
            };
            let wl = self.line.read(0, sweep(0.0));
            let wr = self.line.read(1, sweep(FRAC_PI_2));
            self.line.write(*a + wl * feedback, *b + wr * feedback);
            // At 100 % an equal blend of dry and delayed, like a flanger pedal.
            *a += (0.5 * (*a + wl) - *a) * mix;
            *b += (0.5 * (*b + wr) - *b) * mix;
            self.phase = (self.phase + step) % TAU;
        }
    }

    pub fn reset(&mut self) {
        self.line.reset();
    }
}

/// Stages a phaser can have.
pub const PHASER_STAGES: &[&str] = &["4", "6", "8", "12"];
const STAGE_COUNTS: [usize; 4] = [4, 6, 8, 12];
const MAX_STAGES: usize = 12;
/// The all-pass coefficient is updated every this many samples.
const PHASER_CONTROL: usize = 8;

/// Sweeping notches from a chain of all-pass filters.
pub struct Phaser {
    /// First-order all-pass memory: (x1, y1) per stage, per side.
    state: [[(f32, f32); MAX_STAGES]; 2],
    phase: f32,
    fb: [f32; 2],
    coef: [f32; 2],
    tick: usize,
}

impl Phaser {
    pub fn new() -> Self {
        Self {
            state: [[(0.0, 0.0); MAX_STAGES]; 2],
            phase: 0.0,
            fb: [0.0; 2],
            coef: [0.0; 2],
            tick: 0,
        }
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let sr = ctx.sample_rate;
        let step = TAU * p[0].clamp(0.01, 10.0) / sr;
        let depth = (p[1] / 100.0).clamp(0.0, 1.0) * 2.0; // ± octaves
        let centre = p[2].clamp(50.0, 8000.0);
        let feedback = (p[3] / 100.0).clamp(0.0, 0.95);
        let stages = STAGE_COUNTS[(p[4].round() as usize).min(STAGE_COUNTS.len() - 1)];
        let mix = (p[5] / 100.0).clamp(0.0, 1.0);
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            if self.tick == 0 {
                for (ch, c) in self.coef.iter_mut().enumerate() {
                    let f = (centre
                        * 2f32.powf(depth * (self.phase + ch as f32 * FRAC_PI_2).sin()))
                    .clamp(20.0, sr * 0.45);
                    let t = (PI * f / sr).tan();
                    *c = (t - 1.0) / (t + 1.0);
                }
            }
            self.tick = (self.tick + 1) % PHASER_CONTROL;
            for (ch, x) in [a, b].into_iter().enumerate() {
                let c = self.coef[ch];
                let mut y = *x + self.fb[ch] * feedback;
                for (x1, y1) in self.state[ch].iter_mut().take(stages) {
                    let out = c * y + *x1 - c * *y1;
                    *x1 = y;
                    *y1 = flush(out);
                    y = out;
                }
                self.fb[ch] = flush(y);
                // Dry plus the shifted copy makes the notches.
                *x += (0.5 * (*x + y) - *x) * mix;
            }
            self.phase = (self.phase + step) % TAU;
        }
    }

    pub fn reset(&mut self) {
        self.state = [[(0.0, 0.0); MAX_STAGES]; 2];
        self.fb = [0.0; 2];
    }
}

impl Default for Phaser {
    fn default() -> Self {
        Self::new()
    }
}
