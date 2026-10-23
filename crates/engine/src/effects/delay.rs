//! A stereo echo with feedback, a tone control, optional tempo sync and
//! ping-pong. Changing the time glides instead of clicking.

use super::{Context, MAX_PARAMS, flush};

/// Longest delay (a half note at 40 BPM is 3 s).
const MAX_SECONDS: f32 = 4.0;

pub struct Delay {
    buf: [Vec<f32>; 2],
    pos: usize,
    /// Current delay in samples, gliding towards the target.
    current: f32,
    /// One-pole low-pass state in the feedback path, per side.
    tone: [f32; 2],
}

/// Whole-note fractions for each `sync` step (index 0 is "off").
const SYNC_BEATS: [f32; 7] = [0.0, 1.0, 0.5, 0.75, 0.25, 2.0 / 3.0, 2.0];

impl Delay {
    pub fn new(rate: f32) -> Self {
        let len = (MAX_SECONDS * rate) as usize + 2;
        Self {
            buf: [vec![0.0; len], vec![0.0; len]],
            pos: 0,
            current: 0.0,
            tone: [0.0; 2],
        }
    }

    /// Delay time in seconds for these settings.
    pub fn seconds(p: &[f32; MAX_PARAMS], bpm: f32) -> f32 {
        let sync = p[1].round() as usize;
        let secs = match SYNC_BEATS.get(sync) {
            Some(&beats) if sync > 0 => beats * 60.0 / bpm.max(1.0),
            _ => p[0] / 1000.0,
        };
        secs.clamp(0.001, MAX_SECONDS - 0.01)
    }

    #[inline]
    fn read(buf: &[f32], pos: usize, delay: f32) -> f32 {
        let len = buf.len();
        let back = delay.max(1.0);
        let i = back.floor() as usize;
        let frac = back - i as f32;
        let a = buf[(pos + len - i) % len];
        let b = buf[(pos + len - i - 1) % len];
        a + (b - a) * frac
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let target = Self::seconds(p, ctx.bpm) * ctx.sample_rate;
        if self.current == 0.0 {
            self.current = target;
        }
        let feedback = (p[2] / 100.0).clamp(0.0, 0.95);
        let tone =
            1.0 - (-std::f32::consts::TAU * p[3].clamp(200.0, 20_000.0) / ctx.sample_rate).exp();
        let ping_pong = p[4] >= 0.5;
        let wet = (p[5] / 100.0).clamp(0.0, 1.0);
        let dry = 1.0 - wet * 0.5;

        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            // Glide over ~50 ms when the time changes.
            self.current += (target - self.current) * 0.0005;
            let el = Self::read(&self.buf[0], self.pos, self.current);
            let er = Self::read(&self.buf[1], self.pos, self.current);
            self.tone[0] = flush(self.tone[0] + (el - self.tone[0]) * tone);
            self.tone[1] = flush(self.tone[1] + (er - self.tone[1]) * tone);
            let (fl, fr) = (self.tone[0] * feedback, self.tone[1] * feedback);
            if ping_pong {
                // The input enters on the left; each echo crosses to the other side.
                self.buf[0][self.pos] = (*a + *b) * 0.5 + fr;
                self.buf[1][self.pos] = fl;
            } else {
                self.buf[0][self.pos] = *a + fl;
                self.buf[1][self.pos] = *b + fr;
            }
            self.pos = (self.pos + 1) % self.buf[0].len();
            *a = *a * dry + el * wet;
            *b = *b * dry + er * wet;
        }
    }

    pub fn reset(&mut self) {
        self.buf[0].fill(0.0);
        self.buf[1].fill(0.0);
        self.tone = [0.0; 2];
    }
}

#[cfg(test)]
mod tests {
    use super::super::{EffectKind, tests::CTX};
    use super::*;

    #[test]
    fn sync_follows_the_tempo() {
        let mut p = EffectKind::Delay.defaults();
        p[1] = 1.0; // 1/4
        assert!((Delay::seconds(&p, 120.0) - 0.5).abs() < 1e-6);
        p[1] = 3.0; // 1/8 dotted
        assert!((Delay::seconds(&p, 120.0) - 0.375).abs() < 1e-6);
        p[1] = 0.0;
        p[0] = 250.0;
        assert!((Delay::seconds(&p, 120.0) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn a_click_comes_back_after_the_delay_time() {
        let mut d = Delay::new(48_000.0);
        let mut p = EffectKind::Delay.defaults();
        p[1] = 0.0;
        p[0] = 100.0; // 4800 samples
        p[2] = 0.0;
        p[3] = 20_000.0;
        p[5] = 100.0;
        let (mut l, mut r) = (vec![0.0f32; 9600], vec![0.0f32; 9600]);
        l[0] = 1.0;
        r[0] = 1.0;
        d.process(&p, &CTX, &mut l, &mut r);
        let loudest = (1..9600)
            .max_by(|&a, &b| l[a].abs().total_cmp(&l[b].abs()))
            .unwrap();
        assert!((loudest as i64 - 4800).abs() <= 2, "{loudest}");
    }
}
