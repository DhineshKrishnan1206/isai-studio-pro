//! Pitch Correct: nudges a voice (or any single melody) onto the notes of a
//! key, from a gentle touch-up to the hard, robotic snap.
//!
//! It listens to a downsampled copy of the input to find the pitch (YIN,
//! cheap enough for slow computers), picks the nearest note in the chosen key
//! and scale, and shifts the sound by the difference with a two-tap delay
//! line whose taps sweep and crossfade (a "Doppler" pitch shifter). Speed is
//! how quickly the pitch moves to the note: 0 ms is the classic hard effect.

use std::f32::consts::PI;

use super::{Context, MAX_PARAMS, flush};

pub const KEYS: &[&str] = &["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
pub const SCALES: &[&str] = &["Chromatic", "Major", "Minor", "Major pentatonic", "Minor pentatonic"];
const SCALE_STEPS: [&[u8]; 5] = [
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    &[0, 2, 4, 5, 7, 9, 11],
    &[0, 2, 3, 5, 7, 8, 10],
    &[0, 2, 4, 7, 9],
    &[0, 3, 5, 7, 10],
];

/// The rate pitch is detected at.
const ANALYSIS_RATE: f32 = 12_000.0;
/// Analysis window and hop, in downsampled samples.
const WINDOW: usize = 512;
const HOP: usize = 128;
const MIN_HZ: f32 = 70.0;
const MAX_HZ: f32 = 1000.0;
/// The shifter's grain, in seconds: longer is smoother, shorter is quicker.
const GRAIN: f32 = 0.025;

pub struct PitchCorrect {
    // Analysis: a ring of downsampled mono input.
    ring: Vec<f32>,
    ring_pos: usize,
    acc: f32,
    acc_count: usize,
    since_hop: usize,
    /// The pitch heard last, in Hz (0 when unpitched).
    pub heard: f32,
    /// The note it is being moved to, as a MIDI number (−1 when none).
    pub target: f32,
    // Shifting.
    ratio: f32,
    buf: [Vec<f32>; 2],
    pos: usize,
    phase: f32,
}

impl PitchCorrect {
    pub fn new(rate: f32) -> Self {
        let len = (GRAIN * rate * 2.0) as usize + 8;
        Self {
            ring: vec![0.0; WINDOW],
            ring_pos: 0,
            acc: 0.0,
            acc_count: 0,
            since_hop: 0,
            heard: 0.0,
            target: -1.0,
            ratio: 1.0,
            buf: [vec![0.0; len], vec![0.0; len]],
            pos: 0,
            phase: 0.0,
        }
    }

    /// YIN on the analysis ring: the pitch in Hz, if clear.
    fn detect(&self) -> Option<f32> {
        let n = WINDOW;
        let x = |i: usize| self.ring[(self.ring_pos + i) % n];
        let max_tau = ((ANALYSIS_RATE / MIN_HZ) as usize).min(n / 2);
        let min_tau = (ANALYSIS_RATE / MAX_HZ) as usize;
        let w = n - max_tau;
        let energy: f32 = (0..w).map(|i| x(i) * x(i)).sum();
        if energy / (w as f32) < 1e-5 {
            return None;
        }
        let mut d = [0.0f32; WINDOW / 2 + 1];
        let mut sum = 0.0;
        d[0] = 1.0;
        for (tau, slot) in d.iter_mut().enumerate().take(max_tau + 1).skip(1) {
            let v: f32 = (0..w).map(|i| (x(i) - x(i + tau)).powi(2)).sum();
            sum += v;
            *slot = if sum > 0.0 { v * tau as f32 / sum } else { 1.0 };
        }
        let mut tau = (min_tau.max(2)..max_tau).find(|&t| d[t] < 0.15)?;
        while tau + 1 < max_tau && d[tau + 1] < d[tau] {
            tau += 1;
        }
        let (a, b, c) = (d[tau - 1], d[tau], d[tau + 1]);
        let den = a - 2.0 * b + c;
        let shift = if den.abs() > 1e-9 { (0.5 * (a - c) / den).clamp(-1.0, 1.0) } else { 0.0 };
        Some(ANALYSIS_RATE / (tau as f32 + shift))
    }

    /// The nearest note to `midi` (fractional) in the key and scale.
    fn nearest(midi: f32, key: u8, scale: usize) -> f32 {
        let steps = SCALE_STEPS[scale.min(SCALE_STEPS.len() - 1)];
        let base = midi.round() as i32;
        (base - 6..=base + 6)
            .filter(|n| steps.contains(&((n - i32::from(key)).rem_euclid(12) as u8)))
            .map(|n| n as f32)
            .min_by(|a, b| (a - midi).abs().total_cmp(&(b - midi).abs()))
            .unwrap_or(midi)
    }

    pub fn process(&mut self, p: &[f32; MAX_PARAMS], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        let rate = ctx.sample_rate;
        let key = (p[0].round() as u8).min(11);
        let scale = p[1].round() as usize;
        let speed = p[2].max(0.0);
        let amount = (p[3] / 100.0).clamp(0.0, 1.0);
        let mix = (p[4] / 100.0).clamp(0.0, 1.0);
        let decimate = (rate / ANALYSIS_RATE).round().max(1.0) as usize;
        // How fast the shift follows its target (0 ms: at once).
        let follow = if speed < 0.5 { 0.0 } else { (-1.0 / (speed * 0.001 * rate)).exp() };
        let grain = GRAIN * rate;
        let len = self.buf[0].len();

        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            // 1. Listen.
            self.acc += (*a + *b) * 0.5;
            self.acc_count += 1;
            if self.acc_count == decimate {
                self.ring[self.ring_pos] = self.acc / decimate as f32;
                self.ring_pos = (self.ring_pos + 1) % WINDOW;
                self.acc = 0.0;
                self.acc_count = 0;
                self.since_hop += 1;
                if self.since_hop >= HOP {
                    self.since_hop = 0;
                    match self.detect() {
                        Some(hz) => {
                            let midi = 69.0 + 12.0 * (hz / 440.0).log2();
                            self.heard = hz;
                            self.target = Self::nearest(midi, key, scale);
                        }
                        None => {
                            self.heard = 0.0;
                            self.target = -1.0;
                        }
                    }
                }
            }
            // 2. Glide the shift towards the note (back to none when unpitched).
            let want = if self.target >= 0.0 && self.heard > 0.0 {
                let midi = 69.0 + 12.0 * (self.heard / 440.0).log2();
                2f32.powf((self.target - midi) * amount / 12.0)
            } else {
                1.0
            };
            self.ratio = if follow == 0.0 { want } else { want + (self.ratio - want) * follow };

            // 3. Shift: two taps sweeping through the grain, crossfaded.
            self.buf[0][self.pos] = flush(*a);
            self.buf[1][self.pos] = flush(*b);
            self.phase = (self.phase + (1.0 - self.ratio) / grain).rem_euclid(1.0);
            let read = |buf: &[f32], delay: f32| {
                let back = delay.clamp(1.0, (len - 3) as f32);
                let i = back as usize;
                let f = back - i as f32;
                let s0 = buf[(self.pos + len - i) % len];
                let s1 = buf[(self.pos + len - i - 1) % len];
                s0 + (s1 - s0) * f
            };
            let p1 = self.phase;
            let p2 = (self.phase + 0.5) % 1.0;
            let (g1, g2) = ((PI * p1).sin().powi(2), (PI * p2).sin().powi(2));
            let wet_l = read(&self.buf[0], 1.0 + p1 * grain) * g1 + read(&self.buf[0], 1.0 + p2 * grain) * g2;
            let wet_r = read(&self.buf[1], 1.0 + p1 * grain) * g1 + read(&self.buf[1], 1.0 + p2 * grain) * g2;
            self.pos = (self.pos + 1) % len;
            *a += (wet_l - *a) * mix;
            *b += (wet_r - *b) * mix;
        }
    }

    pub fn reset(&mut self) {
        // Runs on the audio thread: clear in place, never allocate.
        self.ring.fill(0.0);
        self.buf.iter_mut().for_each(|b| b.fill(0.0));
        self.ratio = 1.0;
        self.heard = 0.0;
        self.target = -1.0;
    }
}
