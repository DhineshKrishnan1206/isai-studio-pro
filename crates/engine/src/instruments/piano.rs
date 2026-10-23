//! "Isai Piano": a synthesized piano, so it needs no sample files and stays
//! light on slow computers.
//!
//! Every tone is built from a few kinds of voice:
//! * Strings (Grand, Felt, Clav, Honky-tonk): a handful of slightly
//!   stretched (inharmonic) partials, each ringing down at its own rate, on
//!   two strings tuned a hair apart for the slow shimmer of a real piano,
//!   with a hammer knock at the start and a damper thud at the end.
//! * Tines and reeds (Electric, Wurli): a two-operator FM voice.
//! * Bars (Toy, Music box, Celesta): a few clangorous, non-harmonic partials.
//!
//! Notes are spread across the stereo field like a pianist hears them (low
//! left, high right), then pass through a wooden body filter, optional drive,
//! a chorus ("Modulation"), tremolo or auto-pan, and a room ("Space": the
//! studio reverb, built in). The sustain pedal holds notes after their keys
//! are released, and lets the body ring along ("Resonance").

use std::f32::consts::{FRAC_PI_4, PI, TAU};

use super::{LiveNote, MAX_INST_PARAMS, PlayNote, SUSTAIN_PEDAL};
use crate::effects::biquad::{Biquad, Coeffs};
use crate::effects::{Context, EffectKind, MAX_PARAMS, Reverb, flush};
use crate::mixer::SILENT;

const VOICES: usize = 24;
/// Voices in Light mode.
const LIGHT_VOICES: usize = 12;
const PARTIALS: usize = 10;
/// Partials in Light mode.
const LIGHT_PARTIALS: usize = 5;
/// End position for notes held live (released by a note-off instead).
const HELD: u64 = u64::MAX;
/// Chorus delay line, in samples (enough for 48 kHz and above).
const CHORUS_LEN: usize = 4096;
/// Frames rendered at a time (the room works on whole chunks).
const CHUNK: usize = 512;
/// How long the room keeps ringing after the last note, in seconds.
const ROOM_TAIL: f32 = 5.0;

/// Parameter indices (see `PIANO_PARAMS`).
pub mod param {
    pub const TONE: usize = 0;
    pub const BRIGHTNESS: usize = 1;
    pub const DECAY: usize = 2;
    pub const RELEASE: usize = 3;
    pub const VELOCITY: usize = 4;
    pub const MOD_RATE: usize = 5;
    pub const MOD_DEPTH: usize = 6;
    pub const TREM_RATE: usize = 7;
    pub const TREM_DEPTH: usize = 8;
    pub const AUTOPAN: usize = 9;
    pub const VOLUME: usize = 10;
    pub const SPACE: usize = 11;
    pub const STEREO: usize = 12;
    pub const STRINGS: usize = 13;
    pub const BODY: usize = 14;
    pub const MECHANICS: usize = 15;
    pub const RESONANCE: usize = 16;
    pub const DRIVE: usize = 17;
    pub const QUALITY: usize = 18;
}

/// Tones, in the order of `PIANO_TONES` (Grand is 0, the default).
mod tone {
    pub const ELECTRIC: u32 = 1;
    pub const FELT: u32 = 2;
    pub const WURLI: u32 = 3;
    pub const CLAV: u32 = 4;
    pub const TOY: u32 = 5;
    pub const MUSIC_BOX: u32 = 6;
    pub const CELESTA: u32 = 7;
    pub const HONKY_TONK: u32 = 8;
}

#[derive(Clone, Copy)]
struct Partial {
    /// A decaying rotating phasor: (re, im) turns by (c, s) each sample.
    re: f32,
    im: f32,
    c: f32,
    s: f32,
}

impl Partial {
    const OFF: Partial = Partial {
        re: 0.0,
        im: 0.0,
        c: 1.0,
        s: 0.0,
    };

    fn new(amp: f32, freq: f32, t60_secs: f32, rate: f32) -> Self {
        let w = TAU * freq / rate;
        let d = t60(t60_secs, rate);
        // Starts at zero (a sine), so the note doesn't begin with a jump.
        Partial {
            re: amp,
            im: 0.0,
            c: w.cos() * d,
            s: w.sin() * d,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Partials,
    Fm,
}

#[derive(Clone, Copy)]
struct Voice {
    on: bool,
    kind: Kind,
    key: u8,
    /// Timeline sample where a sequenced note ends, or `HELD` for live notes.
    end: u64,
    live: bool,
    released: bool,
    /// Key let go while the pedal was down: held by the pedal.
    pedalled: bool,
    /// Partials in use: the first string's, then the second string's.
    partials: [Partial; PARTIALS * 2],
    count: usize,
    /// Hammer knock and damper thud: decaying noise, low-passed.
    knock: f32,
    knock_lp: f32,
    /// Damper (after release) and attack envelopes; the attack's step per sample.
    damp: f32,
    attack: f32,
    attack_step: f32,
    // FM: phases, modulation index and its decay, amplitude and its decay,
    // the modulator's frequency ratio, and how hard the voice saturates.
    carrier: f32,
    modulator: f32,
    ratio: f32,
    index: f32,
    index_decay: f32,
    amp: f32,
    amp_decay: f32,
    saturate: f32,
    freq: f32,
    /// Stereo gains from where the key sits on the keyboard.
    pan_l: f32,
    pan_r: f32,
    /// Recent level (for stealing the quietest voice).
    level: f32,
}

impl Voice {
    const OFF: Voice = Voice {
        on: false,
        kind: Kind::Partials,
        key: 0,
        end: 0,
        live: false,
        released: false,
        pedalled: false,
        partials: [Partial::OFF; PARTIALS * 2],
        count: 0,
        knock: 0.0,
        knock_lp: 0.0,
        damp: 1.0,
        attack: 0.0,
        attack_step: 1.0,
        carrier: 0.0,
        modulator: 0.0,
        ratio: 1.0,
        index: 0.0,
        index_decay: 1.0,
        amp: 0.0,
        amp_decay: 1.0,
        saturate: 0.0,
        freq: 0.0,
        pan_l: 1.0,
        pan_r: 1.0,
        level: 0.0,
    };
}

pub struct Piano {
    voices: [Voice; VOICES],
    expected: u64,
    cursor: usize,
    /// Keys struck since `keys()` was last called, and how hard each key was last played.
    struck: u128,
    velocities: [u8; 128],
    pedal: bool,
    /// The pedal, smoothed 0..1 (drives the resonance).
    pedal_amount: f32,
    noise: u32,
    chorus: Vec<[f32; 2]>,
    chorus_pos: usize,
    mod_phase: f32,
    trem_phase: f32,
    body: [Biquad; 3],
    body_for: (f32, f32),
    room: Reverb,
    /// Frames the room still rings for after the voices stop.
    room_tail: usize,
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
}

impl Default for Piano {
    fn default() -> Self {
        Self::new()
    }
}

/// Time (seconds) for a level to fall 60 dB → per-sample multiplier.
fn t60(seconds: f32, rate: f32) -> f32 {
    (-6.9078 / (seconds.max(0.005) * rate)).exp()
}

impl Piano {
    pub fn new() -> Self {
        Self {
            voices: [Voice::OFF; VOICES],
            expected: SILENT,
            cursor: 0,
            struck: 0,
            velocities: [0; 128],
            pedal: false,
            pedal_amount: 0.0,
            noise: 0x1234_5678,
            chorus: vec![[0.0; 2]; CHORUS_LEN],
            chorus_pos: 0,
            mod_phase: 0.0,
            trem_phase: 0.0,
            body: [Biquad::new(); 3],
            body_for: (-1.0, 0.0),
            // Sized for rates up to 96 kHz.
            room: Reverb::new(96_000.0),
            room_tail: 0,
            buf_l: vec![0.0; CHUNK],
            buf_r: vec![0.0; CHUNK],
        }
    }

    pub fn keys(&mut self) -> (u128, u128) {
        let held = self
            .voices
            .iter()
            .filter(|v| v.on && !v.released)
            .fold(0u128, |m, v| m | 1 << (v.key & 127));
        (held, std::mem::take(&mut self.struck))
    }

    /// How hard each key was last played (1..=127, 0 = not yet).
    pub fn velocities(&self) -> &[u8; 128] {
        &self.velocities
    }

    pub fn is_active(&self) -> bool {
        self.room_tail > 0 || self.voices.iter().any(|v| v.on)
    }

    pub fn reset(&mut self) {
        self.voices = [Voice::OFF; VOICES];
        self.expected = SILENT;
        self.pedal = false;
        self.chorus.iter_mut().for_each(|s| *s = [0.0; 2]);
        self.body.iter_mut().for_each(Biquad::reset);
        self.room.reset();
        self.room_tail = 0;
    }

    fn note_on(
        &mut self,
        p: &[f32; MAX_INST_PARAMS],
        key: u8,
        velocity: u8,
        end: u64,
        live: bool,
        rate: f32,
    ) {
        use param::*;
        let light = p[QUALITY] >= 0.5;
        let voices = if light { LIGHT_VOICES } else { VOICES };
        // The same key still ringing lets go (it fades, like a real damper)
        // rather than being cut off, which would click.
        self.force_release(|v| v.key == key);
        // A free voice, else the quietest (cut, but by then it's nearly silent).
        let i = (0..voices)
            .find(|&i| !self.voices[i].on)
            .unwrap_or_else(|| {
                (0..voices)
                    .min_by(|&a, &b| self.voices[a].level.total_cmp(&self.voices[b].level))
                    .unwrap_or(0)
            });

        let key = key.min(127);
        self.struck |= 1 << key;
        self.velocities[usize::from(key)] = velocity.clamp(1, 127);
        let tone = p[TONE].round() as u32;
        let freq = 440.0 * 2f32.powf((f32::from(key) - 69.0) / 12.0);
        let sens = (p[VELOCITY] / 100.0).clamp(0.0, 1.0);
        let vel = f32::from(velocity.clamp(1, 127)) / 127.0;
        let loud = 1.0 - sens + sens * vel.powf(1.6);
        let bright = (p[BRIGHTNESS] / 100.0).clamp(0.0, 1.0);
        let mech = (p[MECHANICS] / 100.0).clamp(0.0, 1.0);
        // Low notes ring longer than high ones.
        let ring = p[DECAY].max(0.2) * (261.6 / freq).powf(0.45).clamp(0.2, 2.5);
        // Where the key sits: low notes to the left, high to the right.
        let width = (p[STEREO] / 100.0).clamp(0.0, 1.0);
        let place = ((f32::from(key) - 64.0) / 36.0).clamp(-1.0, 1.0) * width;
        let angle = (place + 1.0) * FRAC_PI_4;

        let mut v = Voice {
            on: true,
            key,
            end,
            live,
            freq,
            pan_l: angle.cos() * 2f32.sqrt(),
            pan_r: angle.sin() * 2f32.sqrt(),
            ..Voice::OFF
        };
        let max_partials = if light { LIGHT_PARTIALS } else { PARTIALS };
        match tone {
            tone::ELECTRIC | tone::WURLI => {
                let wurli = tone == tone::WURLI;
                v.kind = Kind::Fm;
                v.ratio = 1.0;
                v.index = if wurli {
                    (1.6 + 3.0 * bright) * (0.4 + 0.6 * vel)
                } else {
                    (1.2 + 4.0 * bright) * (0.35 + 0.65 * vel)
                };
                v.index_decay = t60(
                    if wurli {
                        0.25
                    } else {
                        0.35 + 0.4 * (1.0 - bright)
                    },
                    rate,
                );
                v.amp = loud * if wurli { 0.42 } else { 0.55 };
                v.amp_decay = t60(ring * if wurli { 0.55 } else { 0.8 }, rate);
                // A Wurlitzer's reed growls when hit hard.
                v.saturate = if wurli { 1.0 + 3.0 * vel } else { 0.0 };
                v.attack_step = 1.0 / (0.001 * rate);
                v.knock = 0.01 * mech * vel;
            }
            tone::TOY | tone::MUSIC_BOX | tone::CELESTA => {
                // Struck metal bars and tines: few, clangy partials.
                let (ratios, amps, length, attack): (&[f32], &[f32], f32, f32) = match tone {
                    tone::TOY => (
                        &[1.0, 2.76, 5.40, 8.93],
                        &[1.0, 0.55, 0.3, 0.15],
                        0.35,
                        0.001,
                    ),
                    tone::MUSIC_BOX => {
                        (&[1.0, 3.0, 5.93, 9.8], &[1.0, 0.3, 0.18, 0.08], 0.55, 0.001)
                    }
                    _ => (&[1.0, 4.0, 10.0], &[1.0, 0.22, 0.06], 0.75, 0.003),
                };
                let mut total = 0.0;
                for (n, (&ratio, &amp)) in ratios.iter().zip(amps).enumerate() {
                    let f = freq * ratio;
                    if f > rate * 0.45 || n >= max_partials {
                        break;
                    }
                    let a = amp * (if n == 0 { 1.0 } else { 0.4 + 0.6 * bright });
                    v.partials[n] = Partial::new(a, f, ring * length / (1.0 + n as f32), rate);
                    v.count = n + 1;
                    total += a;
                }
                let norm = loud * 0.45 / total.max(0.3);
                v.partials[..v.count].iter_mut().for_each(|p| p.re *= norm);
                v.attack_step = 1.0 / (attack * rate);
                v.knock = 0.015 * mech * vel;
            }
            _ => {
                // Strings: Grand, Felt, Clav, Honky-tonk.
                let felt = tone == tone::FELT;
                let clav = tone == tone::CLAV;
                let honky = tone == tone::HONKY_TONK;
                // Stretched partials: real strings run a little sharp up high.
                let b = 0.00035 * (freq / 261.6).powf(1.2);
                // Where the string is struck shapes the partials: a clavinet
                // is plucked near its end, so it is bright and nasal.
                let strike = if clav { 0.035 } else { 0.125 };
                let tilt = if felt {
                    2.4
                } else if clav {
                    1.0 - 0.4 * bright
                } else {
                    1.9 - 1.0 * bright - 0.5 * vel
                };
                let length = if clav { 0.3 } else { 1.0 };
                // The second string: a few tenths of a cent off (or a lot, honky-tonk).
                let unison = (p[STRINGS] / 100.0).clamp(0.0, 1.0);
                let cents = if honky {
                    14.0 + 6.0 * unison
                } else {
                    unison * (0.6 + 1.6 * (1.0 - f32::from(key) / 127.0))
                };
                let two = !light && (cents > 0.05);
                let detune = 2f32.powf(cents / 1200.0);
                let mut total = 0.0;
                let mut count = 0;
                for n in 0..max_partials {
                    let k = (n + 1) as f32;
                    let f = freq * k * (1.0 + b * k * k).sqrt();
                    if f * detune > rate * 0.45 {
                        break;
                    }
                    let comb = (PI * k * strike).sin().abs().max(0.15);
                    let a = comb / k.powf(tilt.max(0.6));
                    let secs =
                        ring * length / (1.0 + 0.45 * (k - 1.0) * if felt { 2.0 } else { 1.0 });
                    if two {
                        v.partials[n] = Partial::new(a * 0.5, f, secs, rate);
                        v.partials[PARTIALS + n] =
                            Partial::new(a * 0.5, f * detune, secs * 1.15, rate);
                    } else {
                        v.partials[n] = Partial::new(a, f, secs, rate);
                    }
                    total += a;
                    count = n + 1;
                }
                // Partials are laid out first string then second; `count` covers
                // the first, and the loop below also walks the second when used.
                v.count = if two { PARTIALS + count } else { count };
                let norm = loud * 0.5 / total.max(0.3);
                v.partials.iter_mut().for_each(|p| p.re *= norm);
                // A soft thump of the hammer, felt more than heard.
                v.knock = if felt { 0.012 } else { 0.03 }
                    * vel
                    * (0.5 + 0.5 * bright)
                    * (0.3 + 1.4 * mech);
                // Felt has a softer start; the others rise in 2 ms.
                v.attack_step = 1.0 / (if felt { 0.012 } else { 0.002 } * rate);
            }
        }
        v.level = loud;
        self.voices[i] = v;
    }

    /// A key let go: the damper stops the note, unless the sustain pedal is down.
    fn key_up(&mut self, mech: f32, pred: impl Fn(&Voice) -> bool) {
        let pedal = self.pedal;
        for v in &mut self.voices {
            if v.on && !v.released && pred(v) {
                if pedal {
                    v.pedalled = true;
                } else {
                    Self::damp(v, mech);
                }
            }
        }
    }

    /// Stops notes whatever the pedal does (a jump in the song, a key played again).
    fn force_release(&mut self, pred: impl Fn(&Voice) -> bool) {
        for v in &mut self.voices {
            if v.on && !v.released && pred(v) {
                v.released = true;
            }
        }
    }

    fn damp(v: &mut Voice, mech: f32) {
        v.released = true;
        // The damper lands with a soft thud.
        if v.kind == Kind::Partials && mech > 0.0 {
            v.knock = v.knock.max(0.006 * mech);
        }
    }

    fn set_pedal(&mut self, down: bool, mech: f32) {
        self.pedal = down;
        if !down {
            for v in &mut self.voices {
                if v.on && v.pedalled && !v.released {
                    v.pedalled = false;
                    Self::damp(v, mech);
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
        for (i, ((pos, l), r)) in positions
            .chunks(CHUNK)
            .zip(l.chunks_mut(CHUNK))
            .zip(r.chunks_mut(CHUNK))
            .enumerate()
        {
            self.render_chunk(p, notes, pos, if i == 0 { live } else { &[] }, rate, l, r);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_chunk(
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
        let frames = positions.len();
        let mech = (p[MECHANICS] / 100.0).clamp(0.0, 1.0);
        for n in live {
            if n.key == SUSTAIN_PEDAL {
                self.set_pedal(n.on, mech);
            } else if n.on && n.velocity > 0 {
                self.note_on(p, n.key, n.velocity, HELD, true, rate);
            } else {
                self.key_up(mech, |v| v.live && v.key == n.key);
            }
        }

        let release_rate = t60(p[RELEASE] / 1000.0, rate);
        let volume = 10f32.powf(p[VOLUME] / 20.0);
        let knock_decay = t60(0.02, rate);
        let knock_lp = 1.0 - (-TAU * 900.0 / rate).exp();
        // Chorus: 7 ms ± up to 4 ms, the two sides a quarter turn apart.
        let mod_depth = (p[MOD_DEPTH] / 100.0).clamp(0.0, 1.0);
        let mod_step = TAU * p[MOD_RATE].max(0.05) / rate;
        let base = 0.007 * rate;
        let swing = 0.004 * rate * mod_depth;
        let trem_depth = (p[TREM_DEPTH] / 100.0).clamp(0.0, 1.0);
        let trem_step = TAU * p[TREM_RATE].max(0.05) / rate;
        let autopan = p[AUTOPAN] >= 0.5;
        let drive = (p[DRIVE] / 100.0).clamp(0.0, 1.0);
        let drive_gain = 1.0 + 7.0 * drive;
        let drive_makeup = 1.0 / drive_gain.sqrt();
        let light = p[QUALITY] >= 0.5;

        // The wooden body: warmth low down, a little "box", less harshness.
        let body = if light {
            0.0
        } else {
            (p[BODY] / 100.0).clamp(0.0, 1.0)
        };
        if body > 0.0 && (body, rate) != self.body_for {
            self.body[0].c = Coeffs::peak(110.0, 4.0 * body, 0.9, rate);
            self.body[1].c = Coeffs::peak(420.0, 2.0 * body, 1.4, rate);
            self.body[2].c = Coeffs::peak(2800.0, -2.5 * body, 0.9, rate);
            self.body_for = (body, rate);
        }

        for (k, &pos) in positions.iter().enumerate() {
            if pos != self.expected {
                self.force_release(|v| !v.live);
                if pos != SILENT {
                    self.cursor = notes.partition_point(|n| n.start < pos);
                }
            }
            if pos != SILENT {
                while let Some(&n) = notes.get(self.cursor).filter(|n| n.start <= pos) {
                    if n.start == pos && n.end > pos {
                        self.note_on(p, n.key, n.velocity, n.end, false, rate);
                    }
                    self.cursor += 1;
                }
                self.key_up(mech, |v| !v.live && v.end <= pos);
                self.expected = pos + 1;
            } else {
                self.expected = SILENT;
            }

            let (mut out_l, mut out_r) = (0.0, 0.0);
            for i in 0..VOICES {
                if !self.voices[i].on {
                    continue;
                }
                let n = if self.voices[i].knock > 1e-5 {
                    self.noise()
                } else {
                    0.0
                };
                let v = &mut self.voices[i];
                if v.released {
                    v.damp *= release_rate;
                }
                if v.attack < 1.0 {
                    v.attack = (v.attack + v.attack_step).min(1.0);
                }
                let mut s = 0.0;
                let mut level = 0.0;
                if v.kind == Kind::Fm {
                    let m = (v.modulator * TAU).sin();
                    s = (v.carrier * TAU + v.index * m).sin() * v.amp;
                    if v.saturate > 0.0 {
                        s = (s * v.saturate).tanh() / v.saturate.sqrt();
                    }
                    let dt = v.freq / rate;
                    v.carrier = (v.carrier + dt) % 1.0;
                    v.modulator = (v.modulator + dt * v.ratio) % 1.0;
                    v.index = flush(v.index * v.index_decay);
                    v.amp = flush(v.amp * v.amp_decay);
                    level = v.amp;
                } else {
                    let count = v.count;
                    for (n, part) in v.partials.iter_mut().enumerate() {
                        // Skip the unused gap between the two strings' partials.
                        if n >= count {
                            break;
                        }
                        if part.c == 1.0 && part.s == 0.0 {
                            continue;
                        }
                        let re = part.re * part.c - part.im * part.s;
                        let im = part.re * part.s + part.im * part.c;
                        part.re = flush(re);
                        part.im = flush(im);
                        s += im;
                        level += im.abs() + re.abs();
                    }
                }
                if v.knock > 1e-5 {
                    v.knock_lp += (n * v.knock - v.knock_lp) * knock_lp;
                    s += v.knock_lp;
                    v.knock *= knock_decay;
                }
                let env = v.damp * v.attack;
                out_l += s * env * v.pan_l;
                out_r += s * env * v.pan_r;
                v.level = level * env;
                if env < 1e-4 || (level < 1e-5 && v.knock < 1e-5) {
                    *v = Voice::OFF;
                }
            }
            let (mut ol, mut or) = (out_l * volume, out_r * volume);

            if body > 0.0 {
                for b in &mut self.body {
                    ol = b.tick(0, ol);
                    or = b.tick(1, or);
                }
            }
            if drive > 0.0 {
                ol = (ol * drive_gain).tanh() * drive_makeup;
                or = (or * drive_gain).tanh() * drive_makeup;
            }

            // Chorus (only when there is some depth).
            if mod_depth > 0.0 {
                self.chorus[self.chorus_pos] = [ol, or];
                let tap = |phase: f32, ch: usize, chorus: &Vec<[f32; 2]>, pos: usize| {
                    let d = (base + swing * phase.sin()).clamp(1.0, (CHORUS_LEN - 2) as f32);
                    let i = d.floor() as usize;
                    let f = d - i as f32;
                    let a = chorus[(pos + CHORUS_LEN - i) % CHORUS_LEN][ch];
                    let b = chorus[(pos + CHORUS_LEN - i - 1) % CHORUS_LEN][ch];
                    a + (b - a) * f
                };
                let wl = tap(self.mod_phase, 0, &self.chorus, self.chorus_pos);
                let wr = tap(
                    self.mod_phase + TAU * 0.25,
                    1,
                    &self.chorus,
                    self.chorus_pos,
                );
                let mix = 0.5 * mod_depth;
                ol = ol * (1.0 - mix) + wl * mix;
                or = or * (1.0 - mix) + wr * mix;
                self.chorus_pos = (self.chorus_pos + 1) % CHORUS_LEN;
                self.mod_phase = (self.mod_phase + mod_step) % TAU;
            }
            // Tremolo (both sides together) or auto-pan (opposite sides).
            if trem_depth > 0.0 {
                let lfo = self.trem_phase.sin();
                if autopan {
                    ol *= 1.0 - trem_depth * 0.5 * (1.0 + lfo);
                    or *= 1.0 - trem_depth * 0.5 * (1.0 - lfo);
                } else {
                    let g = 1.0 - trem_depth * 0.5 * (1.0 + lfo);
                    ol *= g;
                    or *= g;
                }
                self.trem_phase = (self.trem_phase + trem_step) % TAU;
            }
            self.buf_l[k] = ol;
            self.buf_r[k] = or;
        }

        // The pedal lets the strings and body ring along: more room while it's down.
        let target = if self.pedal { 1.0 } else { 0.0 };
        self.pedal_amount +=
            (target - self.pedal_amount) * (frames as f32 / (0.15 * rate)).min(1.0);
        let resonance = (p[RESONANCE] / 100.0).clamp(0.0, 1.0) * self.pedal_amount;

        // The room: rings on after the notes for a few seconds.
        let space = (p[SPACE] / 100.0).clamp(0.0, 1.0);
        if self.voices.iter().any(|v| v.on) {
            self.room_tail = (ROOM_TAIL * rate) as usize;
        } else {
            self.room_tail = self.room_tail.saturating_sub(frames);
        }
        let wet = (space + 0.35 * resonance).min(1.0);
        if wet > 0.001 {
            let mut rp: [f32; MAX_PARAMS] = EffectKind::Reverb.defaults();
            use crate::effects::reverb::param as rv;
            rp[rv::DECAY] = 1.2 + 3.0 * space + 2.0 * resonance;
            rp[rv::SIZE] = 85.0;
            rp[rv::PREDELAY] = 18.0;
            rp[rv::DAMPING] = 6500.0;
            rp[rv::LOW_CUT] = 180.0;
            rp[rv::MODULATION] = 30.0;
            rp[rv::WIDTH] = 100.0;
            rp[rv::MIX] = 55.0 * wet;
            let ctx = Context {
                sample_rate: rate,
                bpm: 120.0,
            };
            self.room.process(
                &rp,
                &ctx,
                &mut self.buf_l[..frames],
                &mut self.buf_r[..frames],
            );
        } else {
            self.room_tail = 0;
        }
        for k in 0..frames {
            l[k] += self.buf_l[k];
            r[k] += self.buf_r[k];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::InstrumentKind;
    use super::*;

    fn run(
        piano: &mut Piano,
        p: &[f32; MAX_INST_PARAMS],
        notes: &[PlayNote],
        frames: usize,
    ) -> (Vec<f32>, Vec<f32>) {
        let positions: Vec<u64> = (0..frames as u64).collect();
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        piano.render(p, notes, &positions, &[], 48_000.0, &mut l, &mut r);
        (l, r)
    }

    fn loudness(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    #[test]
    fn every_tone_plays_rings_down_and_stays_in_range() {
        for tone in 0..9 {
            for light in [0.0, 1.0] {
                let mut p = InstrumentKind::Piano.defaults();
                p[param::TONE] = tone as f32;
                p[param::QUALITY] = light;
                p[param::MOD_DEPTH] = 50.0;
                p[param::TREM_DEPTH] = 50.0;
                p[param::DRIVE] = 40.0;
                let mut piano = Piano::new();
                let notes = [PlayNote {
                    start: 0,
                    end: 24_000,
                    key: 60,
                    velocity: 110,
                }];
                let (out, _) = run(&mut piano, &p, &notes, 96_000);
                assert!(
                    out.iter().all(|s| s.is_finite() && s.abs() < 1.5),
                    "tone {tone}"
                );
                let early = loudness(&out[1000..6000]);
                let late = loudness(&out[60_000..96_000]);
                assert!(early > 0.01, "tone {tone} sounds: {early}");
                assert!(
                    late < early * 0.1,
                    "tone {tone} fades after release: {early} → {late}"
                );
            }
        }
    }

    #[test]
    fn harder_notes_are_louder() {
        let p = InstrumentKind::Piano.defaults();
        let play = |velocity| {
            let mut piano = Piano::new();
            let (out, _) = run(
                &mut piano,
                &p,
                &[PlayNote {
                    start: 0,
                    end: 48_000,
                    key: 64,
                    velocity,
                }],
                12_000,
            );
            loudness(&out)
        };
        assert!(play(120) > play(30) * 1.5);
    }

    #[test]
    fn notes_start_and_restart_without_a_click() {
        // A click is a sudden jump between neighbouring samples.
        let biggest_step = |out: &[f32]| {
            out.windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0.0f32, f32::max)
        };
        for tone in 0..9 {
            let mut p = InstrumentKind::Piano.defaults();
            p[param::TONE] = tone as f32;
            p[param::SPACE] = 0.0;
            let mut piano = Piano::new();
            // The same key twice: the second hit lands while the first still rings.
            let notes = [
                PlayNote {
                    start: 100,
                    end: 20_000,
                    key: 60,
                    velocity: 127,
                },
                PlayNote {
                    start: 4000,
                    end: 20_000,
                    key: 60,
                    velocity: 127,
                },
            ];
            let (out, _) = run(&mut piano, &p, &notes, 8000);
            // Compared with the note's own steepest slope once it's ringing.
            let ringing = biggest_step(&out[500..1500]);
            let start = biggest_step(&out[95..400]);
            let again = biggest_step(&out[3995..4300]);
            assert!(
                start < ringing * 1.6 && again < ringing * 1.6,
                "tone {tone}: ringing {ringing}, steps {start} {again}"
            );
        }
    }

    #[test]
    fn low_notes_lean_left_and_high_notes_right() {
        let p = InstrumentKind::Piano.defaults();
        let side = |key| {
            let mut piano = Piano::new();
            let (l, r) = run(
                &mut piano,
                &p,
                &[PlayNote {
                    start: 0,
                    end: 48_000,
                    key,
                    velocity: 100,
                }],
                9600,
            );
            loudness(&l) / loudness(&r)
        };
        assert!(
            side(36) > 1.2 && side(96) < 0.8,
            "{} {}",
            side(36),
            side(96)
        );
    }

    #[test]
    fn the_pedal_holds_notes_after_their_keys_let_go() {
        let mut p = InstrumentKind::Piano.defaults();
        p[param::SPACE] = 0.0;
        p[param::RESONANCE] = 0.0;
        let ring_after_release = |pedal: bool| {
            let mut piano = Piano::new();
            let frames = 4800;
            let positions = vec![SILENT; frames];
            let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
            let mut live = vec![
                LiveNote {
                    on: pedal,
                    key: SUSTAIN_PEDAL,
                    velocity: 0,
                },
                LiveNote {
                    on: true,
                    key: 60,
                    velocity: 100,
                },
            ];
            piano.render(&p, &[], &positions, &live, 48_000.0, &mut l, &mut r);
            live = vec![LiveNote {
                on: false,
                key: 60,
                velocity: 0,
            }];
            for _ in 0..10 {
                l.fill(0.0);
                r.fill(0.0);
                piano.render(&p, &[], &positions, &live, 48_000.0, &mut l, &mut r);
                live.clear();
            }
            (loudness(&l), piano.keys().0)
        };
        let (with, held_with) = ring_after_release(true);
        let (without, _) = ring_after_release(false);
        assert!(
            with > without * 10.0,
            "pedal keeps it ringing: {with} vs {without}"
        );
        assert_eq!(held_with, 1 << 60, "still shown as playing");
    }
}
