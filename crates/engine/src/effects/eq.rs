//! Parametric EQ with eight bands, laid out like a studio EQ:
//! low cut · low shelf · four bells · high shelf · high cut.
//!
//! Every band can be switched on or off; the cuts have selectable slopes
//! (6, 12, 24 or 48 dB per octave, Butterworth). Parameter order is the
//! same as `EQ_PARAMS` in `mod.rs`.

use super::biquad::{Biquad, Coeffs};
use super::{Context, MAX_PARAMS, db};

/// Filter stages: 4 for the low cut, 1 shelf, 4 bells, 1 shelf, 4 for the high cut.
const SLOTS: usize = 14;
const LOW_CUT: usize = 0;
const LOW_SHELF: usize = 4;
const BELLS: usize = 5;
const HIGH_SHELF: usize = 9;
const HIGH_CUT: usize = 10;

/// Butterworth stage Qs for each slope (12, 24 and 48 dB/oct; 6 is first order).
const BUTTERWORTH: [&[f32]; 3] = [&[std::f32::consts::FRAC_1_SQRT_2], &[0.541_196_1, 1.306_563], &[0.509_795_6, 0.601_344_9, 0.899_976_2, 2.562_915_4]];

/// Parameter indices (see `EQ_PARAMS`).
pub mod param {
    pub const LC_ON: usize = 0;
    pub const LC_FREQ: usize = 1;
    pub const LC_SLOPE: usize = 2;
    pub const LS_ON: usize = 3;
    pub const LS_FREQ: usize = 4;
    pub const LS_GAIN: usize = 5;
    pub const LS_Q: usize = 6;
    /// Bell `i` (0..4): on, freq, gain, q at `BELL + 4 * i`.
    pub const BELL: usize = 7;
    pub const HS_ON: usize = 23;
    pub const HS_FREQ: usize = 24;
    pub const HS_GAIN: usize = 25;
    pub const HS_Q: usize = 26;
    pub const HC_ON: usize = 27;
    pub const HC_FREQ: usize = 28;
    pub const HC_SLOPE: usize = 29;
    pub const OUTPUT: usize = 30;
}

pub struct Eq {
    stages: [Biquad; SLOTS],
    active: [bool; SLOTS],
    last: [f32; MAX_PARAMS],
    last_rate: f32,
}

impl Default for Eq {
    fn default() -> Self {
        Self::new()
    }
}

/// The filters for these settings, one optional stage per slot.
fn design(p: &[f32; MAX_PARAMS], rate: f32) -> [Option<Coeffs>; SLOTS] {
    use param::*;
    let mut out = [None; SLOTS];
    let on = |i: usize| p[i] >= 0.5;
    let cut = |out: &mut [Option<Coeffs>; SLOTS], first: usize, freq: f32, slope: f32, high: bool| {
        let slope = slope.round().clamp(0.0, 3.0) as usize;
        if slope == 0 {
            out[first] = Some(if high { Coeffs::highpass1(freq, rate) } else { Coeffs::lowpass1(freq, rate) });
        } else {
            for (k, &q) in BUTTERWORTH[slope - 1].iter().enumerate() {
                out[first + k] = Some(if high { Coeffs::highpass(freq, q, rate) } else { Coeffs::lowpass(freq, q, rate) });
            }
        }
    };
    if on(LC_ON) {
        cut(&mut out, LOW_CUT, p[LC_FREQ], p[LC_SLOPE], true);
    }
    if on(LS_ON) && p[LS_GAIN].abs() > 0.01 {
        out[LOW_SHELF] = Some(Coeffs::low_shelf(p[LS_FREQ], p[LS_GAIN], p[LS_Q], rate));
    }
    for i in 0..4 {
        let b = BELL + 4 * i;
        if on(b) && p[b + 2].abs() > 0.01 {
            out[BELLS + i] = Some(Coeffs::peak(p[b + 1], p[b + 2], p[b + 3].max(0.05), rate));
        }
    }
    if on(HS_ON) && p[HS_GAIN].abs() > 0.01 {
        out[HIGH_SHELF] = Some(Coeffs::high_shelf(p[HS_FREQ], p[HS_GAIN], p[HS_Q], rate));
    }
    if on(HC_ON) {
        cut(&mut out, HIGH_CUT, p[HC_FREQ], p[HC_SLOPE], false);
    }
    out
}

impl Eq {
    pub fn new() -> Self {
        Self { stages: [Biquad::new(); SLOTS], active: [false; SLOTS], last: [f32::NAN; MAX_PARAMS], last_rate: 0.0 }
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        if *p != self.last || ctx.sample_rate != self.last_rate {
            for (slot, c) in design(p, ctx.sample_rate).into_iter().enumerate() {
                match c {
                    // Keep the filter's memory when only its settings move, so turning a knob never clicks.
                    Some(c) => {
                        if !self.active[slot] {
                            self.stages[slot].reset();
                        }
                        self.stages[slot].c = c;
                        self.active[slot] = true;
                    }
                    None => self.active[slot] = false,
                }
            }
            self.last = *p;
            self.last_rate = ctx.sample_rate;
        }
        for (stage, _) in self.stages.iter_mut().zip(self.active).filter(|(_, on)| *on) {
            stage.process(l, r);
        }
        let out = p[param::OUTPUT];
        if out.abs() > 0.01 {
            let g = db(out);
            l.iter_mut().chain(r.iter_mut()).for_each(|s| *s *= g);
        }
    }

    /// Each band's own curve in dB at `freqs` (0 low cut, 1 low shelf, 2–5
    /// the bells, 6 high shelf, 7 high cut); `None` while a band is off or flat.
    pub fn band_curves(p: &[f32; MAX_PARAMS], freqs: &[f32], rate: f32) -> [Option<Vec<f32>>; 8] {
        let design = design(p, rate);
        let ranges = [LOW_CUT..LOW_SHELF, LOW_SHELF..BELLS, BELLS..BELLS + 1, BELLS + 1..BELLS + 2, BELLS + 2..BELLS + 3, BELLS + 3..HIGH_SHELF, HIGH_SHELF..HIGH_CUT, HIGH_CUT..SLOTS];
        ranges.map(|slots| {
            let stages: Vec<Coeffs> = design[slots].iter().flatten().copied().collect();
            (!stages.is_empty()).then(|| freqs.iter().map(|&f| stages.iter().map(|c| c.response_db(f, rate)).sum()).collect())
        })
    }

    /// The EQ's overall gain in dB at `freq`, for drawing its curve.
    pub fn response_db(p: &[f32; MAX_PARAMS], freq: f32, rate: f32) -> f32 {
        design(p, rate).iter().flatten().map(|c| c.response_db(freq, rate)).sum::<f32>() + p[param::OUTPUT]
    }

    pub fn reset(&mut self) {
        for s in &mut self.stages {
            s.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{EffectKind, tests::CTX};
    use super::param::*;
    use super::*;

    #[test]
    fn flat_by_default_and_passes_sound_unchanged() {
        let p = EffectKind::Eq.defaults();
        assert!(Eq::response_db(&p, 1000.0, 48_000.0).abs() < 1e-4);
        let mut eq = Eq::new();
        let (mut l, mut r) = (vec![0.3; 64], vec![-0.2; 64]);
        eq.process(&p, &CTX, &mut l, &mut r);
        assert!(l.iter().all(|&s| s == 0.3) && r.iter().all(|&s| s == -0.2));
    }

    #[test]
    fn bands_shape_the_curve() {
        let mut p = EffectKind::Eq.defaults();
        p[BELL] = 1.0; // bell 1 on
        p[BELL + 1] = 1000.0;
        p[BELL + 2] = 6.0;
        assert!((Eq::response_db(&p, 1000.0, 48_000.0) - 6.0).abs() < 0.05);
        // A steeper low cut takes more off an octave below.
        p[LC_ON] = 1.0;
        p[LC_FREQ] = 200.0;
        p[LC_SLOPE] = 1.0;
        let gentle = Eq::response_db(&p, 100.0, 48_000.0);
        p[LC_SLOPE] = 3.0;
        let steep = Eq::response_db(&p, 100.0, 48_000.0);
        assert!((gentle + 12.3).abs() < 1.0 && (steep + 48.0).abs() < 2.0, "{gentle} {steep}");
        p[OUTPUT] = -3.0;
        assert!((Eq::response_db(&p, 1000.0, 48_000.0) - 3.0).abs() < 0.1);
    }

    #[test]
    fn band_curves_add_up_to_the_whole() {
        let mut p = EffectKind::Eq.defaults();
        p[BELL] = 1.0;
        p[BELL + 2] = 4.0;
        p[LC_ON] = 1.0;
        let freqs = [50.0, 1000.0, 5000.0];
        let bands = Eq::band_curves(&p, &freqs, 48_000.0);
        assert!(bands[1].is_none() && bands[3].is_none(), "flat or off bands have no curve");
        for (i, &f) in freqs.iter().enumerate() {
            let sum: f32 = bands.iter().flatten().map(|c| c[i]).sum();
            assert!((sum - Eq::response_db(&p, f, 48_000.0)).abs() < 1e-3);
        }
    }

    #[test]
    fn low_cut_removes_rumble() {
        let mut eq = Eq::new();
        let mut p = EffectKind::Eq.defaults();
        p[LC_ON] = 1.0;
        p[LC_FREQ] = 200.0;
        let sine: Vec<f32> = (0..48_000).map(|i| (i as f32 * 30.0 * std::f32::consts::TAU / 48_000.0).sin()).collect();
        let (mut l, mut r) = (sine.clone(), sine);
        for (cl, cr) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            eq.process(&p, &CTX, cl, cr);
        }
        let peak = l[24_000..].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak < 0.02, "{peak}");
    }
}
