//! A studio compressor: soft knee, linked stereo detection (peak or RMS), a
//! sidechain high-pass so bass doesn't pump the whole mix, automatic makeup
//! gain, and a dry/wet mix for parallel compression.

use super::biquad::{Biquad, Coeffs};
use super::{Context, MAX_PARAMS, db};

/// Parameter indices (see `COMPRESSOR_PARAMS`).
pub mod param {
    pub const THRESHOLD: usize = 0;
    pub const RATIO: usize = 1;
    pub const ATTACK: usize = 2;
    pub const RELEASE: usize = 3;
    pub const KNEE: usize = 4;
    pub const MAKEUP: usize = 5;
    pub const AUTO_MAKEUP: usize = 6;
    pub const DETECT: usize = 7;
    pub const SC_HPF: usize = 8;
    pub const MIX: usize = 9;
}

pub struct Compressor {
    /// Current gain reduction in dB (≤ 0), smoothed by attack and release.
    reduction: f32,
    /// Deepest reduction during the last block, for the meter.
    block_reduction: f32,
    /// Running mean square of the detector signal (RMS mode).
    mean_square: f32,
    sidechain: Biquad,
    sc_freq: f32,
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new()
    }
}

impl Compressor {
    pub fn new() -> Self {
        Self { reduction: 0.0, block_reduction: 0.0, mean_square: 0.0, sidechain: Biquad::new(), sc_freq: 0.0 }
    }

    /// Gain change in dB for an input level, from the static curve.
    pub fn curve(level_db: f32, threshold: f32, ratio: f32, knee: f32) -> f32 {
        let over = level_db - threshold;
        let slope = 1.0 / ratio.max(1.0) - 1.0;
        if knee > 0.0 && over.abs() <= knee / 2.0 {
            slope * (over + knee / 2.0).powi(2) / (2.0 * knee)
        } else if over > 0.0 {
            slope * over
        } else {
            0.0
        }
    }

    /// Automatic makeup: brings back about half the reduction a full-scale
    /// signal would get, so the result sounds about as loud.
    pub fn auto_makeup(threshold: f32, ratio: f32, knee: f32) -> f32 {
        -Self::curve(0.0, threshold, ratio, knee) * 0.5
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        use param::*;
        let (threshold, ratio, knee) = (p[THRESHOLD], p[RATIO], p[KNEE]);
        let coeff = |ms: f32| (-1.0 / (ms.max(0.01) * 0.001 * ctx.sample_rate)).exp();
        let (att, rel) = (coeff(p[ATTACK]), coeff(p[RELEASE]));
        let auto = if p[AUTO_MAKEUP] >= 0.5 { Self::auto_makeup(threshold, ratio, knee) } else { 0.0 };
        let makeup = db(p[MAKEUP] + auto);
        let wet = (p[MIX] / 100.0).clamp(0.0, 1.0);
        let rms = p[DETECT] >= 0.5;
        // ~10 ms averaging for RMS detection.
        let rms_coef = coeff(10.0);
        let sc = p[SC_HPF];
        let use_sc = sc > 20.5;
        if use_sc && (sc - self.sc_freq).abs() > 0.01 {
            self.sidechain.c = Coeffs::highpass(sc, std::f32::consts::FRAC_1_SQRT_2, ctx.sample_rate);
            self.sc_freq = sc;
        }

        self.block_reduction = 0.0;
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let (mut dl, mut dr) = (*a, *b);
            if use_sc {
                dl = self.sidechain.tick(0, dl);
                dr = self.sidechain.tick(1, dr);
            }
            let level = if rms {
                let sq = (dl * dl).max(dr * dr);
                self.mean_square = sq + (self.mean_square - sq) * rms_coef;
                self.mean_square.sqrt()
            } else {
                dl.abs().max(dr.abs())
            };
            let level_db = 20.0 * level.max(1e-6).log10();
            let target = Self::curve(level_db, threshold, ratio, knee);
            // Reduction grows at the attack rate and recovers at the release rate.
            let c = if target < self.reduction { att } else { rel };
            self.reduction = target + (self.reduction - target) * c;
            self.block_reduction = self.block_reduction.min(self.reduction);
            let g = db(self.reduction) * makeup;
            let mixed = wet * g + (1.0 - wet);
            *a *= mixed;
            *b *= mixed;
        }
    }

    /// Deepest gain reduction in the last block, in dB (≤ 0).
    pub fn reduction_db(&self) -> f32 {
        self.block_reduction
    }

    pub fn reset(&mut self) {
        self.reduction = 0.0;
        self.block_reduction = 0.0;
        self.mean_square = 0.0;
        self.sidechain.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::super::{EffectKind, tests::CTX};
    use super::param::*;
    use super::*;

    #[test]
    fn static_curve_follows_threshold_and_ratio() {
        assert_eq!(Compressor::curve(-30.0, -20.0, 4.0, 0.0), 0.0);
        assert!((Compressor::curve(-8.0, -20.0, 4.0, 0.0) + 9.0).abs() < 1e-5, "12 dB over at 4:1 → 3 dB over");
        assert!(Compressor::curve(-21.0, -20.0, 4.0, 6.0) < 0.0, "soft knee starts early");
        assert!((Compressor::auto_makeup(-20.0, 4.0, 0.0) - 7.5).abs() < 1e-5);
    }

    #[test]
    fn loud_steady_signal_settles_at_the_curve() {
        let mut c = Compressor::new();
        let mut p = EffectKind::Compressor.defaults();
        p[THRESHOLD] = -20.0;
        p[RATIO] = 4.0;
        p[KNEE] = 0.0;
        for _ in 0..200 {
            let (mut l, mut r) = (vec![db(-8.0); 512], vec![db(-8.0); 512]);
            c.process(&p, &CTX, &mut l, &mut r);
        }
        assert!((c.reduction_db() + 9.0).abs() < 0.1, "{}", c.reduction_db());
    }

    #[test]
    fn sidechain_filter_ignores_bass() {
        let mut p = EffectKind::Compressor.defaults();
        p[THRESHOLD] = -20.0;
        p[KNEE] = 0.0;
        let bass: Vec<f32> = (0..512).map(|i| 0.5 * (i as f32 * 40.0 * std::f32::consts::TAU / 48_000.0).sin()).collect();
        let run = |sc: f32| {
            let mut c = Compressor::new();
            let mut p = p;
            p[SC_HPF] = sc;
            for _ in 0..100 {
                let (mut l, mut r) = (bass.clone(), bass.clone());
                c.process(&p, &CTX, &mut l, &mut r);
            }
            c.reduction_db()
        };
        assert!(run(20.0) < -5.0, "bass compresses without the filter");
        assert!(run(300.0) > run(20.0) + 5.0, "much less with it");
    }
}
