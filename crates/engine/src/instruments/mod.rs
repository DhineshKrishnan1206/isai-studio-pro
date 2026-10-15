//! Built-in software instruments, played by MIDI notes.
//!
//! Like effects, an instrument's settings travel in the arrangement
//! (`PlayInstrument`) and its running state (voices) lives in an
//! `InstrumentBank` on the audio thread, keyed by the instrument's id.

pub mod drums;
mod piano;
pub mod sampler;
mod synth;

use std::sync::Arc;

pub use drums::Drums;
pub use piano::Piano;
pub use sampler::Sampler;
pub use synth::Synth;

use crate::clip::Clip;

use crate::effects::ParamDef;

/// Most parameters any instrument has.
pub const MAX_INST_PARAMS: usize = 48;
/// The "key" of a live sustain-pedal event (`on` = pedal down). Real notes are 0..=127.
pub const SUSTAIN_PEDAL: u8 = 128;
/// Most instruments the audio thread keeps at once.
pub const MAX_INSTRUMENTS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentKind {
    Synth,
    Piano,
    Drums,
    Sampler,
    /// A third-party plugin instrument.
    Plugin,
}

const fn p(key: &'static str, label: &'static str, min: f32, max: f32, default: f32, unit: &'static str, log: bool) -> ParamDef {
    ParamDef { key, label, min, max, default, unit, log, choices: &[] }
}

pub const WAVES: &[&str] = &["Saw", "Square", "Triangle", "Sine", "Pulse", "Noise"];
pub const FILTER_TYPES: &[&str] = &["Low-pass", "High-pass", "Band-pass"];
pub const PLAY_MODES: &[&str] = &["Poly", "Mono", "Legato"];
pub const LFO_TARGETS: &[&str] = &["Pitch", "Filter", "Volume", "Pan"];
pub const UNISON_VOICES: &[&str] = &["1", "2", "3", "4", "5", "6", "7"];
pub const OCTAVES: &[&str] = &["-2", "-1", "0", "+1", "+2"];
pub const SUB_OCTAVES: &[&str] = &["-1 oct", "-2 oct"];
pub const LFO_SHAPES: &[&str] = &["Sine", "Triangle", "Square", "Saw", "Random"];

const fn choice(key: &'static str, label: &'static str, choices: &'static [&'static str], default: f32) -> ParamDef {
    ParamDef { key, label, min: 0.0, max: (choices.len() - 1) as f32, default, unit: "", log: false, choices }
}

// The first ten keep the original synth's keys, so older projects still load.
const SYNTH_PARAMS: &[ParamDef] = &[
    choice("wave", "Wave A", WAVES, 0.0),
    p("detune", "Fine B", -50.0, 50.0, 8.0, "ct", false),
    p("cutoff", "Cutoff", 40.0, 18000.0, 2400.0, "Hz", true),
    p("resonance", "Resonance", 0.0, 100.0, 20.0, "%", false),
    p("env_amount", "Env amount", 0.0, 100.0, 40.0, "%", false),
    p("attack", "Attack", 1.0, 5000.0, 5.0, "ms", true),
    p("decay", "Decay", 5.0, 5000.0, 300.0, "ms", true),
    p("sustain", "Sustain", 0.0, 100.0, 70.0, "%", false),
    p("release", "Release", 5.0, 8000.0, 300.0, "ms", true),
    p("volume", "Volume", -24.0, 6.0, -6.0, "dB", false),
    choice("wave_b", "Wave B", WAVES, 0.0),
    p("semi_b", "Pitch B", -24.0, 24.0, 0.0, "st", false),
    p("mix", "Mix A/B", 0.0, 100.0, 50.0, "%", false),
    p("pulse_width", "Pulse width", 5.0, 95.0, 50.0, "%", false),
    p("sub", "Sub", 0.0, 100.0, 0.0, "%", false),
    p("noise", "Noise", 0.0, 100.0, 0.0, "%", false),
    choice("unison", "Unison", UNISON_VOICES, 0.0),
    p("uni_detune", "Detune", 0.0, 100.0, 20.0, "ct", false),
    p("spread", "Spread", 0.0, 100.0, 60.0, "%", false),
    choice("filter_type", "Filter", FILTER_TYPES, 0.0),
    p("drive", "Drive", 0.0, 100.0, 0.0, "%", false),
    p("keytrack", "Key track", 0.0, 100.0, 50.0, "%", false),
    p("f_attack", "Attack", 1.0, 5000.0, 5.0, "ms", true),
    p("f_decay", "Decay", 5.0, 5000.0, 300.0, "ms", true),
    p("f_sustain", "Sustain", 0.0, 100.0, 0.0, "%", false),
    p("f_release", "Release", 5.0, 8000.0, 300.0, "ms", true),
    p("glide", "Glide", 0.0, 2000.0, 0.0, "ms", false),
    choice("mono", "Voices", PLAY_MODES, 0.0),
    p("lfo_rate", "LFO rate", 0.05, 20.0, 4.0, "Hz", true),
    p("lfo_depth", "LFO depth", 0.0, 100.0, 0.0, "%", false),
    choice("lfo_target", "LFO to", LFO_TARGETS, 1.0),
    p("velocity", "Velocity", 0.0, 100.0, 50.0, "%", false),
    choice("oct_a", "Octave A", OCTAVES, 2.0),
    p("fine_a", "Fine A", -50.0, 50.0, 0.0, "ct", false),
    choice("sub_oct", "Sub octave", SUB_OCTAVES, 0.0),
    choice("lfo_shape", "LFO shape", LFO_SHAPES, 0.0),
    p("lfo_delay", "LFO delay", 0.0, 3000.0, 0.0, "ms", false),
];

// Kit, volume and drive, then four knobs per piece in `drums::PIECES` order.
const DRUM_PARAMS: &[ParamDef] = &[
    choice("kit", "Kit", drums::KITS, 0.0),
    p("volume", "Volume", -24.0, 6.0, -3.0, "dB", false),
    p("drive", "Drive", 0.0, 100.0, 0.0, "%", false),
    p("kick_level", "Kick level", -60.0, 6.0, 0.0, "dB", false),
    p("kick_tune", "Kick tune", -12.0, 12.0, 0.0, "st", false),
    p("kick_decay", "Kick length", 0.1, 4.0, 1.0, "×", true),
    p("kick_pan", "Kick pan", -1.0, 1.0, 0.0, "", false),
    p("snare_level", "Snare level", -60.0, 6.0, 0.0, "dB", false),
    p("snare_tune", "Snare tune", -12.0, 12.0, 0.0, "st", false),
    p("snare_decay", "Snare length", 0.1, 4.0, 1.0, "×", true),
    p("snare_pan", "Snare pan", -1.0, 1.0, 0.0, "", false),
    p("clap_level", "Clap level", -60.0, 6.0, 0.0, "dB", false),
    p("clap_tune", "Clap tune", -12.0, 12.0, 0.0, "st", false),
    p("clap_decay", "Clap length", 0.1, 4.0, 1.0, "×", true),
    p("clap_pan", "Clap pan", -1.0, 1.0, 0.1, "", false),
    p("rim_level", "Rim level", -60.0, 6.0, 0.0, "dB", false),
    p("rim_tune", "Rim tune", -12.0, 12.0, 0.0, "st", false),
    p("rim_decay", "Rim length", 0.1, 4.0, 1.0, "×", true),
    p("rim_pan", "Rim pan", -1.0, 1.0, -0.2, "", false),
    p("chh_level", "Closed hat level", -60.0, 6.0, 0.0, "dB", false),
    p("chh_tune", "Closed hat tune", -12.0, 12.0, 0.0, "st", false),
    p("chh_decay", "Closed hat length", 0.1, 4.0, 1.0, "×", true),
    p("chh_pan", "Closed hat pan", -1.0, 1.0, 0.3, "", false),
    p("ohh_level", "Open hat level", -60.0, 6.0, 0.0, "dB", false),
    p("ohh_tune", "Open hat tune", -12.0, 12.0, 0.0, "st", false),
    p("ohh_decay", "Open hat length", 0.1, 4.0, 1.0, "×", true),
    p("ohh_pan", "Open hat pan", -1.0, 1.0, 0.3, "", false),
    p("ltom_level", "Low tom level", -60.0, 6.0, 0.0, "dB", false),
    p("ltom_tune", "Low tom tune", -12.0, 12.0, 0.0, "st", false),
    p("ltom_decay", "Low tom length", 0.1, 4.0, 1.0, "×", true),
    p("ltom_pan", "Low tom pan", -1.0, 1.0, -0.4, "", false),
    p("htom_level", "High tom level", -60.0, 6.0, 0.0, "dB", false),
    p("htom_tune", "High tom tune", -12.0, 12.0, 0.0, "st", false),
    p("htom_decay", "High tom length", 0.1, 4.0, 1.0, "×", true),
    p("htom_pan", "High tom pan", -1.0, 1.0, 0.35, "", false),
    p("crash_level", "Crash level", -60.0, 6.0, -3.0, "dB", false),
    p("crash_tune", "Crash tune", -12.0, 12.0, 0.0, "st", false),
    p("crash_decay", "Crash length", 0.1, 4.0, 1.0, "×", true),
    p("crash_pan", "Crash pan", -1.0, 1.0, -0.3, "", false),
    p("cowbell_level", "Cowbell level", -60.0, 6.0, -3.0, "dB", false),
    p("cowbell_tune", "Cowbell tune", -12.0, 12.0, 0.0, "st", false),
    p("cowbell_decay", "Cowbell length", 0.1, 4.0, 1.0, "×", true),
    p("cowbell_pan", "Cowbell pan", -1.0, 1.0, 0.2, "", false),
];

const OFF_ON: &[&str] = &["Off", "On"];
pub const SAMPLER_MODES: &[&str] = &["Keys", "Slices"];
pub const SLICE_COUNTS: &[&str] = &["4", "8", "16", "32"];

const SAMPLER_PARAMS: &[ParamDef] = &[
    choice("mode", "Mode", SAMPLER_MODES, 0.0),
    p("root", "Root key", 24.0, 96.0, 60.0, "key", false),
    p("start", "Start", 0.0, 100.0, 0.0, "%", false),
    p("end", "End", 0.0, 100.0, 100.0, "%", false),
    choice("loop", "Loop", OFF_ON, 0.0),
    p("loop_start", "Loop start", 0.0, 100.0, 0.0, "%", false),
    p("attack", "Attack", 1.0, 5000.0, 2.0, "ms", true),
    p("decay", "Decay", 5.0, 5000.0, 300.0, "ms", true),
    p("sustain", "Sustain", 0.0, 100.0, 100.0, "%", false),
    p("release", "Release", 5.0, 8000.0, 150.0, "ms", true),
    p("tune", "Tune", -24.0, 24.0, 0.0, "st", false),
    p("fine", "Fine", -50.0, 50.0, 0.0, "ct", false),
    p("cutoff", "Cutoff", 40.0, 20000.0, 20000.0, "Hz", true),
    p("volume", "Volume", -24.0, 6.0, -3.0, "dB", false),
    p("velocity", "Velocity", 0.0, 100.0, 60.0, "%", false),
    choice("reverse", "Reverse", OFF_ON, 0.0),
    choice("slice_count", "Slices", SLICE_COUNTS, 1.0),
];

pub const PIANO_TONES: &[&str] = &["Grand", "Electric", "Felt", "Reed", "Funk", "Toy", "Music box", "Celesta", "Honky-tonk"];
pub const QUALITIES: &[&str] = &["Full", "Light"];
pub const PAN_MODES: &[&str] = &["Tremolo", "Auto-pan"];

const PIANO_PARAMS: &[ParamDef] = &[
    ParamDef { key: "tone", label: "Tone", min: 0.0, max: 8.0, default: 0.0, unit: "", log: false, choices: PIANO_TONES },
    p("brightness", "Brightness", 0.0, 100.0, 55.0, "%", false),
    p("decay", "Decay", 0.5, 20.0, 6.0, "s", true),
    p("release", "Release", 20.0, 2000.0, 250.0, "ms", true),
    p("velocity", "Dynamics", 0.0, 100.0, 70.0, "%", false),
    p("mod_rate", "Rate", 0.1, 8.0, 0.8, "Hz", true),
    p("mod_depth", "Depth", 0.0, 100.0, 0.0, "%", false),
    p("trem_rate", "Rate", 0.5, 12.0, 4.5, "Hz", true),
    p("trem_depth", "Depth", 0.0, 100.0, 0.0, "%", false),
    ParamDef { key: "autopan", label: "Mode", min: 0.0, max: 1.0, default: 0.0, unit: "", log: false, choices: PAN_MODES },
    p("volume", "Volume", -24.0, 6.0, -4.0, "dB", false),
    p("space", "Space", 0.0, 100.0, 15.0, "%", false),
    p("stereo", "Width", 0.0, 100.0, 60.0, "%", false),
    p("strings", "Strings", 0.0, 100.0, 50.0, "%", false),
    p("body", "Body", 0.0, 100.0, 40.0, "%", false),
    p("mechanics", "Mechanics", 0.0, 100.0, 35.0, "%", false),
    p("resonance", "Resonance", 0.0, 100.0, 40.0, "%", false),
    p("drive", "Drive", 0.0, 100.0, 0.0, "%", false),
    ParamDef { key: "quality", label: "Quality", min: 0.0, max: 1.0, default: 0.0, unit: "", log: false, choices: QUALITIES },
];

impl InstrumentKind {
    pub const ALL: [InstrumentKind; 4] = [InstrumentKind::Synth, InstrumentKind::Piano, InstrumentKind::Drums, InstrumentKind::Sampler];

    pub fn key(self) -> &'static str {
        match self {
            InstrumentKind::Synth => "synth",
            InstrumentKind::Piano => "piano",
            InstrumentKind::Drums => "drums",
            InstrumentKind::Sampler => "sampler",
            InstrumentKind::Plugin => "plugin",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            InstrumentKind::Synth => "Isai Synth",
            InstrumentKind::Piano => "Isai Piano",
            InstrumentKind::Drums => "Isai Drums",
            InstrumentKind::Sampler => "Isai Sampler",
            InstrumentKind::Plugin => "Plugin",
        }
    }

    pub fn params(self) -> &'static [ParamDef] {
        match self {
            InstrumentKind::Synth => SYNTH_PARAMS,
            InstrumentKind::Piano => PIANO_PARAMS,
            InstrumentKind::Drums => DRUM_PARAMS,
            InstrumentKind::Sampler => SAMPLER_PARAMS,
            InstrumentKind::Plugin => &[],
        }
    }

    pub fn defaults(self) -> [f32; MAX_INST_PARAMS] {
        let mut out = [0.0; MAX_INST_PARAMS];
        for (o, d) in out.iter_mut().zip(self.params()) {
            *o = d.default;
        }
        out
    }
}

/// A track's instrument, as the arrangement describes it.
#[derive(Clone)]
pub struct PlayInstrument {
    pub id: u64,
    pub kind: InstrumentKind,
    pub params: [f32; MAX_INST_PARAMS],
    /// The sampler's sound, once loaded.
    pub sample: Option<Arc<Clip>>,
    /// Where the sampler's slices start, in samples.
    pub slices: Vec<u64>,
    /// The loaded plugin, for `InstrumentKind::Plugin`.
    pub plugin: Option<crate::plugin::PluginRef>,
    /// The song's tempo where this block plays (for plugins' tempo-synced sounds).
    pub bpm: f64,
}

impl PlayInstrument {
    /// An instrument with no sample (everything but the sampler).
    pub fn new(id: u64, kind: InstrumentKind, params: [f32; MAX_INST_PARAMS]) -> Self {
        Self { id, kind, params, sample: None, slices: Vec::new(), plugin: None, bpm: 120.0 }
    }
}

/// A note on the timeline, in samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayNote {
    pub start: u64,
    pub end: u64,
    pub key: u8,
    /// 1..=127
    pub velocity: u8,
}

/// A note played live (MIDI keyboard, computer keyboard, piano-roll preview).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveNote {
    pub on: bool,
    pub key: u8,
    pub velocity: u8,
}

/// A live note with the timeline position (in samples) it was heard at,
/// captured while recording MIDI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayedNote {
    pub pos: u64,
    pub note: LiveNote,
}

// Always kept in a Box (see `InstrumentBank`), so the size difference costs nothing.
#[allow(clippy::large_enum_variant)]
pub enum Instrument {
    Synth(Synth),
    Piano(Piano),
    Drums(Drums),
    Sampler(Sampler),
    Plugin(crate::plugin::PluginInstrument),
}

impl Instrument {
    /// Builds an instrument (allocating its voices). Call off the audio thread.
    pub fn new(kind: InstrumentKind) -> Self {
        match kind {
            InstrumentKind::Synth => Instrument::Synth(Synth::new()),
            InstrumentKind::Piano => Instrument::Piano(Piano::new()),
            InstrumentKind::Drums => Instrument::Drums(Drums::new()),
            InstrumentKind::Sampler => Instrument::Sampler(Sampler::new()),
            InstrumentKind::Plugin => Instrument::Plugin(crate::plugin::PluginInstrument::new(None, false)),
        }
    }

    pub fn kind(&self) -> InstrumentKind {
        match self {
            Instrument::Synth(_) => InstrumentKind::Synth,
            Instrument::Piano(_) => InstrumentKind::Piano,
            Instrument::Drums(_) => InstrumentKind::Drums,
            Instrument::Sampler(_) => InstrumentKind::Sampler,
            Instrument::Plugin(_) => InstrumentKind::Plugin,
        }
    }

    /// Plays `notes` (sorted by start) at the timeline `positions` of this
    /// block, plus `live` notes at its start, adding into `l` / `r`.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        inst: &PlayInstrument,
        notes: &[PlayNote],
        positions: &[u64],
        live: &[LiveNote],
        sample_rate: f32,
        l: &mut [f32],
        r: &mut [f32],
    ) {
        let params = &inst.params;
        match self {
            Instrument::Synth(s) => s.render(params, notes, positions, live, sample_rate, l, r),
            Instrument::Piano(s) => s.render(params, notes, positions, live, sample_rate, l, r),
            Instrument::Drums(s) => s.render(params, notes, positions, live, sample_rate, l, r),
            Instrument::Sampler(s) => s.render(params, inst.sample.as_deref(), &inst.slices, notes, positions, live, sample_rate, l, r),
            Instrument::Plugin(p) => {
                let transport = crate::plugin::Transport { bpm: inst.bpm, playing: positions.iter().any(|&p| p != crate::mixer::SILENT), sample_rate };
                p.render(notes, positions, live, &transport, l, r)
            }
        }
    }

    /// Keys held down right now, and keys struck since the last call (which
    /// it then forgets), as bit masks (bit `n` = MIDI note `n`).
    pub fn keys(&mut self) -> (u128, u128) {
        match self {
            Instrument::Synth(s) => s.keys(),
            Instrument::Piano(s) => s.keys(),
            Instrument::Drums(s) => s.keys(),
            Instrument::Sampler(s) => s.keys(),
            Instrument::Plugin(s) => s.keys(),
        }
    }

    /// How hard each key was last played (1..=127, 0 = not yet).
    pub fn velocities(&self) -> &[u8; 128] {
        match self {
            Instrument::Synth(s) => s.velocities(),
            Instrument::Piano(s) => s.velocities(),
            Instrument::Drums(s) => s.velocities(),
            Instrument::Sampler(s) => s.velocities(),
            Instrument::Plugin(s) => s.velocities(),
        }
    }

    /// True while any voice is sounding (including release tails).
    pub fn is_active(&self) -> bool {
        match self {
            Instrument::Synth(s) => s.is_active(),
            Instrument::Piano(s) => s.is_active(),
            Instrument::Drums(s) => s.is_active(),
            Instrument::Sampler(s) => s.is_active(),
            // A plugin may ring on (reverb tails, arpeggiators): keep it running.
            Instrument::Plugin(_) => true,
        }
    }

    pub fn reset(&mut self) {
        match self {
            Instrument::Synth(s) => s.reset(),
            Instrument::Piano(s) => s.reset(),
            Instrument::Drums(s) => s.reset(),
            Instrument::Sampler(s) => s.reset(),
            Instrument::Plugin(s) => s.reset(),
        }
    }
}

/// The instruments the audio thread runs, keyed by id.
pub struct InstrumentBank {
    slots: Vec<(u64, Box<Instrument>)>,
}

impl Default for InstrumentBank {
    fn default() -> Self {
        Self { slots: Vec::with_capacity(MAX_INSTRUMENTS) }
    }
}

impl InstrumentBank {
    /// A bank with a fresh instrument for each of `instruments` (for export).
    pub fn for_instruments<'a>(instruments: impl IntoIterator<Item = &'a PlayInstrument>) -> Self {
        let mut bank = Self::default();
        for i in instruments {
            let instrument = match (&i.plugin, i.kind) {
                (Some(p), InstrumentKind::Plugin) => Instrument::Plugin(crate::plugin::PluginInstrument::new(Some(p.clone()), true)),
                _ => Instrument::new(i.kind),
            };
            let _ = bank.add(i.id, Box::new(instrument));
        }
        bank
    }

    pub fn add(&mut self, id: u64, instrument: Box<Instrument>) -> Result<(), Box<Instrument>> {
        if self.slots.len() >= MAX_INSTRUMENTS || self.slots.iter().any(|(i, _)| *i == id) {
            return Err(instrument);
        }
        self.slots.push((id, instrument));
        Ok(())
    }

    pub fn remove(&mut self, id: u64) -> Option<Box<Instrument>> {
        let i = self.slots.iter().position(|(s, _)| *s == id)?;
        Some(self.slots.swap_remove(i).1)
    }

    pub fn get_mut(&mut self, id: u64, kind: InstrumentKind) -> Option<&mut Instrument> {
        self.slots.iter_mut().find(|(i, inst)| *i == id && inst.kind() == kind).map(|(_, inst)| &mut **inst)
    }

    pub fn any_active(&self) -> bool {
        self.slots.iter().any(|(_, i)| i.is_active())
    }
}
