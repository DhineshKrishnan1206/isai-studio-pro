//! The tuner: finds the pitch of the live input (the YIN method) and names
//! the nearest note.

const NAMES: [&str; 12] = [
    "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
];
/// Quieter input than this (RMS) shows no reading.
const GATE: f32 = 0.003;
/// Lower is stricter about what counts as a clear pitch.
const THRESHOLD: f32 = 0.15;
const MIN_HZ: f32 = 30.0;
const MAX_HZ: f32 = 1500.0;

/// A pitch reading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub hz: f32,
    /// 0..1: how clearly the input has one pitch.
    pub clarity: f32,
}

/// The pitch of `samples` (mono, at `rate`), if it has a clear one.
pub fn detect(samples: &[f32], rate: f32) -> Option<Reading> {
    if samples.len() < 256 {
        return None;
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    if rms < GATE {
        return None;
    }
    // Halve high sample rates: the same accuracy for a quarter of the work.
    let (x, rate): (Vec<f32>, f32) = if rate > 32_000.0 {
        (
            samples
                .as_chunks::<2>()
                .0
                .iter()
                .map(|[a, b]| (a + b) * 0.5)
                .collect(),
            rate / 2.0,
        )
    } else {
        (samples.to_vec(), rate)
    };
    let max_tau = ((rate / MIN_HZ) as usize).min(x.len() / 2);
    let min_tau = ((rate / MAX_HZ) as usize).max(2);
    let window = x.len() - max_tau;
    if max_tau <= min_tau + 2 || window < 64 {
        return None;
    }

    // Difference function, then cumulative mean normalised.
    let mut d = vec![0.0f32; max_tau + 1];
    for (tau, slot) in d.iter_mut().enumerate().skip(1) {
        *slot = (0..window).map(|j| (x[j] - x[j + tau]).powi(2)).sum();
    }
    let mut sum = 0.0;
    d[0] = 1.0;
    for (tau, v) in d.iter_mut().enumerate().skip(1) {
        sum += *v;
        *v = if sum > 0.0 {
            *v * tau as f32 / sum
        } else {
            1.0
        };
    }

    // The first dip under the threshold, followed to the bottom of its valley.
    let mut tau = (min_tau..max_tau).find(|&t| d[t] < THRESHOLD)?;
    while tau + 1 < max_tau && d[tau + 1] < d[tau] {
        tau += 1;
    }
    // Parabolic interpolation for a finer estimate between samples.
    let (a, b, c) = (d[tau - 1], d[tau], d[tau + 1]);
    let denom = a - 2.0 * b + c;
    let shift = if denom.abs() > 1e-9 {
        (0.5 * (a - c) / denom).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let hz = rate / (tau as f32 + shift);
    Some(Reading {
        hz,
        clarity: (1.0 - b).clamp(0.0, 1.0),
    })
}

/// The nearest note to `hz` with A4 at `a4` Hz: (name, octave, cents off, MIDI note).
pub fn note(hz: f32, a4: f32) -> (&'static str, i32, f32, i32) {
    let semis = 12.0 * (hz / a4).log2() + 69.0;
    let midi = semis.round() as i32;
    let cents = (semis - midi as f32) * 100.0;
    (
        NAMES[midi.rem_euclid(12) as usize],
        midi.div_euclid(12) - 1,
        cents,
        midi,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f32, rate: f32, n: usize, shape: fn(f32) -> f32) -> Vec<f32> {
        (0..n)
            .map(|i| 0.4 * shape((i as f32 * hz / rate).fract()))
            .collect()
    }

    fn sine(t: f32) -> f32 {
        (t * std::f32::consts::TAU).sin()
    }

    fn saw(t: f32) -> f32 {
        1.0 - 2.0 * t
    }

    #[test]
    fn finds_the_pitch_of_tones_across_the_range() {
        for (hz, shape) in [
            (82.41, saw as fn(f32) -> f32),
            (110.0, sine),
            (440.0, sine),
            (329.63, saw),
            (987.77, sine),
        ] {
            let r = detect(&tone(hz, 48_000.0, 4096, shape), 48_000.0)
                .unwrap_or_else(|| panic!("no pitch at {hz}"));
            let cents = 1200.0 * (r.hz / hz).log2();
            assert!(
                cents.abs() < 3.0,
                "{hz} Hz read as {} ({cents:.1} cents)",
                r.hz
            );
            assert!(r.clarity > 0.8);
        }
    }

    #[test]
    fn silence_and_noise_have_no_pitch() {
        assert_eq!(detect(&[0.0; 4096], 48_000.0), None);
        let mut seed = 1u32;
        let noise: Vec<f32> = (0..4096)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as f32 / u32::MAX as f32 - 0.5
            })
            .collect();
        assert_eq!(detect(&noise, 48_000.0), None);
    }

    #[test]
    fn names_notes_and_cents() {
        assert_eq!(note(440.0, 440.0), ("A", 4, 0.0, 69));
        let (name, octave, cents, _) = note(261.0, 440.0);
        assert_eq!((name, octave), ("C", 4));
        assert!((cents + 4.2).abs() < 0.5, "a little flat: {cents}");
        assert_eq!(note(432.0, 432.0).0, "A", "another reference pitch");
    }
}
