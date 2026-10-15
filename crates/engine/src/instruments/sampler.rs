//! "Isai Sampler": plays an audio file from the keyboard.
//!
//! In Keys mode the sound is pitched across the keyboard from its root key,
//! with start/end points, an optional loop while a key is held, an ADSR, a
//! low-pass filter and reverse. In Slices mode the file is chopped at its
//! slice points (beats of a drum loop, words of a vocal) and key 36 (C2)
//! plays the first slice, 37 the second, and so on, each to its end.

use super::{LiveNote, MAX_INST_PARAMS, PlayNote};
use crate::clip::Clip;
use crate::effects::flush;
use crate::mixer::SILENT;

const VOICES: usize = 16;
/// The key that plays the first slice.
pub const FIRST_SLICE_KEY: u8 = 36;
/// Samples faded at the end of a slice or sample, so nothing clicks.
const END_FADE: f64 = 96.0;
const HELD: u64 = u64::MAX;

pub mod param {
    pub const MODE: usize = 0;
    pub const ROOT: usize = 1;
    pub const START: usize = 2;
    pub const END: usize = 3;
    pub const LOOP: usize = 4;
    pub const LOOP_START: usize = 5;
    pub const ATTACK: usize = 6;
    pub const DECAY: usize = 7;
    pub const SUSTAIN: usize = 8;
    pub const RELEASE: usize = 9;
    pub const TUNE: usize = 10;
    pub const FINE: usize = 11;
    pub const CUTOFF: usize = 12;
    pub const VOLUME: usize = 13;
    pub const VELOCITY: usize = 14;
    pub const REVERSE: usize = 15;
}

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy)]
struct Voice {
    stage: Stage,
    level: f32,
    key: u8,
    live: bool,
    end: u64,
    age: u64,
    /// Playback position in the sample, and how far it moves per output sample
    /// (negative when reversed).
    pos: f64,
    step: f64,
    /// The part of the sample this voice plays, and where a loop goes back to.
    from: f64,
    to: f64,
    looping: Option<f64>,
    /// Slices play to their end, whatever the key does.
    one_shot: bool,
    vel: f32,
    lp: [f32; 2],
}

impl Voice {
    const OFF: Voice = Voice {
        stage: Stage::Off,
        level: 0.0,
        key: 0,
        live: false,
        end: 0,
        age: 0,
        pos: 0.0,
        step: 1.0,
        from: 0.0,
        to: 0.0,
        looping: None,
        one_shot: false,
        vel: 0.0,
        lp: [0.0; 2],
    };

    fn held(&self) -> bool {
        self.stage != Stage::Off && self.stage != Stage::Release
    }

    fn release(&mut self) {
        if self.stage != Stage::Off && !self.one_shot {
            self.stage = Stage::Release;
        }
    }
}

pub struct Sampler {
    voices: [Voice; VOICES],
    expected: u64,
    cursor: usize,
    clock: u64,
    struck: u128,
    velocities: [u8; 128],
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler {
    pub fn new() -> Self {
        Self { voices: [Voice::OFF; VOICES], expected: SILENT, cursor: 0, clock: 0, struck: 0, velocities: [0; 128] }
    }

    pub fn keys(&mut self) -> (u128, u128) {
        let held = self.voices.iter().filter(|v| v.held()).fold(0u128, |m, v| m | 1 << (v.key & 127));
        (held, std::mem::take(&mut self.struck))
    }

    pub fn velocities(&self) -> &[u8; 128] {
        &self.velocities
    }

    pub fn is_active(&self) -> bool {
        self.voices.iter().any(|v| v.stage != Stage::Off)
    }

    pub fn reset(&mut self) {
        self.voices = [Voice::OFF; VOICES];
        self.expected = SILENT;
    }

    #[allow(clippy::too_many_arguments)]
    fn note_on(&mut self, p: &[f32; MAX_INST_PARAMS], clip: &Clip, slices: &[u64], key: u8, velocity: u8, end: u64, live: bool) {
        use param::*;
        let len = clip.frames() as f64;
        if len < 2.0 {
            return;
        }
        let key = key.min(127);
        let tune = f64::from(p[TUNE]) / 12.0 + f64::from(p[FINE]) / 1200.0;
        let reverse = p[REVERSE] >= 0.5;
        let (from, to, step, looping, one_shot) = if p[MODE] >= 0.5 {
            // Slices: a key per slice, at the sample's own pitch.
            let Some(i) = key.checked_sub(FIRST_SLICE_KEY).map(usize::from).filter(|&i| i < slices.len()) else { return };
            let from = slices[i] as f64;
            let to = slices.get(i + 1).map_or(len, |&s| s as f64).min(len);
            (from, to, 2f64.powf(tune), None, true)
        } else {
            let from = (f64::from(p[START]) / 100.0 * len).clamp(0.0, len - 1.0);
            let to = (f64::from(p[END]) / 100.0 * len).clamp(from + 1.0, len);
            let semis = f64::from(key) - f64::from(p[ROOT].round());
            let looping = (p[LOOP] >= 0.5).then(|| (f64::from(p[LOOP_START]) / 100.0 * len).clamp(from, to - 1.0));
            (from, to, 2f64.powf(semis / 12.0 + tune), looping, false)
        };
        if to - from < 2.0 {
            return;
        }
        self.clock += 1;
        self.struck |= 1 << key;
        self.velocities[usize::from(key)] = velocity.clamp(1, 127);
        let sens = (p[VELOCITY] / 100.0).clamp(0.0, 1.0);
        let vel = f32::from(velocity.clamp(1, 127)) / 127.0;
        let slot = self
            .voices
            .iter()
            .position(|v| v.stage == Stage::Off)
            .unwrap_or_else(|| (0..VOICES).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        // Retriggering a slice cuts the one already playing it.
        if one_shot {
            for v in self.voices.iter_mut().filter(|v| v.one_shot && v.key == key) {
                v.stage = Stage::Release;
                v.one_shot = false;
            }
        }
        self.voices[slot] = Voice {
            stage: Stage::Attack,
            key,
            live,
            end,
            age: self.clock,
            pos: if reverse { to - 1.0 } else { from },
            step: if reverse { -step } else { step },
            from,
            to,
            looping,
            one_shot,
            vel: 1.0 - sens + sens * vel,
            ..Voice::OFF
        };
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        p: &[f32; MAX_INST_PARAMS],
        clip: Option<&Clip>,
        slices: &[u64],
        notes: &[PlayNote],
        positions: &[u64],
        live: &[LiveNote],
        rate: f32,
        l: &mut [f32],
        r: &mut [f32],
    ) {
        use param::*;
        let Some(clip) = clip else {
            // Nothing loaded: keep the note cursor in step anyway.
            if let Some(&last) = positions.last().filter(|&&p| p != SILENT) {
                self.cursor = notes.partition_point(|n| n.start <= last);
                self.expected = last + 1;
            }
            return;
        };
        for n in live {
            if n.on && n.velocity > 0 && n.key < 128 {
                self.note_on(p, clip, slices, n.key, n.velocity, HELD, true);
            } else {
                for v in self.voices.iter_mut().filter(|v| v.live && v.key == n.key) {
                    v.release();
                }
            }
        }
        let step_ms = |ms: f32| 1.0 / (ms.max(0.5) * 0.001 * rate);
        let attack = step_ms(p[ATTACK]);
        let decay = (-6.9 * step_ms(p[DECAY])).exp();
        let sustain = (p[SUSTAIN] / 100.0).clamp(0.0, 1.0);
        let release = (-6.9 * step_ms(p[RELEASE])).exp();
        let lp = 1.0 - (-std::f32::consts::TAU * p[CUTOFF].clamp(40.0, 20_000.0).min(rate * 0.45) / rate).exp();
        let filtered = p[CUTOFF] < 19_000.0;
        let volume = 10f32.powf(p[VOLUME] / 20.0) * 0.6;
        let (left, right) = (&clip.left, &clip.right);

        for (k, &pos) in positions.iter().enumerate() {
            if pos != SILENT {
                if pos != self.expected {
                    for v in self.voices.iter_mut().filter(|v| !v.live) {
                        v.release();
                    }
                    self.cursor = notes.partition_point(|n| n.start < pos);
                }
                while let Some(&n) = notes.get(self.cursor).filter(|n| n.start <= pos) {
                    if n.start == pos && n.end > pos {
                        self.note_on(p, clip, slices, n.key, n.velocity, n.end, false);
                    }
                    self.cursor += 1;
                }
                for v in self.voices.iter_mut().filter(|v| !v.live && v.held() && v.end <= pos) {
                    v.release();
                }
                self.expected = pos + 1;
            } else {
                self.expected = SILENT;
            }

            let (mut out_l, mut out_r) = (0.0, 0.0);
            for v in &mut self.voices {
                if v.stage == Stage::Off {
                    continue;
                }
                match v.stage {
                    Stage::Attack => {
                        v.level += attack;
                        if v.level >= 1.0 {
                            v.level = 1.0;
                            v.stage = if v.one_shot { Stage::Sustain } else { Stage::Decay };
                        }
                    }
                    Stage::Decay => {
                        v.level = sustain + (v.level - sustain) * decay;
                        if v.level - sustain < 1e-4 {
                            v.stage = Stage::Sustain;
                        }
                    }
                    Stage::Sustain => {
                        if !v.one_shot {
                            v.level = sustain;
                        }
                    }
                    Stage::Release => {
                        v.level = flush(v.level * release);
                        if v.level < 1e-4 {
                            *v = Voice::OFF;
                            continue;
                        }
                    }
                    Stage::Off => {}
                }
                // Read between samples (linear), then move on.
                let i = v.pos.floor();
                let frac = (v.pos - i) as f32;
                let i = (i as usize).min(left.len() - 2);
                let mut sl = left[i] + (left[i + 1] - left[i]) * frac;
                let mut sr = right[i] + (right[i + 1] - right[i]) * frac;
                v.pos += v.step;
                // Past the end: loop back while held, or stop.
                let (past_end, left_over) = if v.step > 0.0 { (v.pos >= v.to, v.to - v.pos) } else { (v.pos < v.from, v.pos - v.from) };
                if past_end {
                    match v.looping.filter(|_| v.held()) {
                        Some(back) if v.step > 0.0 => v.pos = back + (v.pos - v.to),
                        Some(_) => v.pos = v.to - 1.0 - (v.from - v.pos),
                        None => {
                            *v = Voice::OFF;
                            continue;
                        }
                    }
                } else if v.looping.is_none() && left_over < END_FADE * v.step.abs() {
                    let fade = (left_over / (END_FADE * v.step.abs())) as f32;
                    sl *= fade;
                    sr *= fade;
                }
                if filtered {
                    v.lp[0] = flush(v.lp[0] + (sl - v.lp[0]) * lp);
                    v.lp[1] = flush(v.lp[1] + (sr - v.lp[1]) * lp);
                    sl = v.lp[0];
                    sr = v.lp[1];
                }
                let g = v.level * v.vel;
                out_l += sl * g;
                out_r += sr * g;
            }
            l[k] += out_l * volume;
            r[k] += out_r * volume;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::InstrumentKind;
    use super::*;

    /// One second of a 100 Hz sine at 48 kHz.
    fn clip() -> Clip {
        let wave: Vec<f32> = (0..48_000).map(|i| (i as f32 * 100.0 / 48_000.0 * std::f32::consts::TAU).sin() * 0.5).collect();
        Clip { name: String::new(), sample_rate: 48_000, left: wave.clone(), right: wave }
    }

    fn crossings(s: &[f32]) -> usize {
        s.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count()
    }

    fn play(p: &[f32; MAX_INST_PARAMS], slices: &[u64], key: u8, frames: usize) -> Vec<f32> {
        let mut s = Sampler::new();
        let c = clip();
        let positions: Vec<u64> = (0..frames as u64).collect();
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        let notes = [PlayNote { start: 0, end: frames as u64 / 2, key, velocity: 127 }];
        s.render(p, Some(&c), slices, &notes, &positions, &[], 48_000.0, &mut l, &mut r);
        l
    }

    #[test]
    fn keys_pitch_the_sample_from_its_root() {
        let p = InstrumentKind::Sampler.defaults();
        let at_root = crossings(&play(&p, &[], 60, 19_200)[..9600]);
        let octave_up = crossings(&play(&p, &[], 72, 19_200)[..9600]);
        assert!((at_root as i32 - 20).abs() <= 1, "100 Hz for 0.2 s: {at_root}");
        assert!((octave_up as i32 - 40).abs() <= 1, "an octave up: {octave_up}");
    }

    #[test]
    fn slices_play_their_own_part_and_stop() {
        let mut p = InstrumentKind::Sampler.defaults();
        p[param::MODE] = 1.0;
        // Four slices of a quarter second each; the second key plays the second.
        let slices = [0, 12_000, 24_000, 36_000];
        let out = play(&p, &slices, FIRST_SLICE_KEY + 1, 48_000);
        let loud = |a: usize, b: usize| out[a..b].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(loud(1000, 11_000) > 0.1, "plays");
        assert!(loud(12_100, 48_000) < 1e-6, "stops at the slice's end");
        assert!(play(&p, &slices, FIRST_SLICE_KEY + 9, 4800).iter().all(|s| *s == 0.0), "no slice there");
    }

    #[test]
    fn a_held_loop_keeps_sounding_and_reverse_plays_backwards() {
        let mut p = InstrumentKind::Sampler.defaults();
        p[param::LOOP] = 1.0;
        p[param::END] = 10.0; // the first tenth of a second, looped
        let out = play(&p, &[], 60, 96_000);
        assert!(out[40_000..46_000].iter().any(|s| s.abs() > 0.1), "still sounding long after the end, while held");
        p[param::LOOP] = 0.0;
        p[param::REVERSE] = 1.0;
        let back = play(&p, &[], 60, 9600);
        assert!(back[100] < 0.0 && back[1].abs() < 0.05, "starts from the end, faded in");
    }

    #[test]
    fn nothing_loaded_is_silent() {
        let mut s = Sampler::new();
        let (mut l, mut r) = (vec![0.0; 100], vec![0.0; 100]);
        let notes = [PlayNote { start: 0, end: 50, key: 60, velocity: 100 }];
        s.render(&InstrumentKind::Sampler.defaults(), None, &[], &notes, &(0..100).collect::<Vec<_>>(), &[], 48_000.0, &mut l, &mut r);
        assert!(l.iter().all(|s| *s == 0.0));
    }
}
