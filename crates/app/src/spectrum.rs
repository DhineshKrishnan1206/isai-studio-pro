//! A spectrum analyzer for effect editors: windowed FFT of the latest audio,
//! summarised into bands spaced evenly on a log-frequency scale.

use std::f32::consts::PI;

/// In-place radix-2 FFT (`re`, `im` lengths must be the same power of two).
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f32;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (wr, wi) = ((ang * k as f32).cos(), (ang * k as f32).sin());
                let (a, b) = (start + k, start + k + len / 2);
                let (xr, xi) = (re[b] * wr - im[b] * wi, re[b] * wi + im[b] * wr);
                re[b] = re[a] - xr;
                im[b] = im[a] - xi;
                re[a] += xr;
                im[a] += xi;
            }
        }
        len <<= 1;
    }
}

/// Level in dB of `bands` bands from 20 Hz to 20 kHz (log spaced).
pub fn bands(samples: &[f32], sample_rate: f32, bands: usize) -> Vec<f32> {
    let n = samples.len().next_power_of_two().min(samples.len().max(2));
    let n = if n.is_power_of_two() { n } else { n.next_power_of_two() / 2 };
    let mut re: Vec<f32> = samples[..n]
        .iter()
        .enumerate()
        .map(|(i, s)| s * (0.5 - 0.5 * (2.0 * PI * i as f32 / n as f32).cos()))
        .collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im);
    // Hann window and FFT size: a full-scale sine reads about 0 dB.
    let scale = 4.0 / n as f32;
    let mag: Vec<f32> = (0..n / 2).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt() * scale).collect();
    let bin_hz = sample_rate / n as f32;
    let last = mag.len() - 1;
    (0..bands)
        .map(|b| {
            let f0 = 20.0 * 1000f32.powf(b as f32 / bands as f32);
            let f1 = 20.0 * 1000f32.powf((b + 1) as f32 / bands as f32);
            let level = if f1 - f0 < bin_hz * 2.0 {
                // Narrower than the FFT's bins (the low end): read between
                // bins at the band's centre, so neighbouring bands don't
                // share one value and draw as flat steps.
                let k = ((f0 * f1).sqrt() / bin_hz).min(last as f32);
                let (i, frac) = (k.floor() as usize, k.fract());
                mag[i] + (mag[(i + 1).min(last)] - mag[i]) * frac
            } else {
                let (k0, k1) = ((f0 / bin_hz) as usize, (f1 / bin_hz).ceil() as usize);
                mag[k0.min(last)..k1.min(last + 1).max(k0.min(last) + 1)].iter().fold(0.0f32, |m, v| m.max(*v))
            };
            20.0 * level.max(1e-6).log10()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sine_shows_up_at_its_frequency() {
        let rate = 48_000.0;
        let sine: Vec<f32> = (0..2048).map(|i| (i as f32 * 1000.0 * 2.0 * PI / rate).sin()).collect();
        let b = bands(&sine, rate, 64);
        let loudest = (0..64).max_by(|&x, &y| b[x].total_cmp(&b[y])).unwrap();
        let centre = 20.0 * 1000f32.powf((loudest as f32 + 0.5) / 64.0);
        assert!((centre / 1000.0).log2().abs() < 0.2, "peak near 1 kHz, got {centre}");
        assert!(b[loudest] > -3.0 && b[loudest] < 1.0, "{}", b[loudest]);
        assert!(b[0] < -40.0, "nothing at 20 Hz");
    }
}
