//! The mastering assistant: measures a mix and suggests a master chain (EQ,
//! a gentle compressor and a limiter) that brings it to a target loudness.
//!
//! Loudness is measured like streaming services do (ITU-R BS.1770, in LUFS):
//! K-weighted, in 400 ms blocks, with silence and quiet passages gated out.

use isai_engine::effects::{Context, Effect, EffectKind, MAX_PARAMS};

/// What the mix sounds like now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Analysis {
    /// Integrated loudness, LUFS.
    pub lufs: f32,
    /// Loudest sample, dBFS.
    pub peak_db: f32,
    /// Share of the energy below 200 Hz and above 4 kHz, in dB of the whole.
    pub low_db: f32,
    pub high_db: f32,
}

/// The suggested master chain, and how the mix measures through it.
#[derive(Clone, Debug)]
pub struct Plan {
    pub before: Analysis,
    pub after: Analysis,
    pub eq: Vec<(&'static str, f32)>,
    pub compressor: Vec<(&'static str, f32)>,
    pub limiter: Vec<(&'static str, f32)>,
}

/// Roughly where a balanced, finished mix sits (energy share, dB).
const LOW_REF: f32 = -4.0;
const HIGH_REF: f32 = -12.0;
/// The limiter's ceiling, leaving room for lossy encoding.
const CEILING: f32 = -1.0;

/// A simple biquad for measuring (the engine's own is private to it).
#[derive(Clone, Copy)]
struct Bq {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Bq {
    fn new(b: [f64; 3], a: [f64; 3]) -> Self {
        Self {
            b: [b[0] / a[0], b[1] / a[0], b[2] / a[0]],
            a: [a[1] / a[0], a[2] / a[0]],
            z: [0.0; 2],
        }
    }

    fn high_shelf(f0: f64, gain_db: f64, q: f64, rate: f64) -> Self {
        let a = 10f64.powf(gain_db / 40.0);
        let w = std::f64::consts::TAU * f0 / rate;
        let alpha = w.sin() / (2.0 * q);
        let c = w.cos();
        let s = 2.0 * a.sqrt() * alpha;
        Self::new(
            [
                a * ((a + 1.0) + (a - 1.0) * c + s),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
                a * ((a + 1.0) + (a - 1.0) * c - s),
            ],
            [
                (a + 1.0) - (a - 1.0) * c + s,
                2.0 * ((a - 1.0) - (a + 1.0) * c),
                (a + 1.0) - (a - 1.0) * c - s,
            ],
        )
    }

    fn high_pass(f0: f64, q: f64, rate: f64) -> Self {
        let w = std::f64::consts::TAU * f0 / rate;
        let alpha = w.sin() / (2.0 * q);
        let c = w.cos();
        Self::new(
            [(1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0],
            [1.0 + alpha, -2.0 * c, 1.0 - alpha],
        )
    }

    fn low_pass(f0: f64, q: f64, rate: f64) -> Self {
        let w = std::f64::consts::TAU * f0 / rate;
        let alpha = w.sin() / (2.0 * q);
        let c = w.cos();
        Self::new(
            [(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0],
            [1.0 + alpha, -2.0 * c, 1.0 - alpha],
        )
    }

    #[inline]
    fn tick(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// Integrated loudness of stereo interleaved audio, in LUFS (−70 for silence).
pub fn loudness(audio: &[f32], rate: u32) -> f32 {
    let r = f64::from(rate);
    // K-weighting: a shelf for the head's effect, then a gentle high-pass.
    let mut k = [[
        Bq::high_shelf(1681.97, 4.0, 0.7072, r),
        Bq::high_pass(38.13, 0.5003, r),
    ]; 2];
    let frames = audio.len() / 2;
    let step = (rate as usize) / 10; // 100 ms
    let mut sums: Vec<f64> = Vec::with_capacity(frames / step + 1);
    let mut acc = 0.0;
    for (i, f) in audio.as_chunks::<2>().0.iter().enumerate() {
        for (ch, &x) in f.iter().enumerate() {
            let shelved = k[ch][0].tick(f64::from(x));
            let y = k[ch][1].tick(shelved);
            acc += y * y;
        }
        if (i + 1) % step == 0 {
            sums.push(acc);
            acc = 0.0;
        }
    }
    // 400 ms blocks, 75 % overlap: four 100 ms steps each.
    let blocks: Vec<f64> = sums
        .windows(4)
        .map(|w| w.iter().sum::<f64>() / (4 * step) as f64)
        .collect();
    let lufs = |ms: f64| -0.691 + 10.0 * ms.max(1e-12).log10();
    let loud: Vec<f64> = blocks
        .iter()
        .copied()
        .filter(|&ms| lufs(ms) > -70.0)
        .collect();
    if loud.is_empty() {
        return -70.0;
    }
    let mean = loud.iter().sum::<f64>() / loud.len() as f64;
    let gate = lufs(mean) - 10.0;
    let kept: Vec<f64> = loud.into_iter().filter(|&ms| lufs(ms) > gate).collect();
    lufs(kept.iter().sum::<f64>() / kept.len().max(1) as f64) as f32
}

pub fn analyse(audio: &[f32], rate: u32) -> Analysis {
    let r = f64::from(rate);
    let peak = audio.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let mut low = Bq::low_pass(200.0, 0.707, r);
    let mut high = Bq::high_pass(4000.0, 0.707, r);
    let (mut total, mut lo, mut hi) = (0.0f64, 0.0f64, 0.0f64);
    for f in audio.as_chunks::<2>().0 {
        let m = f64::from(f[0] + f[1]) * 0.5;
        total += m * m;
        let l = low.tick(m);
        let h = high.tick(m);
        lo += l * l;
        hi += h * h;
    }
    let share = |x: f64| (10.0 * (x / total.max(1e-12)).max(1e-9).log10()) as f32;
    Analysis {
        lufs: loudness(audio, rate),
        peak_db: 20.0 * peak.max(1e-6).log10(),
        low_db: share(lo),
        high_db: share(hi),
    }
}

/// Sets named parameters on an effect's defaults.
fn params(kind: EffectKind, set: &[(&str, f32)]) -> [f32; MAX_PARAMS] {
    let mut p = kind.defaults();
    for (key, v) in set {
        if let Some(i) = kind.params().iter().position(|d| d.key == *key) {
            p[i] = *v;
        }
    }
    p
}

/// Runs a chain of effects over stereo interleaved audio.
fn run(chain: &[(EffectKind, [f32; MAX_PARAMS])], audio: &[f32], rate: u32) -> Vec<f32> {
    let ctx = Context {
        sample_rate: rate as f32,
        bpm: 120.0,
    };
    let mut fx: Vec<Effect> = chain.iter().map(|(k, _)| Effect::new(*k, rate)).collect();
    let mut out = Vec::with_capacity(audio.len());
    let (mut l, mut r) = (Vec::with_capacity(512), Vec::with_capacity(512));
    for block in audio.chunks(1024) {
        l.clear();
        r.clear();
        for f in block.as_chunks::<2>().0 {
            l.push(f[0]);
            r.push(f[1]);
        }
        for (e, (_, p)) in fx.iter_mut().zip(chain) {
            e.process(p, &ctx, &mut l, &mut r);
        }
        for (a, b) in l.iter().zip(&r) {
            out.push(*a);
            out.push(*b);
        }
    }
    out
}

/// A master chain bringing `audio` (the mix, before any master effects) to
/// `target` LUFS. `cancel` is checked between passes.
pub fn plan(audio: &[f32], rate: u32, target: f32, cancel: &dyn Fn() -> bool) -> Option<Plan> {
    let before = analyse(audio, rate);
    // Tone: gentle shelves halfway towards a balanced mix, never more than 3 dB.
    let low_gain = ((LOW_REF - before.low_db) * 0.5).clamp(-3.0, 3.0);
    let high_gain = ((HIGH_REF - before.high_db) * 0.5).clamp(-3.0, 3.0);
    let round = |x: f32| (x * 2.0).round() / 2.0;
    let eq = vec![
        ("lc_on", 1.0),
        ("lc_freq", 25.0),
        ("lc_slope", 1.0),
        ("ls_on", if round(low_gain) != 0.0 { 1.0 } else { 0.0 }),
        ("ls_freq", 120.0),
        ("ls_gain", round(low_gain)),
        ("hs_on", if round(high_gain) != 0.0 { 1.0 } else { 0.0 }),
        ("hs_freq", 9000.0),
        ("hs_gain", round(high_gain)),
    ];
    // Glue: 2:1, slow enough to keep the punch, touching only the loud parts.
    let threshold = (before.lufs + 4.0).clamp(-40.0, -6.0).round();
    let compressor = vec![
        ("threshold", threshold),
        ("ratio", 2.0),
        ("attack", 30.0),
        ("release", 150.0),
        ("knee", 6.0),
        ("auto_makeup", 0.0),
        ("makeup", 0.0),
        ("mix", 100.0),
    ];
    let shaped = run(
        &[
            (EffectKind::Eq, params(EffectKind::Eq, &eq)),
            (
                EffectKind::Compressor,
                params(EffectKind::Compressor, &compressor),
            ),
        ],
        audio,
        rate,
    );
    // The limiter's gain: measure, adjust, and measure again.
    let mut gain = (target - loudness(&shaped, rate)).clamp(0.0, 18.0);
    let mut after = before;
    for _ in 0..3 {
        if cancel() {
            return None;
        }
        let limiter = [("gain", gain), ("ceiling", CEILING), ("release", 100.0)];
        let out = run(
            &[(EffectKind::Limiter, params(EffectKind::Limiter, &limiter))],
            &shaped,
            rate,
        );
        after = analyse(&out, rate);
        let miss = target - after.lufs;
        if miss.abs() < 0.3 {
            break;
        }
        gain = (gain + miss).clamp(0.0, 18.0);
    }
    let limiter = vec![
        ("gain", (gain * 10.0).round() / 10.0),
        ("ceiling", CEILING),
        ("release", 100.0),
    ];
    Some(Plan {
        before,
        after,
        eq,
        compressor,
        limiter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f32, amp: f32, secs: f32) -> Vec<f32> {
        let rate = 48_000.0;
        (0..(secs * rate) as usize)
            .flat_map(|i| {
                let s = amp * (i as f32 * hz / rate * std::f32::consts::TAU).sin();
                [s, s]
            })
            .collect()
    }

    #[test]
    fn loudness_matches_the_standard() {
        // A full-scale 997 Hz sine in both channels reads about −0.7 LUFS... at −20 dBFS, about −20.7.
        let l = loudness(&tone(997.0, 0.1, 5.0), 48_000);
        assert!((l + 20.7).abs() < 0.6, "{l}");
        assert_eq!(loudness(&vec![0.0; 96_000], 48_000), -70.0, "silence");
    }

    #[test]
    fn the_plan_reaches_the_target_without_clipping() {
        // A quiet, bass-heavy "mix".
        let mut mix = tone(80.0, 0.12, 6.0);
        for (m, t) in mix.iter_mut().zip(tone(1200.0, 0.02, 6.0)) {
            *m += t;
        }
        let plan = plan(&mix, 48_000, -14.0, &|| false).unwrap();
        assert!(plan.before.lufs < -18.0);
        assert!(
            (plan.after.lufs + 14.0).abs() < 1.0,
            "reached {}",
            plan.after.lufs
        );
        assert!(
            plan.after.peak_db <= CEILING + 0.3,
            "peak {}",
            plan.after.peak_db
        );
        let low = plan.eq.iter().find(|(k, _)| *k == "ls_gain").unwrap().1;
        assert!(low < 0.0, "too much bass gets turned down: {low}");
    }
}
