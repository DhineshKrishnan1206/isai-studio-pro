//! "Isai Synth": a classic analog-style (subtractive) synthesizer.
//!
//! Each voice has two oscillators (saw, square, triangle, sine, pulse or
//! noise; band-limited so high notes don't alias), a sub oscillator an
//! octave down and a noise layer. Unison stacks up to seven detuned copies
//! spread across the stereo field (the "supersaw"). The mix runs through
//! optional drive and a resonant filter (low-, high- or band-pass) with its
//! own envelope, then the amp envelope. One LFO (sine, triangle, square, saw
//! or random steps, optionally fading in after each note) can wobble the
//! pitch, the filter, the volume or the pan. Glide slides between notes; Mono and
//! Legato play one note at a time, for basses and leads.

use std::f32::consts::{FRAC_PI_4, PI, TAU};

use super::{LiveNote, MAX_INST_PARAMS, PlayNote, SUSTAIN_PEDAL};
use crate::effects::flush;
use crate::mixer::SILENT;

const VOICES: usize = 16;
const MAX_UNISON: usize = 7;
/// Filter cutoff is recomputed every this many samples (it costs a `tan`).
const CONTROL_RATE: u32 = 16;
/// End position for notes held live (released by a note-off instead).
const HELD: u64 = u64::MAX;

/// Parameter indices (see `SYNTH_PARAMS`). The first ten keep the keys
/// of the original synth, so older projects still load.
pub mod param {
    pub const WAVE: usize = 0;
    pub const DETUNE: usize = 1;
    pub const CUTOFF: usize = 2;
    pub const RESONANCE: usize = 3;
    pub const ENV_AMOUNT: usize = 4;
    pub const ATTACK: usize = 5;
    pub const DECAY: usize = 6;
    pub const SUSTAIN: usize = 7;
    pub const RELEASE: usize = 8;
    pub const VOLUME: usize = 9;
    pub const WAVE_B: usize = 10;
    pub const SEMI_B: usize = 11;
    pub const MIX: usize = 12;
    pub const PULSE_WIDTH: usize = 13;
    pub const SUB: usize = 14;
    pub const NOISE: usize = 15;
    pub const UNISON: usize = 16;
    pub const UNI_DETUNE: usize = 17;
    pub const SPREAD: usize = 18;
    pub const FILTER_TYPE: usize = 19;
    pub const DRIVE: usize = 20;
    pub const KEYTRACK: usize = 21;
    pub const F_ATTACK: usize = 22;
    pub const F_DECAY: usize = 23;
    pub const F_SUSTAIN: usize = 24;
    pub const F_RELEASE: usize = 25;
    pub const GLIDE: usize = 26;
    pub const MONO: usize = 27;
    pub const LFO_RATE: usize = 28;
    pub const LFO_DEPTH: usize = 29;
    pub const LFO_TARGET: usize = 30;
    pub const VELOCITY: usize = 31;
    pub const OCT_A: usize = 32;
    pub const FINE_A: usize = 33;
    pub const SUB_OCT: usize = 34;
    pub const LFO_SHAPE: usize = 35;
    pub const LFO_DELAY: usize = 36;
}

/// Waves, in the order of `WAVES`.
mod wave {
    pub const SAW: u32 = 0;
    pub const SQUARE: u32 = 1;
    pub const TRIANGLE: u32 = 2;
    pub const SINE: u32 = 3;
    pub const PULSE: u32 = 4;
    pub const NOISE: u32 = 5;
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// An ADSR envelope.
#[derive(Clone, Copy)]
struct Env {
    stage: Stage,
    level: f32,
}

/// Per-sample steps for an envelope's settings.
#[derive(Clone, Copy)]
struct Shape {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

impl Shape {
    fn new(attack_ms: f32, decay_ms: f32, sustain: f32, release_ms: f32, rate: f32) -> Self {
        let step = |ms: f32| 1.0 / (ms.max(0.5) * 0.001 * rate);
        // Exponential decay/release reaching ~0.1% in the set time.
        let coef = |ms: f32| (-6.9 * step(ms)).exp();
        Shape {
            attack: step(attack_ms),
            decay: coef(decay_ms),
            sustain: sustain.clamp(0.0, 1.0),
            release: coef(release_ms),
        }
    }
}

impl Env {
    const OFF: Env = Env {
        stage: Stage::Off,
        level: 0.0,
    };

    fn trigger(&mut self) {
        self.stage = Stage::Attack;
    }

    fn release(&mut self) {
        if self.stage != Stage::Off {
            self.stage = Stage::Release;
        }
    }

    #[inline]
    fn next(&mut self, s: &Shape) -> f32 {
        match self.stage {
            Stage::Attack => {
                self.level += s.attack;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level = s.sustain + (self.level - s.sustain) * s.decay;
                if self.level - s.sustain < 1e-4 {
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = s.sustain,
            Stage::Release => {
                self.level = flush(self.level * s.release);
                if self.level < 1e-4 {
                    self.level = 0.0;
                    self.stage = Stage::Off;
                }
            }
            Stage::Off => {}
        }
        self.level
    }
}

/// State-variable filter memory for one channel.
#[derive(Clone, Copy, Default)]
struct Svf {
    ic1: f32,
    ic2: f32,
}

#[derive(Clone, Copy)]
struct Voice {
    key: u8,
    velocity: f32,
    /// Timeline sample where a sequenced note ends, or `HELD` for live notes.
    end: u64,
    live: bool,
    /// Key let go while the sustain pedal was down.
    pedalled: bool,
    age: u64,
    /// Pitch now (gliding) and where it's heading, in Hz.
    freq: f32,
    target: f32,
    phase_a: [f32; MAX_UNISON],
    phase_b: [f32; MAX_UNISON],
    sub: f32,
    amp: Env,
    filter: Env,
    svf: [Svf; 2],
    // Filter coefficients (recomputed at control rate).
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
    tick: u32,
}

impl Voice {
    const OFF: Voice = Voice {
        key: 0,
        velocity: 0.0,
        end: 0,
        live: false,
        pedalled: false,
        age: 0,
        freq: 0.0,
        target: 0.0,
        phase_a: [0.0; MAX_UNISON],
        phase_b: [0.0; MAX_UNISON],
        sub: 0.0,
        amp: Env::OFF,
        filter: Env::OFF,
        svf: [Svf { ic1: 0.0, ic2: 0.0 }; 2],
        a1: 0.0,
        a2: 0.0,
        a3: 0.0,
        k: 1.0,
        tick: 0,
    };

    fn sounding(&self) -> bool {
        self.amp.stage != Stage::Off
    }

    fn held(&self) -> bool {
        self.sounding() && self.amp.stage != Stage::Release
    }
}

pub struct Synth {
    voices: [Voice; VOICES],
    /// The timeline position the next frame should have; anything else is a
    /// jump (seek, cycle, start/stop), which releases sequenced notes.
    expected: u64,
    /// Index of the next note to start in the track's notes.
    cursor: usize,
    clock: u64,
    /// Keys struck since `keys()` was last called, and how hard each was last played.
    struck: u128,
    velocities: [u8; 128],
    /// Live keys held down, oldest first (Mono/Legato go back to the
    /// previous one when the newest is let go).
    stack: [u8; 16],
    stack_len: usize,
    pedal: bool,
    /// The last note's pitch, where the next one glides from.
    last_freq: f32,
    /// LFO position, 0..1, and its held value for random steps.
    lfo_phase: f32,
    lfo_hold: f32,
    /// Samples since the LFO last started fading in (a note from silence).
    lfo_age: u32,
    noise: u32,
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}

/// Band-limited step correction (polyBLEP).
#[inline]
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

#[inline]
fn osc(wave: u32, t: f32, dt: f32, width: f32, noise: f32) -> f32 {
    match wave {
        wave::SAW => 2.0 * t - 1.0 - blep(t, dt),
        wave::SQUARE | wave::PULSE => {
            let w = if wave == wave::SQUARE { 0.5 } else { width };
            let sq = if t < w { 1.0 } else { -1.0 };
            sq + blep(t, dt) - blep((t + 1.0 - w) % 1.0, dt)
        }
        wave::TRIANGLE => 1.0 - 4.0 * (t - 0.5).abs(),
        wave::SINE => (t * TAU).sin(),
        _ => noise,
    }
}

impl Synth {
    pub fn new() -> Self {
        Self {
            voices: [Voice::OFF; VOICES],
            expected: SILENT,
            cursor: 0,
            clock: 0,
            struck: 0,
            velocities: [0; 128],
            stack: [0; 16],
            stack_len: 0,
            pedal: false,
            last_freq: 0.0,
            lfo_phase: 0.0,
            lfo_hold: 0.0,
            lfo_age: 0,
            noise: 0x9E37_79B9,
        }
    }

    pub fn keys(&mut self) -> (u128, u128) {
        let held = self
            .voices
            .iter()
            .filter(|v| v.held())
            .fold(0u128, |m, v| m | 1 << (v.key & 127));
        (held, std::mem::take(&mut self.struck))
    }

    pub fn velocities(&self) -> &[u8; 128] {
        &self.velocities
    }

    pub fn is_active(&self) -> bool {
        self.voices.iter().any(Voice::sounding)
    }

    pub fn reset(&mut self) {
        self.voices = [Voice::OFF; VOICES];
        self.expected = SILENT;
        self.stack_len = 0;
        self.pedal = false;
    }

    fn mode(p: &[f32; MAX_INST_PARAMS]) -> u32 {
        p[param::MONO].round() as u32
    }

    fn note_on(&mut self, p: &[f32; MAX_INST_PARAMS], key: u8, velocity: u8, end: u64, live: bool) {
        let key = key.min(127);
        self.clock += 1;
        self.struck |= 1 << key;
        self.velocities[usize::from(key)] = velocity.clamp(1, 127);
        let freq = 440.0 * 2f32.powf((f32::from(key) - 69.0) / 12.0);
        let glide = p[param::GLIDE] > 0.5 && self.last_freq > 0.0;
        let start = if glide { self.last_freq } else { freq };
        self.last_freq = freq;
        let vel = f32::from(velocity.clamp(1, 127)) / 127.0;
        let mode = Self::mode(p);
        // A note out of silence (or a new mono note) starts the LFO's fade-in.
        if mode == 1 || !self.voices.iter().any(Voice::held) {
            self.lfo_age = 0;
        }

        if mode > 0 {
            // Mono / Legato: one voice. Legato slides into a note while one
            // is still held, without starting the envelopes again.
            let v = &mut self.voices[0];
            let legato = mode == 2 && v.held();
            v.key = key;
            v.velocity = vel;
            v.end = end;
            v.live = live;
            v.pedalled = false;
            v.age = self.clock;
            v.target = freq;
            if !v.sounding() {
                v.freq = start;
            }
            if !legato {
                v.amp.trigger();
                v.filter.trigger();
            }
            return;
        }

        // A free voice, else the quietest releasing one, else the oldest.
        let i = self
            .voices
            .iter()
            .position(|v| !v.sounding())
            .or_else(|| {
                (0..VOICES)
                    .filter(|&i| self.voices[i].amp.stage == Stage::Release)
                    .min_by(|&a, &b| {
                        self.voices[a]
                            .amp
                            .level
                            .total_cmp(&self.voices[b].amp.level)
                    })
            })
            .unwrap_or_else(|| (0..VOICES).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        let mut v = Voice {
            key,
            velocity: vel,
            end,
            live,
            age: self.clock,
            freq: start,
            target: freq,
            ..Voice::OFF
        };
        // Spread out the starting phases so unison copies don't start in step.
        for (u, (a, b)) in v.phase_a.iter_mut().zip(v.phase_b.iter_mut()).enumerate() {
            *a = (u as f32 * 0.618_034 + self.clock as f32 * 0.1) % 1.0;
            *b = (u as f32 * 0.381_966 + self.clock as f32 * 0.23) % 1.0;
        }
        v.amp.trigger();
        v.filter.trigger();
        self.voices[i] = v;
    }

    /// A live key let go: release it (or leave it to the pedal).
    fn key_up(&mut self, p: &[f32; MAX_INST_PARAMS], key: u8) {
        if let Some(i) = self.stack[..self.stack_len].iter().position(|&k| k == key) {
            self.stack.copy_within(i + 1..self.stack_len, i);
            self.stack_len -= 1;
        }
        // Mono/Legato: go back to the key still held, if any.
        if Self::mode(p) > 0 && self.voices[0].key == key && self.stack_len > 0 {
            let back = self.stack[self.stack_len - 1];
            let freq = 440.0 * 2f32.powf((f32::from(back) - 69.0) / 12.0);
            let v = &mut self.voices[0];
            v.key = back;
            v.target = freq;
            if p[param::GLIDE] <= 0.5 {
                v.freq = freq;
            }
            self.last_freq = freq;
            return;
        }
        self.release_where(|v| v.live && v.key == key);
    }

    /// Releases matching held voices, or leaves them to the sustain pedal.
    fn release_where(&mut self, pred: impl Fn(&Voice) -> bool) {
        let pedal = self.pedal;
        for v in &mut self.voices {
            if v.held() && pred(v) {
                if pedal {
                    v.pedalled = true;
                } else {
                    v.amp.release();
                    v.filter.release();
                }
            }
        }
    }

    #[inline]
    fn noise(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        (self.noise as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        p: &[f32; MAX_INST_PARAMS],
        notes: &[PlayNote],
        positions: &[u64],
        live: &[LiveNote],
        rate: f32,
        l: &mut [f32],
        r: &mut [f32],
    ) {
        use param::*;
        for n in live {
            if n.key == SUSTAIN_PEDAL {
                self.pedal = n.on;
                if !n.on {
                    for v in &mut self.voices {
                        if v.pedalled {
                            v.pedalled = false;
                            v.amp.release();
                            v.filter.release();
                        }
                    }
                }
            } else if n.on && n.velocity > 0 {
                if self.stack_len < self.stack.len()
                    && !self.stack[..self.stack_len].contains(&n.key)
                {
                    self.stack[self.stack_len] = n.key;
                    self.stack_len += 1;
                }
                self.note_on(p, n.key, n.velocity, HELD, true);
            } else {
                self.key_up(p, n.key);
            }
        }

        // Settings for this block.
        let wave_a = p[WAVE].round() as u32;
        let wave_b = p[WAVE_B].round() as u32;
        let mix = (p[MIX] / 100.0).clamp(0.0, 1.0);
        let (gain_a, gain_b) = (1.0 - mix, mix);
        let ratio_a = 2f32.powf(p[OCT_A].round() - 2.0 + p[FINE_A] / 1200.0);
        let ratio_b = 2f32.powf(p[SEMI_B].round() / 12.0 + p[DETUNE] / 1200.0);
        let sub_ratio = if p[SUB_OCT].round() >= 1.0 { 0.25 } else { 0.5 };
        let width = (p[PULSE_WIDTH] / 100.0).clamp(0.05, 0.95);
        let sub = (p[SUB] / 100.0).clamp(0.0, 1.0);
        let noise_level = (p[NOISE] / 100.0).clamp(0.0, 1.0);
        let wants_noise = wave_a == wave::NOISE || wave_b == wave::NOISE || noise_level > 0.0;
        // Stored as a choice: 0 means one voice.
        let unison = (p[UNISON].round() as usize + 1).clamp(1, MAX_UNISON);
        let uni_cents = p[UNI_DETUNE].max(0.0);
        let spread = (p[SPREAD] / 100.0).clamp(0.0, 1.0);
        // Unison copies: detune and stereo place, worked out once.
        let mut detunes = [1.0f32; MAX_UNISON];
        let mut pans = [(1.0f32, 1.0f32); MAX_UNISON];
        for u in 0..unison {
            let x = if unison == 1 {
                0.0
            } else {
                u as f32 / (unison - 1) as f32 * 2.0 - 1.0
            };
            detunes[u] = 2f32.powf(x * uni_cents / 1200.0);
            let angle = (x * spread + 1.0) * FRAC_PI_4;
            pans[u] = (angle.cos() * 2f32.sqrt(), angle.sin() * 2f32.sqrt());
        }
        let uni_norm = 1.0 / (unison as f32).sqrt();
        let stereo = unison > 1 && spread > 0.0;

        let cutoff = p[CUTOFF];
        let q = 0.5 + (p[RESONANCE] / 100.0).clamp(0.0, 1.0) * 9.5;
        let env_octaves = (p[ENV_AMOUNT] / 100.0).clamp(0.0, 1.0) * 5.0;
        let keytrack = (p[KEYTRACK] / 100.0).clamp(0.0, 1.0);
        let filter_type = p[FILTER_TYPE].round() as u32;
        let drive = (p[DRIVE] / 100.0).clamp(0.0, 1.0);
        let drive_gain = 1.0 + 9.0 * drive;
        let drive_norm = 1.0 / drive_gain.sqrt();
        let amp_shape = Shape::new(p[ATTACK], p[DECAY], p[SUSTAIN] / 100.0, p[RELEASE], rate);
        let filter_shape = Shape::new(
            p[F_ATTACK],
            p[F_DECAY],
            p[F_SUSTAIN] / 100.0,
            p[F_RELEASE],
            rate,
        );
        let glide = if p[GLIDE] > 0.5 {
            (-1.0 / (p[GLIDE] * 0.001 * rate)).exp()
        } else {
            0.0
        };
        let lfo_step = p[LFO_RATE].max(0.01) / rate;
        let lfo_shape = p[LFO_SHAPE].round() as u32;
        let lfo_fade = if p[LFO_DELAY] > 0.5 {
            1.0 / (p[LFO_DELAY] * 0.001 * rate)
        } else {
            0.0
        };
        let lfo_depth = (p[LFO_DEPTH] / 100.0).clamp(0.0, 1.0);
        let lfo_target = p[LFO_TARGET].round() as u32;
        let sens = (p[VELOCITY] / 100.0).clamp(0.0, 1.0);
        let volume = 10f32.powf(p[VOLUME] / 20.0) * 0.3;

        for (k, &pos) in positions.iter().enumerate() {
            // Sequenced notes: release on a jump, then start/stop at this position.
            if pos != self.expected {
                for v in &mut self.voices {
                    if !v.live {
                        v.amp.release();
                        v.filter.release();
                        v.pedalled = false;
                    }
                }
                if pos != SILENT {
                    self.cursor = notes.partition_point(|n| n.start < pos);
                }
            }
            if pos != SILENT {
                while let Some(&n) = notes.get(self.cursor).filter(|n| n.start <= pos) {
                    if n.start == pos && n.end > pos {
                        self.note_on(p, n.key, n.velocity, n.end, false);
                    }
                    self.cursor += 1;
                }
                self.release_where(|v| !v.live && v.end <= pos);
                self.expected = pos + 1;
            } else {
                self.expected = SILENT;
            }

            // The LFO: shared by every voice.
            let t = self.lfo_phase;
            let shape = match lfo_shape {
                1 => 1.0 - 4.0 * (t - 0.5).abs(),
                2 => {
                    if t < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                3 => 1.0 - 2.0 * t,
                4 => self.lfo_hold,
                _ => (t * TAU).sin(),
            };
            let fade = if lfo_fade > 0.0 {
                (self.lfo_age as f32 * lfo_fade).min(1.0)
            } else {
                1.0
            };
            self.lfo_age = self.lfo_age.saturating_add(1);
            let lfo = shape * lfo_depth * fade;
            self.lfo_phase += lfo_step;
            if self.lfo_phase >= 1.0 {
                self.lfo_phase -= 1.0;
                self.lfo_hold = self.noise();
            }
            let pitch_mod = if lfo_target == 0 {
                2f32.powf(lfo / 12.0)
            } else {
                1.0
            };
            let filter_mod = if lfo_target == 1 {
                2f32.powf(lfo * 3.0)
            } else {
                1.0
            };
            let amp_mod = if lfo_target == 2 {
                1.0 - 0.5 * (lfo + lfo_depth * fade)
            } else {
                1.0
            };
            let (pan_l, pan_r) = if lfo_target == 3 {
                (1.0 - lfo.max(0.0), 1.0 + lfo.min(0.0))
            } else {
                (1.0, 1.0)
            };

            let (mut out_l, mut out_r) = (0.0, 0.0);
            for vi in 0..VOICES {
                if !self.voices[vi].sounding() {
                    continue;
                }
                let n = if wants_noise { self.noise() } else { 0.0 };
                let v = &mut self.voices[vi];
                let amp = v.amp.next(&amp_shape);
                if v.amp.stage == Stage::Off {
                    *v = Voice::OFF;
                    continue;
                }
                let fenv = v.filter.next(&filter_shape);
                // Glide toward the target pitch.
                v.freq = if glide > 0.0 {
                    v.target + (v.freq - v.target) * glide
                } else {
                    v.target
                };
                let f = v.freq * pitch_mod;

                // Oscillators (unison copies), sub and noise.
                let (mut sl, mut sr) = (0.0, 0.0);
                for u in 0..unison {
                    let fu = f * detunes[u];
                    let dta = (fu * ratio_a / rate).min(0.49);
                    let dtb = (fu * ratio_b / rate).min(0.49);
                    let s = osc(wave_a, v.phase_a[u], dta, width, n) * gain_a
                        + osc(wave_b, v.phase_b[u], dtb, width, n) * gain_b;
                    v.phase_a[u] = (v.phase_a[u] + dta) % 1.0;
                    v.phase_b[u] = (v.phase_b[u] + dtb) % 1.0;
                    sl += s * pans[u].0;
                    sr += s * pans[u].1;
                }
                sl *= uni_norm;
                sr *= uni_norm;
                if sub > 0.0 {
                    let dts = (f * sub_ratio / rate).min(0.49);
                    let s = osc(wave::SQUARE, v.sub, dts, 0.5, 0.0) * sub * 0.7;
                    v.sub = (v.sub + dts) % 1.0;
                    sl += s;
                    sr += s;
                }
                if noise_level > 0.0 {
                    sl += n * noise_level * 0.5;
                    sr += n * noise_level * 0.5;
                }
                if drive > 0.0 {
                    sl = (sl * drive_gain).tanh() * drive_norm;
                    sr = (sr * drive_gain).tanh() * drive_norm;
                }

                // Filter (Cytomic state-variable), coefficients at control rate.
                if v.tick == 0 {
                    let track = (f / 261.6).powf(keytrack);
                    let env = env_octaves * fenv * (0.6 + 0.4 * v.velocity);
                    let fc =
                        (cutoff * track * 2f32.powf(env) * filter_mod).clamp(20.0, rate * 0.45);
                    let g = (PI * fc / rate).tan();
                    v.k = 1.0 / q;
                    v.a1 = 1.0 / (1.0 + g * (g + v.k));
                    v.a2 = g * v.a1;
                    v.a3 = g * v.a2;
                }
                v.tick = (v.tick + 1) % CONTROL_RATE;
                let (a1, a2, a3, kq) = (v.a1, v.a2, v.a3, v.k);
                let run = |svf: &mut Svf, x: f32| {
                    let v3 = x - svf.ic2;
                    let v1 = a1 * svf.ic1 + a2 * v3;
                    let v2 = svf.ic2 + a2 * svf.ic1 + a3 * v3;
                    svf.ic1 = flush(2.0 * v1 - svf.ic1);
                    svf.ic2 = flush(2.0 * v2 - svf.ic2);
                    match filter_type {
                        1 => x - kq * v1 - v2, // high-pass
                        2 => v1 * kq,          // band-pass (unity at the centre)
                        _ => v2,               // low-pass
                    }
                };
                let (fl, fr) = if stereo {
                    let [a, b] = &mut v.svf;
                    (run(a, sl), run(b, sr))
                } else {
                    let m = run(&mut v.svf[0], sl);
                    (m, m)
                };

                let gain = amp * (1.0 - sens + sens * v.velocity) * amp_mod;
                out_l += fl * gain;
                out_r += fr * gain;
            }
            l[k] += out_l * volume * pan_l;
            r[k] += out_r * volume * pan_r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::InstrumentKind;
    use super::*;

    fn defaults() -> [f32; MAX_INST_PARAMS] {
        InstrumentKind::Synth.defaults()
    }

    fn run(
        synth: &mut Synth,
        p: &[f32; MAX_INST_PARAMS],
        notes: &[PlayNote],
        from: u64,
        frames: usize,
        live: &[LiveNote],
    ) -> (Vec<f32>, Vec<f32>) {
        let positions: Vec<u64> = (from..from + frames as u64).collect();
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        synth.render(p, notes, &positions, live, 48_000.0, &mut l, &mut r);
        (l, r)
    }

    fn loudness(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    fn sounding(synth: &Synth) -> Vec<u8> {
        synth
            .voices
            .iter()
            .filter(|v| v.held())
            .map(|v| v.key)
            .collect()
    }

    #[test]
    fn plays_a_note_from_its_start_to_its_end() {
        let mut synth = Synth::new();
        let notes = [PlayNote {
            start: 4800,
            end: 24_000,
            key: 60,
            velocity: 100,
        }];
        let (out, _) = run(&mut synth, &defaults(), &notes, 0, 48_000, &[]);
        assert_eq!(loudness(&out[..4800]), 0.0, "silent before the note");
        assert!(loudness(&out[6000..20_000]) > 0.01, "sounds during it");
        assert!(loudness(&out[45_000..]) < 1e-3, "released after it");
        assert!(out.iter().all(|s| s.is_finite() && s.abs() < 2.0));
    }

    #[test]
    fn a_seek_releases_sequenced_notes_but_not_live_ones() {
        let mut synth = Synth::new();
        let p = defaults();
        let notes = [PlayNote {
            start: 0,
            end: 1_000_000,
            key: 60,
            velocity: 100,
        }];
        run(
            &mut synth,
            &p,
            &notes,
            0,
            4800,
            &[LiveNote {
                on: true,
                key: 72,
                velocity: 90,
            }],
        );
        assert_eq!(sounding(&synth).len(), 2);
        run(&mut synth, &p, &notes, 500_000, 48_000, &[]);
        assert_eq!(sounding(&synth), vec![72]);
    }

    #[test]
    fn plays_live_notes_while_stopped() {
        let mut synth = Synth::new();
        let positions = vec![SILENT; 9600];
        let (mut l, mut r) = (vec![0.0; 9600], vec![0.0; 9600]);
        synth.render(
            &defaults(),
            &[],
            &positions,
            &[LiveNote {
                on: true,
                key: 64,
                velocity: 100,
            }],
            48_000.0,
            &mut l,
            &mut r,
        );
        assert!(loudness(&l[2000..]) > 0.01);
    }

    #[test]
    fn every_wave_filter_and_lfo_target_stays_in_range() {
        for wave in 0..6 {
            for filter in 0..3 {
                for target in 0..4 {
                    let mut p = defaults();
                    p[param::LFO_SHAPE] = ((wave + target) % 5) as f32;
                    p[param::SUB_OCT] = (filter % 2) as f32;
                    p[param::OCT_A] = (target % 5) as f32;
                    p[param::WAVE] = wave as f32;
                    p[param::WAVE_B] = ((wave + 1) % 6) as f32;
                    p[param::FILTER_TYPE] = filter as f32;
                    p[param::LFO_TARGET] = target as f32;
                    p[param::LFO_DEPTH] = 80.0;
                    p[param::UNISON] = 4.0; // five voices
                    p[param::SUB] = 50.0;
                    p[param::NOISE] = 20.0;
                    p[param::DRIVE] = 60.0;
                    p[param::RESONANCE] = 90.0;
                    let mut synth = Synth::new();
                    let notes = [
                        PlayNote {
                            start: 0,
                            end: 12_000,
                            key: 48,
                            velocity: 120,
                        },
                        PlayNote {
                            start: 0,
                            end: 12_000,
                            key: 84,
                            velocity: 120,
                        },
                    ];
                    let (l, r) = run(&mut synth, &p, &notes, 0, 24_000, &[]);
                    assert!(
                        l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 4.0),
                        "wave {wave} filter {filter} lfo {target}"
                    );
                    assert!(
                        loudness(&l[1000..10_000]) > 1e-3,
                        "wave {wave} filter {filter} lfo {target} is silent"
                    );
                }
            }
        }
    }

    fn crossings(s: &[f32]) -> usize {
        s.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count()
    }

    #[test]
    fn octave_a_moves_oscillator_a_by_whole_octaves() {
        let tone = |oct: f32| {
            let mut p = defaults();
            p[param::WAVE] = 3.0; // sine
            p[param::MIX] = 0.0;
            p[param::CUTOFF] = 18_000.0;
            p[param::ENV_AMOUNT] = 0.0;
            p[param::OCT_A] = oct;
            let mut synth = Synth::new();
            let (l, _) = run(
                &mut synth,
                &p,
                &[PlayNote {
                    start: 0,
                    end: 48_000,
                    key: 57,
                    velocity: 100,
                }],
                0,
                48_000,
                &[],
            );
            crossings(&l[4800..43_200])
        };
        let (down, mid, up) = (tone(1.0), tone(2.0), tone(3.0));
        assert!(
            (mid as f32 / 0.8 - 220.0).abs() < 4.0,
            "A3 is 220 Hz: {mid}"
        );
        assert!(
            (down as f32 * 2.0 - mid as f32).abs() < 4.0,
            "an octave down: {down}"
        );
        assert!(
            (up as f32 - mid as f32 * 2.0).abs() < 6.0,
            "an octave up: {up}"
        );
    }

    #[test]
    fn the_lfo_delay_fades_the_wobble_in() {
        let mut p = defaults();
        p[param::LFO_TARGET] = 2.0; // volume
        p[param::LFO_DEPTH] = 100.0;
        p[param::LFO_RATE] = 8.0;
        p[param::LFO_SHAPE] = 2.0; // square: on/off
        p[param::LFO_DELAY] = 1000.0;
        p[param::SUSTAIN] = 100.0;
        let mut synth = Synth::new();
        let (l, _) = run(
            &mut synth,
            &p,
            &[PlayNote {
                start: 0,
                end: 96_000,
                key: 60,
                velocity: 100,
            }],
            0,
            96_000,
            &[],
        );
        // Loudness of each 1/16 s window; the spread between loud and quiet grows.
        let wobble = |a: usize, b: usize| {
            let w: Vec<f32> = l[a..b].chunks(3000).map(loudness).collect();
            let max = w.iter().cloned().fold(0.0, f32::max);
            let min = w.iter().cloned().fold(f32::MAX, f32::min);
            max / min.max(1e-6)
        };
        let early = wobble(6000, 18_000);
        let late = wobble(60_000, 90_000);
        assert!(early < 1.5, "barely wobbles at first: {early}");
        assert!(late > 3.0, "wobbles fully later: {late}");
    }

    #[test]
    fn unison_spreads_across_the_stereo_field() {
        let mut p = defaults();
        p[param::UNISON] = 6.0; // seven voices
        p[param::SPREAD] = 100.0;
        let mut synth = Synth::new();
        let (l, r) = run(
            &mut synth,
            &p,
            &[PlayNote {
                start: 0,
                end: 48_000,
                key: 60,
                velocity: 100,
            }],
            0,
            24_000,
            &[],
        );
        let diff: f32 = l.iter().zip(&r).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff > 1.0, "left and right differ");
    }

    #[test]
    fn legato_glides_and_returns_to_the_held_key() {
        let mut p = defaults();
        p[param::MONO] = 2.0;
        p[param::GLIDE] = 100.0;
        let mut synth = Synth::new();
        let live = |on, key| LiveNote {
            on,
            key,
            velocity: 100,
        };
        let positions = vec![SILENT; 480];
        let (mut l, mut r) = (vec![0.0; 480], vec![0.0; 480]);
        synth.render(
            &p,
            &[],
            &positions,
            &[live(true, 48)],
            48_000.0,
            &mut l,
            &mut r,
        );
        synth.render(
            &p,
            &[],
            &positions,
            &[live(true, 60)],
            48_000.0,
            &mut l,
            &mut r,
        );
        assert_eq!(sounding(&synth), vec![60], "one voice");
        let f = synth.voices[0].freq;
        assert!(f > 130.0 && f < 261.0, "gliding up: {f}");
        synth.render(
            &p,
            &[],
            &positions,
            &[live(false, 60)],
            48_000.0,
            &mut l,
            &mut r,
        );
        assert_eq!(sounding(&synth), vec![48], "back to the key still held");
    }

    #[test]
    fn the_pedal_holds_notes() {
        let mut synth = Synth::new();
        let p = defaults();
        let positions = vec![SILENT; 480];
        let (mut l, mut r) = (vec![0.0; 480], vec![0.0; 480]);
        let pedal = |on| LiveNote {
            on,
            key: SUSTAIN_PEDAL,
            velocity: 0,
        };
        synth.render(
            &p,
            &[],
            &positions,
            &[
                pedal(true),
                LiveNote {
                    on: true,
                    key: 60,
                    velocity: 100,
                },
            ],
            48_000.0,
            &mut l,
            &mut r,
        );
        synth.render(
            &p,
            &[],
            &positions,
            &[LiveNote {
                on: false,
                key: 60,
                velocity: 0,
            }],
            48_000.0,
            &mut l,
            &mut r,
        );
        assert_eq!(sounding(&synth), vec![60], "held by the pedal");
        synth.render(
            &p,
            &[],
            &positions,
            &[pedal(false)],
            48_000.0,
            &mut l,
            &mut r,
        );
        assert!(sounding(&synth).is_empty(), "let go with the pedal");
    }
}
