//! A look-ahead peak limiter: it sees peaks 1.5 ms before they play and turns
//! the gain down smoothly in time, so the output never goes over the ceiling.

use super::{Context, MAX_PARAMS, db};

const LOOKAHEAD_MS: f32 = 1.5;

pub struct Limiter {
    /// Delayed audio, so the gain can move before a peak arrives.
    delay: [Vec<f32>; 2],
    /// (time, gain needed ≤ 1) for samples in the look-ahead window, oldest
    /// first, with gains only increasing: the front is always the smallest
    /// gain in the window (a sliding minimum). A ring buffer.
    window: Vec<(u64, f32)>,
    head: usize,
    tail: usize,
    count: usize,
    now: u64,
    pos: usize,
    gain: f32,
    /// Lowest gain during the last block, for the meter.
    block_gain: f32,
}

impl Limiter {
    pub fn new(rate: f32) -> Self {
        let len = ((LOOKAHEAD_MS * 0.001 * rate) as usize).max(1);
        Self {
            delay: [vec![0.0; len], vec![0.0; len]],
            window: vec![(0, 1.0); len + 2],
            head: 0,
            tail: 0,
            count: 0,
            now: 0,
            pos: 0,
            gain: 1.0,
            block_gain: 1.0,
        }
    }

    /// Samples of delay this limiter adds.
    pub fn latency(&self) -> usize {
        self.delay[0].len()
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let [gain_db, ceiling_db, release_ms, ..] = *p;
        let input = db(gain_db);
        let ceiling = db(ceiling_db.min(0.0));
        let len = self.delay[0].len();
        let cap = self.window.len();
        // Fall fast enough to reach any peak within the look-ahead; rise at the release rate.
        let attack = 1.0 - (-4.0 / len as f32).exp();
        let release = 1.0 - (-1.0 / (release_ms.max(1.0) * 0.001 * ctx.sample_rate)).exp();

        self.block_gain = 1.0;
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let (xl, xr) = (*a * input, *b * input);
            let peak = xl.abs().max(xr.abs());
            let need = if peak > ceiling { ceiling / peak } else { 1.0 };

            // Slide the window over the last `len + 1` samples: the one about
            // to play (entered `len` samples ago) up to the one entering now.
            while self.count > 0 && self.window[self.head].0 + (len as u64) < self.now {
                self.head = (self.head + 1) % cap;
                self.count -= 1;
            }
            while self.count > 0 && self.window[(self.tail + cap - 1) % cap].1 >= need {
                self.tail = (self.tail + cap - 1) % cap;
                self.count -= 1;
            }
            self.window[self.tail] = (self.now, need);
            self.tail = (self.tail + 1) % cap;
            self.count += 1;
            self.now += 1;
            let target = self.window[self.head].1;
            let i = self.pos;

            let coef = if target < self.gain { attack } else { release };
            self.gain += (target - self.gain) * coef;
            self.block_gain = self.block_gain.min(self.gain);

            let (dl, dr) = (self.delay[0][i], self.delay[1][i]);
            self.delay[0][i] = xl;
            self.delay[1][i] = xr;
            self.pos = (i + 1) % len;
            // The smoothed gain can trail a very sharp peak; the clamp keeps the ceiling exact.
            *a = (dl * self.gain).clamp(-ceiling, ceiling);
            *b = (dr * self.gain).clamp(-ceiling, ceiling);
        }
    }

    /// Deepest gain reduction in the last block, in dB (≤ 0).
    pub fn reduction_db(&self) -> f32 {
        20.0 * self.block_gain.max(1e-6).log10()
    }

    pub fn reset(&mut self) {
        self.delay[0].fill(0.0);
        self.delay[1].fill(0.0);
        self.head = 0;
        self.tail = 0;
        self.count = 0;
        self.gain = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::super::{EffectKind, tests::CTX};
    use super::*;

    #[test]
    fn quiet_sound_passes_after_the_lookahead() {
        let mut lim = Limiter::new(48_000.0);
        let n = lim.latency();
        let (mut l, mut r) = (vec![0.0f32; 256], vec![0.0f32; 256]);
        l[0] = 0.5;
        r[0] = 0.5;
        lim.process(&EffectKind::Limiter.defaults(), &CTX, &mut l, &mut r);
        assert!((l[n] - 0.5).abs() < 1e-4, "delayed by {n} samples: {}", l[n]);
    }

    #[test]
    fn a_sudden_peak_is_turned_down_smoothly_before_it_arrives() {
        let mut lim = Limiter::new(48_000.0);
        let n = lim.latency();
        let mut l: Vec<f32> = vec![0.1; 2048];
        for s in &mut l[1000..1100] {
            *s = 2.0;
        }
        let mut r = l.clone();
        lim.process(&EffectKind::Limiter.defaults(), &CTX, &mut l, &mut r);
        let ceiling = db(-1.0);
        assert!(l.iter().all(|s| s.abs() <= ceiling + 1e-6));
        // The gain is already well down when the peak plays (no hard clipping needed).
        assert!(l[1000 + n] < ceiling * 1.001 && l[1000 + n] > ceiling * 0.8, "{}", l[1000 + n]);
    }
}
