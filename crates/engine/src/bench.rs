//! A short CPU benchmark used by the first-launch setup.
//!
//! Hardware specs are a poor guide to audio performance (an old i7 can match a
//! new i3), so we measure instead: render typical synth voices for a moment and
//! count how many this machine can play in real time on one core.

use std::hint::black_box;
use std::time::{Duration, Instant};

const SAMPLE_RATE: f32 = 48_000.0;
const BLOCK: usize = 512;
const VOICES: usize = 32;

/// Returns how many synth voices one CPU core can render in real time.
/// Runs for about `duration`.
pub fn voice_capacity(duration: Duration) -> u32 {
    let mut voices: Vec<Voice> = (0..VOICES)
        .map(|i| Voice::new(110.0 * (1.0 + i as f32 * 0.07)))
        .collect();
    let mut buf = vec![0.0f32; BLOCK];

    let started = Instant::now();
    let mut blocks = 0u64;
    while started.elapsed() < duration {
        buf.fill(0.0);
        for voice in &mut voices {
            voice.render(&mut buf);
        }
        black_box(&buf);
        blocks += 1;
    }

    let cpu_secs = started.elapsed().as_secs_f64();
    let audio_secs = blocks as f64 * BLOCK as f64 / f64::from(SAMPLE_RATE);
    (VOICES as f64 * audio_secs / cpu_secs) as u32
}

/// Roughly the work of a simple subtractive synth voice: two detuned saws,
/// a low-pass filter and an envelope.
struct Voice {
    phase_a: f32,
    phase_b: f32,
    inc_a: f32,
    inc_b: f32,
    filter: f32,
    env: f32,
}

impl Voice {
    fn new(freq: f32) -> Self {
        Self {
            phase_a: 0.0,
            phase_b: 0.3,
            inc_a: freq / SAMPLE_RATE,
            inc_b: freq * 1.005 / SAMPLE_RATE,
            filter: 0.0,
            env: 0.0,
        }
    }

    fn render(&mut self, out: &mut [f32]) {
        for s in out {
            self.phase_a = (self.phase_a + self.inc_a).fract();
            self.phase_b = (self.phase_b + self.inc_b).fract();
            let raw = (self.phase_a * 2.0 - 1.0) + (self.phase_b * 2.0 - 1.0);
            let cutoff = 0.05 + 0.2 * (self.env * 3.0).sin().abs();
            self.filter += (raw - self.filter) * cutoff;
            self.env = (self.env + 0.0001).min(1.0);
            *s += self.filter * self.env * 0.1;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reports_some_capacity() {
        assert!(super::voice_capacity(std::time::Duration::from_millis(50)) > 0);
    }
}
