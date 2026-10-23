//! Level-reacting effects beyond the compressor: a noise gate and a de-esser.

use super::biquad::{Biquad, Coeffs};
use super::{Context, MAX_PARAMS, db, flush};

/// Seconds to a one-pole smoothing coefficient.
#[inline]
fn coef(ms: f32, rate: f32) -> f32 {
    (-1.0 / (ms.max(0.01) * 0.001 * rate)).exp()
}

/// Silences the quiet parts between notes: hiss, hum, bleed from other mics.
pub struct Gate {
    env: f32,
    gain: f32,
    /// Samples left before a closing gate starts to fall.
    hold: u32,
    open: bool,
    reduction: f32,
}

impl Gate {
    pub fn new() -> Self {
        Self {
            env: 0.0,
            gain: 0.0,
            hold: 0,
            open: false,
            reduction: 0.0,
        }
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let rate = ctx.sample_rate;
        let open_at = db(p[0].clamp(-90.0, 0.0));
        // A few dB of hysteresis, so a note fading around the threshold doesn't chatter.
        let close_at = open_at * db(-4.0);
        let attack = coef(p[1], rate);
        let hold = (p[2].max(0.0) * 0.001 * rate) as u32;
        let release = coef(p[3], rate);
        let floor = db(p[4].clamp(-90.0, 0.0));
        let detect = coef(5.0, rate);
        let mut lowest = 1.0f32;
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let level = a.abs().max(b.abs());
            self.env = if level > self.env {
                level
            } else {
                flush(level + (self.env - level) * detect)
            };
            if self.env > open_at {
                self.open = true;
                self.hold = hold;
            } else if self.env < close_at {
                if self.hold > 0 {
                    self.hold -= 1;
                } else {
                    self.open = false;
                }
            }
            let target = if self.open { 1.0 } else { floor };
            let c = if target > self.gain { attack } else { release };
            self.gain = flush(target + (self.gain - target) * c);
            lowest = lowest.min(self.gain);
            *a *= self.gain;
            *b *= self.gain;
        }
        self.reduction = 20.0 * lowest.max(1e-5).log10();
    }

    pub fn reduction_db(&self) -> f32 {
        self.reduction
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

/// Turns down harsh "s" and "t" sounds in vocals, leaving the rest alone.
/// A Linkwitz-Riley crossover splits the sound at the frequency (the two
/// halves add back up flat), and only the highs are turned down when they
/// get louder than the threshold.
pub struct DeEsser {
    /// Two cascaded Butterworth stages each: a 4th-order low and high half.
    low: [Biquad; 2],
    high: [Biquad; 2],
    env: f32,
    gain: f32,
    last: (f32, f32),
    reduction: f32,
}

impl DeEsser {
    pub fn new() -> Self {
        Self {
            low: [Biquad::new(); 2],
            high: [Biquad::new(); 2],
            env: 0.0,
            gain: 1.0,
            last: (f32::NAN, f32::NAN),
            reduction: 0.0,
        }
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let rate = ctx.sample_rate;
        let freq = p[0].clamp(1500.0, rate * 0.45);
        let threshold = p[1].clamp(-60.0, 0.0);
        let most = p[2].clamp(0.0, 30.0);
        let listen = p[3] >= 0.5;
        if (freq, rate) != self.last {
            const BUTTERWORTH: f32 = std::f32::consts::FRAC_1_SQRT_2;
            for b in &mut self.low {
                b.c = Coeffs::lowpass(freq, BUTTERWORTH, rate);
            }
            for b in &mut self.high {
                b.c = Coeffs::highpass(freq, BUTTERWORTH, rate);
            }
            self.last = (freq, rate);
        }
        let attack = coef(0.5, rate);
        let release = coef(60.0, rate);
        let mut deepest = 0.0f32;
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let [l1, l2] = &mut self.low;
            let [h1, h2] = &mut self.high;
            let (ll, lr) = (l2.tick(0, l1.tick(0, *a)), l2.tick(1, l1.tick(1, *b)));
            let (hl, hr) = (h2.tick(0, h1.tick(0, *a)), h2.tick(1, h1.tick(1, *b)));
            let level = hl.abs().max(hr.abs());
            let c = if level > self.env { attack } else { release };
            self.env = flush(level + (self.env - level) * c);
            let over = 20.0 * self.env.max(1e-6).log10() - threshold;
            // Like a 4:1 compressor on the highs, never more than `most` dB.
            let cut = (over * 0.75).clamp(0.0, most);
            let target = db(-cut);
            self.gain += (target - self.gain) * 0.2;
            deepest = deepest.max(cut);
            if listen {
                *a = hl * self.gain;
                *b = hr * self.gain;
            } else {
                // The lows pass untouched; only the split-off highs are turned down.
                *a = ll + hl * self.gain;
                *b = lr + hr * self.gain;
            }
        }
        self.reduction = -deepest;
    }

    pub fn reduction_db(&self) -> f32 {
        self.reduction
    }

    pub fn reset(&mut self) {
        self.low
            .iter_mut()
            .chain(&mut self.high)
            .for_each(Biquad::reset);
        self.env = 0.0;
        self.gain = 1.0;
        self.reduction = 0.0;
    }
}

impl Default for DeEsser {
    fn default() -> Self {
        Self::new()
    }
}
