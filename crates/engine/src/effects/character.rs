//! Effects that change a sound's character: distortion (five kinds of
//! saturation) and a bitcrusher (fewer bits, lower sample rate).

use super::biquad::{Biquad, Coeffs};
use super::{Context, MAX_PARAMS, db, flush};

pub const DISTORTION_TYPES: &[&str] = &["Soft", "Hard", "Fuzz", "Tube", "Fold"];

/// Saturation from gentle warmth to broken-speaker fuzz.
pub struct Distortion {
    tone: Biquad,
    /// Blocks the offset asymmetric shapes add.
    dc: Biquad,
    last: (f32, f32),
}

impl Distortion {
    pub fn new() -> Self {
        Self {
            tone: Biquad::new(),
            dc: Biquad::new(),
            last: (f32::NAN, f32::NAN),
        }
    }

    #[inline]
    fn shape(kind: u32, x: f32) -> f32 {
        match kind {
            1 => x.clamp(-1.0, 1.0),
            2 => {
                // Fuzz: squares off quickly, a little lopsided.
                let y = x.signum() * (1.0 - (-x.abs() * 2.5).exp());
                if y > 0.0 { y } else { y * 0.8 }
            }
            3 => {
                // Tube: softer on one side than the other, which adds warm even harmonics.
                const BIAS: f32 = 0.25;
                (x + BIAS).tanh() - BIAS.tanh()
            }
            4 => (x * std::f32::consts::FRAC_PI_2).sin(),
            _ => x.tanh(),
        }
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let drive = db(p[0].clamp(0.0, 48.0));
        let kind = p[1].round() as u32;
        let tone = p[2].clamp(500.0, 20_000.0);
        let out = db(p[3].clamp(-24.0, 12.0));
        let mix = (p[4] / 100.0).clamp(0.0, 1.0);
        if (tone, ctx.sample_rate) != self.last {
            self.tone.c = Coeffs::lowpass(tone.min(ctx.sample_rate * 0.45), 0.707, ctx.sample_rate);
            self.dc.c = Coeffs::highpass1(12.0, ctx.sample_rate);
            self.last = (tone, ctx.sample_rate);
        }
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            for (ch, x) in [a, b].into_iter().enumerate() {
                let wet = self
                    .dc
                    .tick(ch, self.tone.tick(ch, Self::shape(kind, *x * drive)));
                *x = flush((*x + (wet - *x) * mix) * out);
            }
        }
    }

    pub fn reset(&mut self) {
        self.tone.reset();
        self.dc.reset();
    }
}

impl Default for Distortion {
    fn default() -> Self {
        Self::new()
    }
}

/// Lo-fi: fewer bits (grit) and a lower sample rate (aliasing whine).
pub struct Bitcrusher {
    held: [f32; 2],
    /// How far through the current held sample, in output samples.
    count: f32,
}

impl Bitcrusher {
    pub fn new() -> Self {
        Self {
            held: [0.0; 2],
            count: f32::MAX,
        }
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let bits = p[0].clamp(1.0, 16.0);
        let levels = 2f32.powf(bits - 1.0);
        let hold = (ctx.sample_rate / p[1].clamp(200.0, ctx.sample_rate)).max(1.0);
        let mix = (p[2] / 100.0).clamp(0.0, 1.0);
        let out = db(p[3].clamp(-24.0, 12.0));
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            self.count += 1.0;
            if self.count >= hold {
                self.count -= hold;
                if self.count >= hold {
                    self.count = 0.0;
                }
                self.held = [
                    (*a * levels).round() / levels,
                    (*b * levels).round() / levels,
                ];
            }
            *a = (*a + (self.held[0] - *a) * mix) * out;
            *b = (*b + (self.held[1] - *b) * mix) * out;
        }
    }

    pub fn reset(&mut self) {
        self.held = [0.0; 2];
        self.count = f32::MAX;
    }
}

impl Default for Bitcrusher {
    fn default() -> Self {
        Self::new()
    }
}
