//! Everything that runs on the audio thread.
//!
//! Rules for this file: no allocation, no freeing, no locks, no I/O, no printing.
//! Anything that must be freed is handed back to the UI thread to drop.
//!
//! The transport runs on a timeline measured in samples. It plays whether or
//! not there is audio on it, like any DAW, and the metronome follows the
//! project's tempo and time signature.
//!
//! Live input (microphone or interface) arrives through a lock-free queue
//! filled by the input stream. It is read here, in the output callback, so each
//! recorded sample is tied to an exact timeline position; the same samples
//! feed input monitoring.

use std::f32::consts::TAU;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

use cpal::{FromSample, Sample};

use crate::arrangement::{Arrangement, MAX_BUSES, MAX_TRACKS};
use crate::effects::{Effect, EffectBank};
use crate::instruments::{Instrument, InstrumentBank, LiveNote, PlayedNote};
use crate::clip::Clip;
use crate::mixer::{BLOCK, Block, Mixer, SILENT};
use crate::time::{Meter, TempoMap};

pub(crate) enum Command {
    /// Start playback, first counting in this many bars on the metronome.
    Play { count_in_bars: u32 },
    Stop,
    Seek(u64),
    /// Loop between two timeline positions (in samples) while `on`.
    SetCycle { on: bool, start: u64, end: u64 },
    /// The song's tempo and time signatures (built on the UI thread).
    SetTempo(Box<TempoMap>),
    SetMetronome { on: bool, volume: f32 },
    SetVolume(f32),
    SetArrangement(Arc<Arrangement>),
    /// Live input as stereo interleaved samples at the output's rate.
    SetInput(Option<rtrb::Consumer<f32>>),
    /// Capture input to the record queue from the next played sample.
    SetRecording(bool),
    /// Hear the input through the outputs.
    SetMonitor(bool),
    TestTone,
    /// A new effect (built on the UI thread) for the insert with this id.
    AddEffect(u64, Box<Effect>, Option<Arc<crate::effects::Meters>>),
    RemoveEffect(u64),
    /// A new instrument (built on the UI thread) for the track instrument with this id.
    AddInstrument(u64, Box<Instrument>),
    RemoveInstrument(u64),
    /// Which track (arrangement index) live notes play on.
    SetLiveTrack(Option<usize>),
    /// Report live notes with their timeline position while playing (MIDI recording).
    SetCaptureNotes(bool),
    /// Play a sound on its own, after everything else (the browser's preview).
    /// With `sync` while playing, it starts on the next bar. `None` stops it.
    Preview { clip: Option<Arc<Clip>>, sync: bool },
}

/// Things the audio thread replaced and hands back to the UI thread to free.
pub(crate) enum Garbage {
    Arrangement(#[allow(dead_code)] Arc<Arrangement>),
    Input(#[allow(dead_code)] rtrb::Consumer<f32>),
    Effect(#[allow(dead_code)] Box<Effect>, #[allow(dead_code)] Option<Arc<crate::effects::Meters>>),
    Instrument(#[allow(dead_code)] Box<Instrument>),
    Clip(#[allow(dead_code)] Arc<Clip>),
    Tempo(#[allow(dead_code)] Box<TempoMap>),
}

/// Not yet recording.
pub(crate) const NO_RECORD_START: u64 = u64::MAX;

/// State the audio thread publishes for the UI to read.
pub(crate) struct Shared {
    pub position: AtomicU64,
    pub playing: AtomicBool,
    pub counting_in: AtomicBool,
    /// f32 bits. The audio thread raises them; the UI resets them when it reads.
    pub peak_l: AtomicU32,
    pub peak_r: AtomicU32,
    /// f32 bits: smoothed fraction of the buffer's time budget spent processing.
    pub load: AtomicU32,
    /// Callbacks that took longer than their time budget, which the user hears as crackles.
    pub overruns: AtomicU64,
    /// Timeline position of the first recorded sample, or `NO_RECORD_START`.
    pub record_start: AtomicU64,
    /// Input level (f32 bits), raised by the audio thread, reset by the UI.
    pub input_peak_l: AtomicU32,
    pub input_peak_r: AtomicU32,
    /// Recorded samples lost because the disk writer fell behind.
    pub record_dropped: AtomicU64,
    /// Post-fader level per track, then per bus (f32 bits, left then right),
    /// raised by the audio thread and reset by the UI.
    pub track_peaks: Vec<AtomicU32>,
    /// The live track's instrument keys (bit masks, low then high 64 notes):
    /// held now, and struck since the UI last read them (it resets those).
    pub keys_held: [AtomicU64; 2],
    pub keys_struck: [AtomicU64; 2],
    /// How hard each of those keys was last played (1..=127).
    pub key_velocities: Vec<AtomicU8>,
    /// A browser preview is playing.
    pub previewing: AtomicBool,
    /// The last `INPUT_SCOPE` input samples (mono, f32 bits) in a ring, and
    /// how many have been written, for the tuner.
    pub input_scope: Vec<AtomicU32>,
    pub input_scope_pos: AtomicUsize,
}

/// Input samples kept for the tuner: enough for two cycles of a low E bass string.
pub const INPUT_SCOPE: usize = 4096;

impl Default for Shared {
    fn default() -> Self {
        Self {
            position: AtomicU64::new(0),
            playing: AtomicBool::new(false),
            counting_in: AtomicBool::new(false),
            peak_l: AtomicU32::new(0),
            peak_r: AtomicU32::new(0),
            load: AtomicU32::new(0),
            overruns: AtomicU64::new(0),
            record_start: AtomicU64::new(NO_RECORD_START),
            input_peak_l: AtomicU32::new(0),
            input_peak_r: AtomicU32::new(0),
            record_dropped: AtomicU64::new(0),
            track_peaks: (0..(MAX_TRACKS + MAX_BUSES) * 2).map(|_| AtomicU32::new(0)).collect(),
            keys_held: [AtomicU64::new(0), AtomicU64::new(0)],
            keys_struck: [AtomicU64::new(0), AtomicU64::new(0)],
            key_velocities: (0..128).map(|_| AtomicU8::new(0)).collect(),
            previewing: AtomicBool::new(false),
            input_scope: (0..INPUT_SCOPE).map(|_| AtomicU32::new(0)).collect(),
            input_scope_pos: AtomicUsize::new(0),
        }
    }
}

pub(crate) fn raise_peak(peak: &AtomicU32, value: f32) {
    swap_f32_while(peak, value, |new, old| new > old);
}

/// Stores `value` in `cell` (an f32's bits) for as long as `better(value, current)`
/// holds, retrying if another thread changes it in between. Lock-free.
pub(crate) fn swap_f32_while(cell: &AtomicU32, value: f32, better: impl Fn(f32, f32) -> bool) {
    let mut old = cell.load(Relaxed);
    while better(value, f32::from_bits(old)) {
        match cell.compare_exchange_weak(old, value.to_bits(), Relaxed, Relaxed) {
            Ok(_) => break,
            Err(now) => old = now,
        }
    }
}

/// A short pitched blip for the metronome: higher on the first beat of a bar.
struct Click {
    phase: f32,
    inc: f32,
    env: f32,
    decay: f32,
    level: f32,
}

impl Click {
    fn new(sample_rate: f32) -> Self {
        // Falls to -60 dB in about 40 ms: crisp, and never smears into the next beat.
        Self { phase: 0.0, inc: 0.0, env: 0.0, decay: (0.001f32.ln() / (0.04 * sample_rate)).exp(), level: 0.0 }
    }

    fn trigger(&mut self, accent: bool, sample_rate: f32, volume: f32) {
        let freq = if accent { 1760.0 } else { 1320.0 };
        self.inc = TAU * freq / sample_rate;
        self.phase = 0.0;
        self.env = 1.0;
        self.level = volume * if accent { 0.5 } else { 0.35 };
    }

    fn next(&mut self) -> f32 {
        if self.env < 0.0005 {
            return 0.0;
        }
        let s = self.phase.sin() * self.env * self.level;
        self.phase = (self.phase + self.inc) % TAU;
        self.env *= self.decay;
        s
    }
}

pub(crate) struct Processor {
    commands: rtrb::Consumer<Command>,
    garbage: rtrb::Producer<Garbage>,
    shared: Arc<Shared>,
    sample_rate: f32,

    input: Option<rtrb::Consumer<f32>>,
    record: rtrb::Producer<f32>,
    recording: bool,
    record_started: bool,
    monitor: bool,

    arrangement: Arc<Arrangement>,
    mixer: Mixer,
    effects: EffectBank,
    instruments: InstrumentBank,
    /// Notes played live, from the UI and MIDI input; drained every callback.
    live_rx: rtrb::Consumer<LiveNote>,
    live: Vec<LiveNote>,
    live_track: Option<usize>,
    /// Live notes with their positions, out to the UI while `capture` is on.
    played: rtrb::Producer<PlayedNote>,
    capture: bool,
    // Per-block scratch, allocated once: the timeline position of each frame
    // (or SILENT), the start/stop envelope times volume, what's added after
    // the mix (click, monitoring, test tone), and the mix itself.
    positions: Vec<u64>,
    fades: Vec<f32>,
    vols: Vec<f32>,
    extra_l: Vec<f32>,
    extra_r: Vec<f32>,
    mix_l: Vec<f32>,
    mix_r: Vec<f32>,
    playing: bool,
    pos: usize,

    /// The tempo map, and the meter where playback started (for the count-in).
    tempo: Box<TempoMap>,
    meter: Meter,
    samples_per_beat: f64,
    /// The next metronome click: its timeline sample, its beat, and whether it starts a bar.
    next_click: usize,
    click_beat: f64,
    click_down: bool,
    metronome: bool,
    click_volume: f32,
    click: Click,
    count_in_left: usize,
    count_in_pos: usize,

    cycle: bool,
    cycle_start: usize,
    cycle_end: usize,

    volume: f32,
    gain: f32,
    /// 0..1 envelope applied when starting and stopping, so playback never clicks.
    fade: f32,
    fade_step: f32,

    tone_phase: f32,
    tone_left: usize,
    tone_len: usize,

    /// The browser's preview: the sound, how far into it, and whether it is
    /// still waiting for the next bar.
    preview: Option<Arc<Clip>>,
    preview_pos: usize,
    preview_wait: bool,

    /// Metronome clicks so far (accented ones counted separately). Only tests read these.
    pub(crate) clicks: u64,
    pub(crate) accents: u64,
}

impl Processor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        commands: rtrb::Consumer<Command>,
        garbage: rtrb::Producer<Garbage>,
        shared: Arc<Shared>,
        sample_rate: u32,
        // Allocated here, on the UI thread, never on the audio thread.
        arrangement: Arc<Arrangement>,
        record: rtrb::Producer<f32>,
        live_rx: rtrb::Consumer<LiveNote>,
        played: rtrb::Producer<PlayedNote>,
    ) -> Self {
        let meter = Meter::default();
        let samples_per_beat = meter.samples_per_beat(sample_rate);
        let sample_rate = sample_rate as f32;
        Self {
            commands,
            garbage,
            shared,
            sample_rate,
            input: None,
            record,
            recording: false,
            record_started: false,
            monitor: false,
            arrangement,
            mixer: Mixer::new(),
            effects: EffectBank::default(),
            instruments: InstrumentBank::default(),
            live_rx,
            live: Vec::with_capacity(256),
            live_track: None,
            played,
            capture: false,
            positions: vec![SILENT; BLOCK],
            fades: vec![0.0; BLOCK],
            vols: vec![0.0; BLOCK],
            extra_l: vec![0.0; BLOCK],
            extra_r: vec![0.0; BLOCK],
            mix_l: vec![0.0; BLOCK],
            mix_r: vec![0.0; BLOCK],
            playing: false,
            pos: 0,
            tempo: Box::new(TempoMap::constant(meter)),
            meter,
            samples_per_beat,
            next_click: 0,
            click_beat: 0.0,
            click_down: true,
            metronome: false,
            click_volume: 0.8,
            click: Click::new(sample_rate),
            count_in_left: 0,
            count_in_pos: 0,
            cycle: false,
            cycle_start: 0,
            cycle_end: 0,
            volume: 1.0,
            gain: 1.0,
            fade: 0.0,
            fade_step: 1.0 / (0.005 * sample_rate),
            tone_phase: 0.0,
            tone_left: 0,
            tone_len: sample_rate as usize,
            preview: None,
            preview_pos: 0,
            preview_wait: false,
            clicks: 0,
            accents: 0,
        }
    }

    pub fn process<T: Sample + FromSample<f32>>(&mut self, out: &mut [T], channels: usize) {
        let started = Instant::now();
        self.drain_commands();
        let frames_now = out.len() / channels.max(1);
        self.trim_input_backlog(frames_now);

        // Live notes since the last callback (they play from its first block).
        self.live.clear();
        while self.live.len() < self.live.capacity()
            && let Ok(note) = self.live_rx.pop()
        {
            self.live.push(note);
        }
        if self.capture && self.playing {
            // During a count-in the playhead waits at the start, so early notes land there.
            for &note in &self.live {
                let _ = self.played.push(PlayedNote { pos: self.pos as u64, note });
            }
        }

        let (mut peak_l, mut peak_r) = (0.0f32, 0.0f32);
        let channels = channels.max(1);
        for (b, block) in out.chunks_mut(BLOCK * channels).enumerate() {
            let n = block.len() / channels;
            // 1. Transport: where each frame plays, clicks, recording.
            for k in 0..n {
                let (pos, fade, vol, el, er) = self.next_frame();
                self.positions[k] = pos;
                self.fades[k] = fade;
                self.vols[k] = vol;
                self.extra_l[k] = el;
                self.extra_r[k] = er;
            }
            // 2. The tracks.
            let live: &[LiveNote] = if b == 0 { &self.live } else { &[] };
            self.mixer.mix(
                &self.arrangement,
                &mut self.effects,
                &mut self.instruments,
                self.sample_rate,
                &Block { positions: &self.positions[..n], fades: &self.fades[..n], live, live_track: self.live_track },
                &mut self.mix_l[..n],
                &mut self.mix_r[..n],
            );
            // 3. Out to the device.
            for (k, frame) in block.chunks_exact_mut(channels).enumerate() {
                let l = self.mix_l[k] * self.vols[k] + self.extra_l[k];
                let r = self.mix_r[k] * self.vols[k] + self.extra_r[k];
                peak_l = peak_l.max(l.abs());
                peak_r = peak_r.max(r.abs());
                if channels == 1 {
                    frame[0] = T::from_sample((l + r) * 0.5);
                } else {
                    frame[0] = T::from_sample(l);
                    frame[1] = T::from_sample(r);
                    for s in &mut frame[2..] {
                        *s = T::EQUILIBRIUM;
                    }
                }
            }
        }

        let s = &self.shared;
        let (tracks, buses) = (self.arrangement.tracks.len(), self.arrangement.buses.len());
        self.mixer.take_peaks(tracks, buses, |i, l, r| {
            raise_peak(&s.track_peaks[i * 2], l);
            raise_peak(&s.track_peaks[i * 2 + 1], r);
        });
        let (held, struck, velocities) = self.mixer.take_keys();
        if struck != 0 {
            for key in (0..128).filter(|k| struck >> k & 1 == 1) {
                s.key_velocities[key].store(velocities[key], Relaxed);
            }
        }
        s.keys_held[0].store(held as u64, Relaxed);
        s.keys_held[1].store((held >> 64) as u64, Relaxed);
        s.keys_struck[0].fetch_or(struck as u64, Relaxed);
        s.keys_struck[1].fetch_or((struck >> 64) as u64, Relaxed);
        s.position.store(self.pos as u64, Relaxed);
        s.playing.store(self.playing, Relaxed);
        s.counting_in.store(self.count_in_left > 0, Relaxed);
        raise_peak(&s.peak_l, peak_l);
        raise_peak(&s.peak_r, peak_r);

        let frames = out.len() / channels.max(1);
        let budget = frames as f32 / self.sample_rate;
        let used = started.elapsed().as_secs_f32() / budget;
        let load = f32::from_bits(s.load.load(Relaxed)) * 0.9 + used * 0.1;
        s.load.store(load.to_bits(), Relaxed);
        if used > 1.0 {
            s.overruns.fetch_add(1, Relaxed);
        }
    }

    /// Finds the first metronome beat at or after the playhead.
    fn schedule_click(&mut self) {
        let rate = self.sample_rate as u32;
        let beat = self.tempo.samples_to_beats(self.pos as u64, rate);
        let (b, down) = self.tempo.next_click(beat);
        self.click_beat = b;
        self.click_down = down;
        self.next_click = self.tempo.beats_to_samples(b, rate) as usize;
    }

    /// The tempo and time signature at the playhead, for the count-in.
    fn local_meter(&mut self) {
        let rate = self.sample_rate as u32;
        self.meter = self.tempo.meter_at(self.tempo.samples_to_beats(self.pos as u64, rate));
        self.samples_per_beat = self.meter.samples_per_beat(rate);
    }

    fn drain_commands(&mut self) {
        while let Ok(cmd) = self.commands.pop() {
            match cmd {
                Command::Play { count_in_bars } => {
                    if self.playing {
                        continue;
                    }
                    self.playing = true;
                    self.local_meter();
                    self.schedule_click();
                    let bar = self.samples_per_beat * f64::from(self.meter.numerator.max(1));
                    self.count_in_left = (bar * f64::from(count_in_bars)).round() as usize;
                    self.count_in_pos = 0;
                }
                Command::Stop => {
                    self.playing = false;
                    self.count_in_left = 0;
                }
                Command::Seek(frame) => {
                    self.pos = frame as usize;
                    self.schedule_click();
                }
                Command::SetCycle { on, start, end } => {
                    self.cycle = on && end > start;
                    self.cycle_start = start as usize;
                    self.cycle_end = end as usize;
                }
                Command::SetTempo(map) => {
                    let old = std::mem::replace(&mut self.tempo, map);
                    let _ = self.garbage.push(Garbage::Tempo(old));
                    self.local_meter();
                    self.schedule_click();
                }
                Command::SetMetronome { on, volume } => {
                    self.metronome = on;
                    self.click_volume = volume.clamp(0.0, 1.5);
                    self.schedule_click();
                }
                Command::SetVolume(v) => self.volume = v,
                Command::SetArrangement(next) => {
                    let old = std::mem::replace(&mut self.arrangement, next);
                    // Hand the old arrangement to the UI thread to free. The queue
                    // only fills if the UI stops collecting, and then it is freed
                    // here as a last resort.
                    let _ = self.garbage.push(Garbage::Arrangement(old));
                }
                Command::SetInput(next) => {
                    if let Some(old) = std::mem::replace(&mut self.input, next) {
                        let _ = self.garbage.push(Garbage::Input(old));
                    }
                }
                Command::SetRecording(on) => {
                    self.recording = on;
                    self.record_started = false;
                    if on {
                        self.shared.record_start.store(NO_RECORD_START, Relaxed);
                    }
                }
                Command::SetMonitor(on) => self.monitor = on,
                Command::AddEffect(id, effect, meters) => {
                    if let Err((effect, meters)) = self.effects.add(id, effect, meters) {
                        let _ = self.garbage.push(Garbage::Effect(effect, meters));
                    }
                }
                Command::AddInstrument(id, instrument) => {
                    if let Err(instrument) = self.instruments.add(id, instrument) {
                        let _ = self.garbage.push(Garbage::Instrument(instrument));
                    }
                }
                Command::RemoveInstrument(id) => {
                    if let Some(instrument) = self.instruments.remove(id) {
                        let _ = self.garbage.push(Garbage::Instrument(instrument));
                    }
                }
                Command::SetLiveTrack(track) => self.live_track = track,
                Command::SetCaptureNotes(on) => self.capture = on,
                Command::RemoveEffect(id) => {
                    if let Some((effect, meters)) = self.effects.remove(id) {
                        let _ = self.garbage.push(Garbage::Effect(effect, meters));
                    }
                }
                Command::TestTone => {
                    self.tone_left = self.tone_len;
                    self.tone_phase = 0.0;
                }
                Command::Preview { clip, sync } => {
                    if let Some(old) = std::mem::replace(&mut self.preview, clip) {
                        let _ = self.garbage.push(Garbage::Clip(old));
                    }
                    self.preview_pos = 0;
                    self.preview_wait = sync;
                    self.shared.previewing.store(self.preview.is_some(), Relaxed);
                }
            }
        }
    }

    /// Keeps input latency low: if more input is queued than a few buffers'
    /// worth (the input and output clocks drift apart), skip the oldest.
    fn trim_input_backlog(&mut self, frames: usize) {
        let Some(input) = &mut self.input else { return };
        let keep = (frames * 3 + 256) * 2;
        let queued = input.slots();
        if queued > keep
            && let Ok(chunk) = input.read_chunk(queued - keep)
        {
            chunk.commit_all();
        }
    }

    /// The next live input frame, or silence when none has arrived.
    #[inline]
    fn next_input(&mut self) -> (f32, f32) {
        match &mut self.input {
            Some(input) if input.slots() >= 2 => match (input.pop(), input.pop()) {
                (Ok(l), Ok(r)) => (l, r),
                _ => (0.0, 0.0),
            },
            _ => (0.0, 0.0),
        }
    }

    fn click(&mut self, accent: bool) {
        self.click.trigger(accent, self.sample_rate, self.click_volume);
        self.clicks += 1;
        if accent {
            self.accents += 1;
        }
    }

    /// Advances the transport by one frame. Returns the timeline position to
    /// mix (or `SILENT`), the start/stop fade for the song, the master
    /// volume, and what to add on top of the mix.
    fn next_frame(&mut self) -> (u64, f32, f32, f32, f32) {
        let fade_target = if self.playing && self.count_in_left == 0 { 1.0 } else { 0.0 };
        if self.fade < fade_target {
            self.fade = (self.fade + self.fade_step).min(1.0);
        } else if self.fade > fade_target {
            self.fade = (self.fade - self.fade_step).max(0.0);
        }
        // Smooth volume changes over a few milliseconds so slider moves don't crackle.
        self.gain += (self.volume - self.gain) * 0.002;

        let (mut l, mut r) = (0.0, 0.0);
        let (mut mix_pos, mut mix_fade) = (SILENT, 0.0);
        let num = i64::from(self.meter.numerator.max(1));

        let (il, ir) = self.next_input();
        if self.input.is_some() {
            raise_peak(&self.shared.input_peak_l, il.abs());
            raise_peak(&self.shared.input_peak_r, ir.abs());
            let at = self.shared.input_scope_pos.load(Relaxed);
            self.shared.input_scope[at % INPUT_SCOPE].store(((il + ir) * 0.5).to_bits(), Relaxed);
            self.shared.input_scope_pos.store(at.wrapping_add(1), Relaxed);
        }

        if self.playing && self.count_in_left > 0 {
            // Count-in: the metronome plays bars of clicks while the playhead waits.
            let beat = (self.count_in_pos as f64 / self.samples_per_beat).floor() as i64;
            if self.count_in_pos == 0 || beat != ((self.count_in_pos - 1) as f64 / self.samples_per_beat).floor() as i64 {
                self.click(beat % num == 0);
            }
            self.count_in_pos += 1;
            self.count_in_left -= 1;
            if self.count_in_left == 0 {
                self.schedule_click();
            }
        } else if self.fade > 0.0 {
            mix_pos = self.pos as u64;
            mix_fade = self.fade;
            if self.playing && self.metronome && self.pos >= self.next_click {
                self.click(self.click_down);
                // The next beat, from the map (a tempo or signature change may come first).
                let rate = self.sample_rate as u32;
                let (b, down) = self.tempo.next_click(self.click_beat + 1e-4);
                self.click_beat = b;
                self.click_down = down;
                self.next_click = self.tempo.beats_to_samples(b, rate) as usize;
            }
            // Record while the transport runs (after any count-in).
            if self.recording && self.playing {
                if !self.record_started {
                    self.record_started = true;
                    self.shared.record_start.store(self.pos as u64, Relaxed);
                }
                if self.record.slots() >= 2 {
                    let _ = self.record.push(il);
                    let _ = self.record.push(ir);
                } else {
                    self.shared.record_dropped.fetch_add(1, Relaxed);
                }
            }
            self.pos += 1;
            // Cycle is ignored while recording, so a take is one continuous file.
            if self.cycle && !self.recording && self.pos >= self.cycle_end {
                self.pos = self.cycle_start;
                self.schedule_click();
            }
        }

        if self.monitor {
            l += il * self.gain;
            r += ir * self.gain;
        }

        let c = self.click.next();
        l += c;
        r += c;

        if self.tone_left > 0 {
            let done = self.tone_len - self.tone_left;
            let ramp = (0.01 * self.sample_rate) as usize;
            let env = (done.min(self.tone_left).min(ramp) as f32) / ramp as f32;
            let s = self.tone_phase.sin() * 0.2 * env;
            self.tone_phase = (self.tone_phase + TAU * 440.0 / self.sample_rate) % TAU;
            self.tone_left -= 1;
            l += s;
            r += s;
        }

        if let Some(clip) = &self.preview {
            // In time: wait for the start of a bar while playing.
            if self.preview_wait {
                let rate = self.sample_rate as u32;
                let beat = self.tempo.samples_to_beats(self.pos as u64, rate);
                let bar = self.tempo.bar_start(self.tempo.bar_at(beat + 1e-6));
                let at_bar = self.tempo.beats_to_samples(bar, rate) == self.pos as u64;
                if !self.playing || self.count_in_left > 0 || at_bar {
                    self.preview_wait = false;
                }
            }
            if !self.preview_wait {
                let i = self.preview_pos;
                if i < clip.left.len() {
                    // A short fade in, so a preview never starts with a click.
                    let env = (i as f32 / 64.0).min(1.0) * 0.8 * self.gain;
                    l += clip.left[i] * env;
                    r += clip.right[i] * env;
                    self.preview_pos += 1;
                } else if let Some(done) = self.preview.take() {
                    let _ = self.garbage.push(Garbage::Clip(done));
                    self.shared.previewing.store(false, Relaxed);
                }
            }
        }

        (mix_pos, mix_fade, self.gain, l, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // At 1000 Hz and 120 BPM, one beat is exactly 500 samples.
    const RATE: u32 = 1000;

    use crate::arrangement::{PlayRegion, PlayTrack};
    use crate::clip::Clip;

    type Setup = (Processor, rtrb::Producer<Command>, rtrb::Consumer<Garbage>, Arc<Shared>);

    fn audio(frames: usize, l: f32, r: f32) -> Arc<Clip> {
        Arc::new(Clip { name: "test".into(), sample_rate: RATE, left: vec![l; frames], right: vec![r; frames] })
    }

    fn one_track(audio: Arc<Clip>, gain: f32, pan: f32, audible: bool) -> PlayTrack {
        let len = audio.frames() as u64;
        PlayTrack { gain, pan, audible, sends: vec![], inserts: vec![], automation: vec![], instrument: None, notes: vec![], regions: vec![PlayRegion { audio, start: 0, offset: 0, len, fade_in: 0, fade_out: 0, gain: 1.0, reverse: false }] }
    }

    fn setup(clip_frames: usize) -> Setup {
        let shared = Arc::new(Shared::default());
        let (mut tx, rx) = rtrb::RingBuffer::new(16);
        let (gtx, grx) = rtrb::RingBuffer::new(4);
        let (record, _record_rx) = rtrb::RingBuffer::new(16);
        let (_live_tx, live_rx) = rtrb::RingBuffer::new(4);
        let (played, _played_rx) = rtrb::RingBuffer::new(4);
        let mut p = Processor::new(rx, gtx, shared.clone(), RATE, Arc::new(Arrangement::default()), record, live_rx, played);
        // Skip the click-free fades so sample values are easy to check.
        p.fade_step = 1.0;
        p.mixer.set_all_gains(1.0);
        if clip_frames > 0 {
            let track = one_track(audio(clip_frames, 0.5, -0.5), 1.0, 0.0, true);
            let _ = tx.push(Command::SetArrangement(Arc::new(Arrangement { tracks: vec![track], ..Default::default() })));
        }
        (p, tx, grx, shared)
    }

    fn run(p: &mut Processor, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0f32; frames * 2];
        p.process(&mut out, 2);
        out
    }

    fn play(tx: &mut rtrb::Producer<Command>) {
        let _ = tx.push(Command::Play { count_in_bars: 0 });
    }

    #[test]
    fn previews_play_alone_and_wait_for_the_bar_when_in_time() {
        let (mut p, mut tx, mut garbage, shared) = setup(0);
        let _ = tx.push(Command::Preview { clip: Some(audio(200, 1.0, 1.0)), sync: true });
        let out = run(&mut p, 100);
        assert!(out[2 * 99] > 0.7, "stopped: starts at once");
        assert!(shared.previewing.load(Relaxed));
        run(&mut p, 200);
        assert!(!shared.previewing.load(Relaxed), "ends with the sound");
        assert!(garbage.pop().is_ok(), "and goes back to the UI thread");

        // Playing at beat 1 of 4/4 (500 samples a beat): it waits for bar 2 at 2000.
        let _ = tx.push(Command::Seek(1000));
        play(&mut tx);
        let _ = tx.push(Command::Preview { clip: Some(audio(2000, 1.0, 1.0)), sync: true });
        let out = run(&mut p, 1100);
        assert!(out[2 * 900].abs() < 1e-6, "still waiting");
        assert!(out[2 * 1099] > 0.7, "started on the bar");
        let _ = tx.push(Command::Preview { clip: None, sync: false });
        let out = run(&mut p, 10);
        assert!(out.iter().all(|s| s.abs() < 1e-6), "stopped");
    }

    #[test]
    fn silent_until_play() {
        let (mut p, _tx, _g, shared) = setup(100);
        assert!(run(&mut p, 10).iter().all(|&s| s == 0.0));
        assert_eq!(shared.position.load(Relaxed), 0);
    }

    #[test]
    fn plays_stereo_and_advances() {
        let (mut p, mut tx, _g, shared) = setup(100);
        play(&mut tx);
        let out = run(&mut p, 10);
        assert_eq!((out[0], out[1]), (0.5, -0.5));
        assert_eq!(shared.position.load(Relaxed), 10);
        assert!(shared.playing.load(Relaxed));
    }

    #[test]
    fn transport_runs_past_the_audio_and_on_an_empty_timeline() {
        let (mut p, mut tx, _g, shared) = setup(20);
        play(&mut tx);
        let out = run(&mut p, 30);
        assert_eq!(out[2 * 25], 0.0, "silence after the clip ends");
        assert!(shared.playing.load(Relaxed), "a DAW keeps playing past the last region");
        assert_eq!(shared.position.load(Relaxed), 30);

        let (mut empty, mut tx, _g, shared) = setup(0);
        play(&mut tx);
        run(&mut empty, 50);
        assert_eq!(shared.position.load(Relaxed), 50);
    }

    #[test]
    fn cycle_loops_between_its_bounds() {
        let (mut p, mut tx, _g, shared) = setup(1000);
        let _ = tx.push(Command::SetCycle { on: true, start: 100, end: 300 });
        let _ = tx.push(Command::Seek(100));
        play(&mut tx);
        run(&mut p, 250);
        assert_eq!(shared.position.load(Relaxed), 150);
    }

    #[test]
    fn metronome_clicks_once_per_beat_with_accented_downbeats() {
        let (mut p, mut tx, _g, _shared) = setup(0);
        let _ = tx.push(Command::SetMetronome { on: true, volume: 1.0 });
        play(&mut tx);
        run(&mut p, 4000); // two bars of 4/4
        assert_eq!(p.clicks, 8);
        assert_eq!(p.accents, 2);
    }

    #[test]
    fn metronome_follows_the_time_signature() {
        let (mut p, mut tx, _g, _shared) = setup(0);
        // 3/4 at 120: beats of 500 samples, bars of 1500.
        let _ = tx.push(Command::SetTempo(Box::new(TempoMap::constant(Meter { bpm: 120.0, numerator: 3, denominator: 4 }))));
        let _ = tx.push(Command::SetMetronome { on: true, volume: 1.0 });
        play(&mut tx);
        run(&mut p, 3000);
        assert_eq!((p.clicks, p.accents), (6, 2));
    }

    #[test]
    fn metronome_follows_tempo_and_signature_changes() {
        use crate::time::{SigChange, TempoPoint};
        let (mut p, mut tx, _g, _shared) = setup(0);
        // setup runs at 1000 Hz: at 120 BPM a beat is 500 samples. One bar of 4/4
        // (2000 samples), then 60 BPM (1000-sample beats) and 3/4 from bar 2.
        let map = TempoMap::new(
            Meter::default(),
            &[TempoPoint { beat: 4.0, bpm: 60.0 }],
            &[SigChange { bar: 2, numerator: 3, denominator: 4 }],
        );
        let _ = tx.push(Command::SetTempo(Box::new(map)));
        let _ = tx.push(Command::SetMetronome { on: true, volume: 1.0 });
        play(&mut tx);
        // 2000 samples of 4/4 (4 clicks) + 6000 of 3/4 at 60 (two bars, 6 clicks).
        run(&mut p, 8000);
        assert_eq!((p.clicks, p.accents), (10, 3));
    }

    #[test]
    fn count_in_holds_the_playhead_then_starts() {
        let (mut p, mut tx, _g, shared) = setup(0);
        let _ = tx.push(Command::Play { count_in_bars: 1 });
        run(&mut p, 1000);
        assert!(shared.counting_in.load(Relaxed));
        assert_eq!(shared.position.load(Relaxed), 0);
        run(&mut p, 1500); // count-in ends at 2000
        assert!(!shared.counting_in.load(Relaxed));
        assert_eq!(shared.position.load(Relaxed), 500);
        assert_eq!(p.accents, 1, "the count-in bar starts on an accent");
        assert_eq!(p.clicks, 4, "metronome off: only the count-in clicks");
    }

    #[test]
    fn replaced_arrangement_is_freed_off_the_audio_thread() {
        let (mut p, mut tx, mut garbage, _shared) = setup(20);
        run(&mut p, 1);
        let _ = tx.push(Command::SetArrangement(Arc::new(Arrangement::default())));
        run(&mut p, 1);
        assert!(garbage.pop().is_ok(), "old arrangement handed back to the UI thread");
    }

    #[test]
    fn seek_moves_anywhere_on_the_timeline() {
        let (mut p, mut tx, _g, shared) = setup(20);
        let _ = tx.push(Command::Seek(1_000));
        run(&mut p, 1);
        assert_eq!(shared.position.load(Relaxed), 1_000);
    }

    #[test]
    fn tracks_are_summed() {
        let (mut p, mut tx, _g, _s) = setup(0);
        let tracks = vec![
            one_track(audio(100, 0.25, 0.25), 1.0, 0.0, true),
            one_track(audio(100, 0.5, 0.5), 1.0, 0.0, true),
        ];
        let _ = tx.push(Command::SetArrangement(Arc::new(Arrangement { tracks, ..Default::default() })));
        play(&mut tx);
        let out = run(&mut p, 4);
        assert!((out[0] - 0.75).abs() < 1e-6 && (out[1] - 0.75).abs() < 1e-6);
    }

    #[test]
    fn muted_tracks_fade_to_silence() {
        let (mut p, mut tx, _g, _s) = setup(0);
        let tracks = vec![one_track(audio(10_000, 0.5, 0.5), 1.0, 0.0, false)];
        let _ = tx.push(Command::SetArrangement(Arc::new(Arrangement { tracks, ..Default::default() })));
        play(&mut tx);
        let out = run(&mut p, 5_000);
        assert!(out[0] > 0.0, "the gain glides down rather than cutting");
        assert!(out[out.len() - 1].abs() < 1e-4, "and ends silent");
    }

    #[test]
    fn regions_start_where_they_are_placed() {
        let (mut p, mut tx, _g, _s) = setup(0);
        let mut track = one_track(audio(10, 1.0, 1.0), 1.0, 0.0, true);
        track.regions[0].start = 5;
        let _ = tx.push(Command::SetArrangement(Arc::new(Arrangement { tracks: vec![track], ..Default::default() })));
        play(&mut tx);
        let out = run(&mut p, 8);
        assert_eq!(out[2 * 4], 0.0);
        assert_eq!(out[2 * 5], 1.0);
    }

    fn with_input(p: &mut Processor, tx: &mut rtrb::Producer<Command>, frames: usize) -> rtrb::Consumer<f32> {
        // Replace the record queue with one we can read, and feed a ramp as input.
        let (record, record_rx) = rtrb::RingBuffer::new(frames * 2 + 16);
        p.record = record;
        let (mut input_tx, input_rx) = rtrb::RingBuffer::new(frames * 2 + 16);
        for i in 0..frames {
            let _ = input_tx.push(i as f32);
            let _ = input_tx.push(-(i as f32));
        }
        let _ = tx.push(Command::SetInput(Some(input_rx)));
        record_rx
    }

    #[test]
    fn recording_starts_at_the_playhead_and_skips_the_count_in() {
        let (mut p, mut tx, _g, shared) = setup(0);
        let mut recorded = with_input(&mut p, &mut tx, 3000);
        let _ = tx.push(Command::Seek(100));
        let _ = tx.push(Command::SetRecording(true));
        let _ = tx.push(Command::Play { count_in_bars: 1 }); // 2000 samples of count-in
        run(&mut p, 2500);
        assert_eq!(shared.record_start.load(Relaxed), 100, "first recorded sample's timeline position");
        let n = recorded.slots();
        assert_eq!(n, 500 * 2, "only the samples after the count-in are recorded");
        // The first recorded frame is the input that arrived right as playback started.
        assert_eq!(recorded.pop().unwrap(), 2000.0);
        assert_eq!(recorded.pop().unwrap(), -2000.0);
    }

    #[test]
    fn nothing_is_recorded_while_stopped() {
        let (mut p, mut tx, _g, shared) = setup(0);
        let recorded = with_input(&mut p, &mut tx, 100);
        let _ = tx.push(Command::SetRecording(true));
        run(&mut p, 50);
        assert_eq!(recorded.slots(), 0);
        assert_eq!(shared.record_start.load(Relaxed), NO_RECORD_START);
    }

    #[test]
    fn monitoring_plays_the_input() {
        let (mut p, mut tx, _g, _shared) = setup(0);
        let _recorded = with_input(&mut p, &mut tx, 10);
        let _ = tx.push(Command::SetMonitor(true));
        let out = run(&mut p, 4);
        assert_eq!((out[2], out[3]), (1.0, -1.0));
    }

    #[test]
    fn plays_through_a_bus() {
        use crate::arrangement::PlayBus;
        let (mut p, mut tx, _g, shared) = setup(0);
        let mut track = one_track(audio(2000, 0.5, 0.5), 1.0, 0.0, true);
        track.sends = vec![(0, 0.5)];
        let arrangement = Arrangement { tracks: vec![track], buses: vec![PlayBus { gain: 1.0, pan: 0.0, audible: true, inserts: vec![] }], ..Default::default() };
        let _ = tx.push(Command::SetArrangement(Arc::new(arrangement)));
        play(&mut tx);
        let out = run(&mut p, 1500);
        assert!((out[2 * 1200] - 0.75).abs() < 1e-3, "{}", out[2 * 1200]);
        assert_eq!(shared.position.load(Relaxed), 1500);
    }

    #[test]
    fn captures_live_notes_with_their_position_while_playing() {
        let shared = Arc::new(Shared::default());
        let (mut tx, rx) = rtrb::RingBuffer::new(16);
        let (gtx, _grx) = rtrb::RingBuffer::new(4);
        let (record, _record_rx) = rtrb::RingBuffer::new(16);
        let (mut live_tx, live_rx) = rtrb::RingBuffer::new(4);
        let (played, mut played_rx) = rtrb::RingBuffer::new(4);
        let mut p = Processor::new(rx, gtx, shared, RATE, Arc::new(Arrangement::default()), record, live_rx, played);
        let note = LiveNote { on: true, key: 60, velocity: 100 };
        // Not captured while stopped, or with capture off.
        let _ = live_tx.push(note);
        let _ = tx.push(Command::SetCaptureNotes(true));
        run(&mut p, 100);
        assert!(played_rx.pop().is_err());
        play(&mut tx);
        run(&mut p, 100);
        let _ = live_tx.push(note);
        run(&mut p, 100);
        assert_eq!(played_rx.pop(), Ok(PlayedNote { pos: 100, note }));
        let _ = tx.push(Command::SetCaptureNotes(false));
        let _ = live_tx.push(note);
        run(&mut p, 100);
        assert!(played_rx.pop().is_err());
    }
}
