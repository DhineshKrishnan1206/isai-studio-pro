//! Second-order filters (the "Audio EQ Cookbook" by Robert Bristow-Johnson).

use std::f32::consts::PI;

use super::flush;

#[derive(Clone, Copy, Debug, Default)]
pub struct Coeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Coeffs {
    /// Passes everything unchanged.
    pub const fn identity() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 }
    }

    fn normalized(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 }
    }

    fn omega(freq: f32, rate: f32) -> (f32, f32) {
        let w = 2.0 * PI * (freq / rate).clamp(1e-5, 0.49);
        (w.cos(), w.sin())
    }

    pub fn highpass(freq: f32, q: f32, rate: f32) -> Self {
        let (cos, sin) = Self::omega(freq, rate);
        let alpha = sin / (2.0 * q);
        Self::normalized((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha)
    }

    pub fn lowpass(freq: f32, q: f32, rate: f32) -> Self {
        let (cos, sin) = Self::omega(freq, rate);
        let alpha = sin / (2.0 * q);
        Self::normalized((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha)
    }

    pub fn peak(freq: f32, gain_db: f32, q: f32, rate: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let (cos, sin) = Self::omega(freq, rate);
        let alpha = sin / (2.0 * q);
        Self::normalized(1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a)
    }

    /// First-order (6 dB per octave) high-pass.
    pub fn highpass1(freq: f32, rate: f32) -> Self {
        let k = (PI * (freq / rate).clamp(1e-5, 0.49)).tan();
        Self::normalized(1.0, -1.0, 0.0, 1.0 + k, k - 1.0, 0.0)
    }

    /// First-order (6 dB per octave) low-pass.
    pub fn lowpass1(freq: f32, rate: f32) -> Self {
        let k = (PI * (freq / rate).clamp(1e-5, 0.49)).tan();
        Self::normalized(k, k, 0.0, 1.0 + k, k - 1.0, 0.0)
    }

    /// Low shelf; `q` sets how steep the shelf is (0.71 is a gentle classic).
    pub fn low_shelf(freq: f32, gain_db: f32, q: f32, rate: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let (cos, sin) = Self::omega(freq, rate);
        let alpha = sin / (2.0 * q.max(0.05));
        let sa = 2.0 * a.sqrt() * alpha;
        Self::normalized(
            a * ((a + 1.0) - (a - 1.0) * cos + sa),
            2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
            a * ((a + 1.0) - (a - 1.0) * cos - sa),
            (a + 1.0) + (a - 1.0) * cos + sa,
            -2.0 * ((a - 1.0) + (a + 1.0) * cos),
            (a + 1.0) + (a - 1.0) * cos - sa,
        )
    }

    pub fn high_shelf(freq: f32, gain_db: f32, q: f32, rate: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let (cos, sin) = Self::omega(freq, rate);
        let alpha = sin / (2.0 * q.max(0.05));
        let sa = 2.0 * a.sqrt() * alpha;
        Self::normalized(
            a * ((a + 1.0) + (a - 1.0) * cos + sa),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
            a * ((a + 1.0) + (a - 1.0) * cos - sa),
            (a + 1.0) - (a - 1.0) * cos + sa,
            2.0 * ((a - 1.0) - (a + 1.0) * cos),
            (a + 1.0) - (a - 1.0) * cos - sa,
        )
    }

    /// Gain in dB at `freq` (for drawing the EQ curve and for tests).
    pub fn response_db(&self, freq: f32, rate: f32) -> f32 {
        let w = 2.0 * PI * freq / rate;
        let (c1, s1, c2, s2) = (w.cos(), w.sin(), (2.0 * w).cos(), (2.0 * w).sin());
        let num_re = self.b0 + self.b1 * c1 + self.b2 * c2;
        let num_im = -(self.b1 * s1 + self.b2 * s2);
        let den_re = 1.0 + self.a1 * c1 + self.a2 * c2;
        let den_im = -(self.a1 * s1 + self.a2 * s2);
        let mag = ((num_re * num_re + num_im * num_im) / (den_re * den_re + den_im * den_im)).sqrt();
        20.0 * mag.max(1e-9).log10()
    }
}

/// A stereo biquad (transposed direct form II).
#[derive(Clone, Copy, Debug, Default)]
pub struct Biquad {
    pub c: Coeffs,
    z: [[f32; 2]; 2],
}

impl Biquad {
    pub fn new() -> Self {
        Self { c: Coeffs::identity(), z: [[0.0; 2]; 2] }
    }

    #[inline]
    pub fn tick(&mut self, ch: usize, x: f32) -> f32 {
        let c = &self.c;
        let z = &mut self.z[ch];
        let y = c.b0 * x + z[0];
        z[0] = flush(c.b1 * x - c.a1 * y + z[1]);
        z[1] = flush(c.b2 * x - c.a2 * y);
        y
    }

    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        for s in l.iter_mut() {
            *s = self.tick(0, *s);
        }
        for s in r.iter_mut() {
            *s = self.tick(1, *s);
        }
    }

    pub fn reset(&mut self) {
        self.z = [[0.0; 2]; 2];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_have_the_expected_response() {
        let rate = 48_000.0;
        let peak = Coeffs::peak(1000.0, 6.0, 1.0, rate);
        assert!((peak.response_db(1000.0, rate) - 6.0).abs() < 0.05);
        assert!(peak.response_db(50.0, rate).abs() < 0.3);
        let hp = Coeffs::highpass(200.0, std::f32::consts::FRAC_1_SQRT_2, rate);
        assert!((hp.response_db(200.0, rate) + 3.0).abs() < 0.1);
        assert!(hp.response_db(20.0, rate) < -30.0);
        let low = Coeffs::low_shelf(100.0, -6.0, std::f32::consts::FRAC_1_SQRT_2, rate);
        assert!((low.response_db(20.0, rate) + 6.0).abs() < 0.5);
        assert!(low.response_db(5000.0, rate).abs() < 0.2);
        let high = Coeffs::high_shelf(8000.0, 4.0, std::f32::consts::FRAC_1_SQRT_2, rate);
        assert!((high.response_db(20000.0, rate) - 4.0).abs() < 0.6);
        let hp1 = Coeffs::highpass1(1000.0, rate);
        assert!((hp1.response_db(1000.0, rate) + 3.0).abs() < 0.1);
        assert!((hp1.response_db(100.0, rate) + 20.0).abs() < 0.5, "6 dB per octave");
        let lp1 = Coeffs::lowpass1(1000.0, rate);
        assert!((lp1.response_db(1000.0, rate) + 3.0).abs() < 0.1);
    }
}
