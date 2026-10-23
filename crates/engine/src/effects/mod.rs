//! Built-in insert effects: EQ, compressor, reverb, delay, limiter, chorus,
//! flanger, phaser, distortion, bitcrusher, gate and de-esser.
//!
//! Each effect's *settings* travel in the arrangement (`PlayInsert`), which
//! the UI rebuilds after every edit. Each effect's *state* (filter memory,
//! reverb tails, delay lines) lives in an `EffectBank` on the audio thread,
//! keyed by the insert's id, so edits never cut a tail off. New effects are
//! built on the UI thread (they allocate their buffers) and handed over.

pub(crate) mod biquad;
mod character;
pub mod compressor;
mod delay;
mod dynamics;
pub mod eq;
mod limiter;
mod meters;
mod modulation;
pub mod pitch;
pub mod reverb;

use std::sync::Arc;

pub use character::{Bitcrusher, DISTORTION_TYPES, Distortion};
pub use compressor::Compressor;
pub use delay::Delay;
pub use dynamics::{DeEsser, Gate};
pub use eq::Eq;
pub use limiter::Limiter;
pub use meters::{Meters, SCOPE};
pub use modulation::{Chorus, Flanger, PHASER_STAGES, Phaser};
pub use pitch::PitchCorrect;
pub use reverb::Reverb;

/// Most parameters any effect has.
pub const MAX_PARAMS: usize = 32;
/// Most effects the audio thread keeps at once.
pub const MAX_EFFECTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    Eq,
    Compressor,
    Reverb,
    Delay,
    Limiter,
    Chorus,
    Flanger,
    Phaser,
    Distortion,
    Bitcrusher,
    Gate,
    DeEsser,
    PitchCorrect,
    /// A third-party plugin (its own controls live in its window).
    Plugin,
}

/// One knob or switch on an effect.
#[derive(Debug)]
pub struct ParamDef {
    pub key: &'static str,
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    /// "dB", "Hz", "ms", "%", "" …
    pub unit: &'static str,
    /// Logarithmic knob travel (frequencies, times).
    pub log: bool,
    /// Named steps instead of a continuous value (index = value).
    pub choices: &'static [&'static str],
}

const fn p(
    key: &'static str,
    label: &'static str,
    min: f32,
    max: f32,
    default: f32,
    unit: &'static str,
    log: bool,
) -> ParamDef {
    ParamDef {
        key,
        label,
        min,
        max,
        default,
        unit,
        log,
        choices: &[],
    }
}

const OFF_ON: &[&str] = &["Off", "On"];
const SLOPES: &[&str] = &["6 dB/oct", "12 dB/oct", "24 dB/oct", "48 dB/oct"];
const fn switch(key: &'static str, label: &'static str, default: f32) -> ParamDef {
    ParamDef {
        key,
        label,
        min: 0.0,
        max: 1.0,
        default,
        unit: "",
        log: false,
        choices: OFF_ON,
    }
}
const fn slope(key: &'static str, label: &'static str) -> ParamDef {
    ParamDef {
        key,
        label,
        min: 0.0,
        max: 3.0,
        default: 2.0,
        unit: "",
        log: false,
        choices: SLOPES,
    }
}

/// Eight bands: low cut, low shelf, four bells, high shelf, high cut (see `eq::param`).
const EQ_PARAMS: &[ParamDef] = &[
    switch("lc_on", "Low cut", 0.0),
    p("lc_freq", "Low cut freq", 20.0, 2000.0, 30.0, "Hz", true),
    slope("lc_slope", "Low cut slope"),
    switch("ls_on", "Low shelf", 0.0),
    p("ls_freq", "Low shelf freq", 20.0, 2000.0, 100.0, "Hz", true),
    p("ls_gain", "Low shelf gain", -24.0, 24.0, 0.0, "dB", false),
    p("ls_q", "Low shelf Q", 0.3, 3.0, 0.71, "", true),
    switch("b1_on", "Band 1", 0.0),
    p("b1_freq", "Band 1 freq", 20.0, 20000.0, 250.0, "Hz", true),
    p("b1_gain", "Band 1 gain", -24.0, 24.0, 0.0, "dB", false),
    p("b1_q", "Band 1 Q", 0.1, 18.0, 1.0, "", true),
    switch("b2_on", "Band 2", 0.0),
    p("b2_freq", "Band 2 freq", 20.0, 20000.0, 800.0, "Hz", true),
    p("b2_gain", "Band 2 gain", -24.0, 24.0, 0.0, "dB", false),
    p("b2_q", "Band 2 Q", 0.1, 18.0, 1.0, "", true),
    switch("b3_on", "Band 3", 0.0),
    p("b3_freq", "Band 3 freq", 20.0, 20000.0, 2500.0, "Hz", true),
    p("b3_gain", "Band 3 gain", -24.0, 24.0, 0.0, "dB", false),
    p("b3_q", "Band 3 Q", 0.1, 18.0, 1.0, "", true),
    switch("b4_on", "Band 4", 0.0),
    p("b4_freq", "Band 4 freq", 20.0, 20000.0, 6000.0, "Hz", true),
    p("b4_gain", "Band 4 gain", -24.0, 24.0, 0.0, "dB", false),
    p("b4_q", "Band 4 Q", 0.1, 18.0, 1.0, "", true),
    switch("hs_on", "High shelf", 0.0),
    p(
        "hs_freq",
        "High shelf freq",
        1000.0,
        20000.0,
        8000.0,
        "Hz",
        true,
    ),
    p("hs_gain", "High shelf gain", -24.0, 24.0, 0.0, "dB", false),
    p("hs_q", "High shelf Q", 0.3, 3.0, 0.71, "", true),
    switch("hc_on", "High cut", 0.0),
    p(
        "hc_freq",
        "High cut freq",
        1000.0,
        20000.0,
        18000.0,
        "Hz",
        true,
    ),
    slope("hc_slope", "High cut slope"),
    p("output", "Output", -24.0, 24.0, 0.0, "dB", false),
];
const COMPRESSOR_PARAMS: &[ParamDef] = &[
    p("threshold", "Threshold", -60.0, 0.0, -18.0, "dB", false),
    p("ratio", "Ratio", 1.0, 20.0, 4.0, ":1", true),
    p("attack", "Attack", 0.05, 200.0, 10.0, "ms", true),
    p("release", "Release", 5.0, 2000.0, 120.0, "ms", true),
    p("knee", "Knee", 0.0, 24.0, 6.0, "dB", false),
    p("makeup", "Makeup", -12.0, 24.0, 0.0, "dB", false),
    switch("auto_makeup", "Auto gain", 1.0),
    ParamDef {
        key: "detect",
        label: "Detect",
        min: 0.0,
        max: 1.0,
        default: 0.0,
        unit: "",
        log: false,
        choices: &["Peak", "RMS"],
    },
    p("sc_hpf", "SC low cut", 20.0, 500.0, 20.0, "Hz", true),
    p("mix", "Mix", 0.0, 100.0, 100.0, "%", false),
];
const REVERB_PARAMS: &[ParamDef] = &[
    p("decay", "Decay", 0.2, 20.0, 2.2, "s", true),
    p("size", "Size", 0.0, 100.0, 60.0, "%", false),
    p("predelay", "Pre-delay", 0.0, 200.0, 15.0, "ms", false),
    p("damping", "High cut", 1000.0, 20000.0, 7000.0, "Hz", true),
    p("low_cut", "Low cut", 20.0, 1000.0, 120.0, "Hz", true),
    p("modulation", "Modulation", 0.0, 100.0, 35.0, "%", false),
    p("width", "Width", 0.0, 100.0, 100.0, "%", false),
    p("mix", "Mix", 0.0, 100.0, 25.0, "%", false),
];
/// Delay times that follow the tempo, as fractions of a whole note.
pub const DELAY_SYNC: &[&str] = &[
    "Off",
    "1/4",
    "1/8",
    "1/8 dotted",
    "1/16",
    "1/4 triplet",
    "1/2",
];
const DELAY_PARAMS: &[ParamDef] = &[
    p("time", "Time", 1.0, 2000.0, 375.0, "ms", true),
    ParamDef {
        key: "sync",
        label: "Sync",
        min: 0.0,
        max: 6.0,
        default: 2.0,
        unit: "",
        log: false,
        choices: DELAY_SYNC,
    },
    p("feedback", "Feedback", 0.0, 95.0, 35.0, "%", false),
    p("tone", "Tone", 1000.0, 20000.0, 6000.0, "Hz", true),
    ParamDef {
        key: "ping_pong",
        label: "Ping-pong",
        min: 0.0,
        max: 1.0,
        default: 0.0,
        unit: "",
        log: false,
        choices: &["Off", "On"],
    },
    p("mix", "Mix", 0.0, 100.0, 25.0, "%", false),
];
const CHORUS_PARAMS: &[ParamDef] = &[
    p("rate", "Rate", 0.05, 8.0, 0.8, "Hz", true),
    p("depth", "Depth", 0.0, 100.0, 45.0, "%", false),
    p("delay", "Delay", 2.0, 30.0, 12.0, "ms", false),
    p("width", "Width", 0.0, 100.0, 100.0, "%", false),
    p("mix", "Mix", 0.0, 100.0, 50.0, "%", false),
];
const FLANGER_PARAMS: &[ParamDef] = &[
    p("rate", "Rate", 0.05, 8.0, 0.25, "Hz", true),
    p("depth", "Depth", 0.0, 100.0, 70.0, "%", false),
    p("delay", "Delay", 0.5, 12.0, 4.0, "ms", true),
    p("feedback", "Feedback", -95.0, 95.0, 55.0, "%", false),
    p("mix", "Mix", 0.0, 100.0, 50.0, "%", false),
];
const PHASER_PARAMS: &[ParamDef] = &[
    p("rate", "Rate", 0.05, 8.0, 0.5, "Hz", true),
    p("depth", "Depth", 0.0, 100.0, 70.0, "%", false),
    p("centre", "Centre", 100.0, 5000.0, 700.0, "Hz", true),
    p("feedback", "Feedback", 0.0, 95.0, 45.0, "%", false),
    ParamDef {
        key: "stages",
        label: "Stages",
        min: 0.0,
        max: 3.0,
        default: 1.0,
        unit: "",
        log: false,
        choices: PHASER_STAGES,
    },
    p("mix", "Mix", 0.0, 100.0, 50.0, "%", false),
];
const DISTORTION_PARAMS: &[ParamDef] = &[
    p("drive", "Drive", 0.0, 48.0, 12.0, "dB", false),
    ParamDef {
        key: "type",
        label: "Type",
        min: 0.0,
        max: 4.0,
        default: 0.0,
        unit: "",
        log: false,
        choices: DISTORTION_TYPES,
    },
    p("tone", "Tone", 500.0, 20000.0, 9000.0, "Hz", true),
    p("output", "Output", -24.0, 12.0, -6.0, "dB", false),
    p("mix", "Mix", 0.0, 100.0, 100.0, "%", false),
];
const BITCRUSHER_PARAMS: &[ParamDef] = &[
    p("bits", "Bits", 1.0, 16.0, 8.0, "", false),
    p("rate", "Sample rate", 200.0, 48000.0, 11025.0, "Hz", true),
    p("mix", "Mix", 0.0, 100.0, 100.0, "%", false),
    p("output", "Output", -24.0, 12.0, 0.0, "dB", false),
];
const GATE_PARAMS: &[ParamDef] = &[
    p("threshold", "Threshold", -90.0, 0.0, -45.0, "dB", false),
    p("attack", "Attack", 0.05, 50.0, 1.0, "ms", true),
    p("hold", "Hold", 0.0, 500.0, 40.0, "ms", false),
    p("release", "Release", 5.0, 2000.0, 150.0, "ms", true),
    p("floor", "Floor", -90.0, 0.0, -80.0, "dB", false),
];
const DEESSER_PARAMS: &[ParamDef] = &[
    p("freq", "Frequency", 2000.0, 12000.0, 6000.0, "Hz", true),
    p("threshold", "Threshold", -60.0, 0.0, -28.0, "dB", false),
    p("amount", "Max cut", 0.0, 30.0, 10.0, "dB", false),
    switch("listen", "Listen", 0.0),
];
const PITCH_PARAMS: &[ParamDef] = &[
    ParamDef {
        key: "key",
        label: "Key",
        min: 0.0,
        max: 11.0,
        default: 0.0,
        unit: "",
        log: false,
        choices: pitch::KEYS,
    },
    ParamDef {
        key: "scale",
        label: "Scale",
        min: 0.0,
        max: 4.0,
        default: 0.0,
        unit: "",
        log: false,
        choices: pitch::SCALES,
    },
    p("speed", "Speed", 0.0, 400.0, 30.0, "ms", false),
    p("amount", "Amount", 0.0, 100.0, 100.0, "%", false),
    p("mix", "Mix", 0.0, 100.0, 100.0, "%", false),
];
const LIMITER_PARAMS: &[ParamDef] = &[
    p("gain", "Gain", 0.0, 24.0, 0.0, "dB", false),
    p("ceiling", "Ceiling", -12.0, 0.0, -1.0, "dB", false),
    p("release", "Release", 10.0, 1000.0, 100.0, "ms", true),
];

impl EffectKind {
    pub const ALL: [EffectKind; 13] = [
        EffectKind::Eq,
        EffectKind::Compressor,
        EffectKind::Reverb,
        EffectKind::Delay,
        EffectKind::Limiter,
        EffectKind::Chorus,
        EffectKind::Flanger,
        EffectKind::Phaser,
        EffectKind::Distortion,
        EffectKind::Bitcrusher,
        EffectKind::Gate,
        EffectKind::DeEsser,
        EffectKind::PitchCorrect,
    ];

    pub fn key(self) -> &'static str {
        match self {
            EffectKind::Eq => "eq",
            EffectKind::Compressor => "compressor",
            EffectKind::Reverb => "reverb",
            EffectKind::Delay => "delay",
            EffectKind::Limiter => "limiter",
            EffectKind::Chorus => "chorus",
            EffectKind::Flanger => "flanger",
            EffectKind::Phaser => "phaser",
            EffectKind::Distortion => "distortion",
            EffectKind::Bitcrusher => "bitcrusher",
            EffectKind::Gate => "gate",
            EffectKind::DeEsser => "deesser",
            EffectKind::PitchCorrect => "pitch",
            EffectKind::Plugin => "plugin",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            EffectKind::Eq => "Equalizer",
            EffectKind::Compressor => "Compressor",
            EffectKind::Reverb => "Reverb",
            EffectKind::Delay => "Delay",
            EffectKind::Limiter => "Limiter",
            EffectKind::Chorus => "Chorus",
            EffectKind::Flanger => "Flanger",
            EffectKind::Phaser => "Phaser",
            EffectKind::Distortion => "Distortion",
            EffectKind::Bitcrusher => "Bitcrusher",
            EffectKind::Gate => "Noise Gate",
            EffectKind::DeEsser => "De-esser",
            EffectKind::PitchCorrect => "Pitch Correct",
            EffectKind::Plugin => "Plugin",
        }
    }

    pub fn params(self) -> &'static [ParamDef] {
        match self {
            EffectKind::Eq => EQ_PARAMS,
            EffectKind::Compressor => COMPRESSOR_PARAMS,
            EffectKind::Reverb => REVERB_PARAMS,
            EffectKind::Delay => DELAY_PARAMS,
            EffectKind::Limiter => LIMITER_PARAMS,
            EffectKind::Chorus => CHORUS_PARAMS,
            EffectKind::Flanger => FLANGER_PARAMS,
            EffectKind::Phaser => PHASER_PARAMS,
            EffectKind::Distortion => DISTORTION_PARAMS,
            EffectKind::Bitcrusher => BITCRUSHER_PARAMS,
            EffectKind::Gate => GATE_PARAMS,
            EffectKind::DeEsser => DEESSER_PARAMS,
            EffectKind::PitchCorrect => PITCH_PARAMS,
            EffectKind::Plugin => &[],
        }
    }

    /// Default values, in `params()` order.
    pub fn defaults(self) -> [f32; MAX_PARAMS] {
        let mut out = [0.0; MAX_PARAMS];
        for (o, d) in out.iter_mut().zip(self.params()) {
            *o = d.default;
        }
        out
    }
}

/// What the effect needs to know about the song.
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub sample_rate: f32,
    /// Quarter notes per minute, for tempo-synced delays.
    pub bpm: f32,
}

/// An effect's running state.
#[allow(clippy::large_enum_variant)]
pub enum Effect {
    Eq(Eq),
    Compressor(Compressor),
    Reverb(Reverb),
    Delay(Delay),
    Limiter(Limiter),
    Chorus(Chorus),
    Flanger(Flanger),
    Phaser(Phaser),
    Distortion(Distortion),
    Bitcrusher(Bitcrusher),
    Gate(Gate),
    DeEsser(DeEsser),
    PitchCorrect(PitchCorrect),
    Plugin(crate::plugin::PluginEffect),
}

impl Effect {
    /// Builds an effect, allocating its buffers. Call off the audio thread.
    pub fn new(kind: EffectKind, sample_rate: u32) -> Self {
        let rate = sample_rate as f32;
        match kind {
            EffectKind::Eq => Effect::Eq(Eq::new()),
            EffectKind::Compressor => Effect::Compressor(Compressor::new()),
            EffectKind::Reverb => Effect::Reverb(Reverb::new(rate)),
            EffectKind::Delay => Effect::Delay(Delay::new(rate)),
            EffectKind::Limiter => Effect::Limiter(Limiter::new(rate)),
            EffectKind::Chorus => Effect::Chorus(Chorus::new(rate)),
            EffectKind::Flanger => Effect::Flanger(Flanger::new(rate)),
            EffectKind::Phaser => Effect::Phaser(Phaser::new()),
            EffectKind::Distortion => Effect::Distortion(Distortion::new()),
            EffectKind::Bitcrusher => Effect::Bitcrusher(Bitcrusher::new()),
            EffectKind::Gate => Effect::Gate(Gate::new()),
            EffectKind::DeEsser => Effect::DeEsser(DeEsser::new()),
            EffectKind::PitchCorrect => Effect::PitchCorrect(PitchCorrect::new(rate)),
            EffectKind::Plugin => Effect::Plugin(crate::plugin::PluginEffect::new(None, false)),
        }
    }

    pub fn kind(&self) -> EffectKind {
        match self {
            Effect::Eq(_) => EffectKind::Eq,
            Effect::Compressor(_) => EffectKind::Compressor,
            Effect::Reverb(_) => EffectKind::Reverb,
            Effect::Delay(_) => EffectKind::Delay,
            Effect::Limiter(_) => EffectKind::Limiter,
            Effect::Chorus(_) => EffectKind::Chorus,
            Effect::Flanger(_) => EffectKind::Flanger,
            Effect::Phaser(_) => EffectKind::Phaser,
            Effect::Distortion(_) => EffectKind::Distortion,
            Effect::Bitcrusher(_) => EffectKind::Bitcrusher,
            Effect::Gate(_) => EffectKind::Gate,
            Effect::DeEsser(_) => EffectKind::DeEsser,
            Effect::PitchCorrect(_) => EffectKind::PitchCorrect,
            Effect::Plugin(_) => EffectKind::Plugin,
        }
    }

    /// Processes a block of stereo audio in place.
    pub fn process(
        &mut self,
        params: &[f32; MAX_PARAMS],
        ctx: &Context,
        l: &mut [f32],
        r: &mut [f32],
    ) {
        match self {
            Effect::Eq(e) => e.process(params, ctx, l, r),
            Effect::Compressor(e) => e.process(params, ctx, l, r),
            Effect::Reverb(e) => e.process(params, ctx, l, r),
            Effect::Delay(e) => e.process(params, ctx, l, r),
            Effect::Limiter(e) => e.process(params, ctx, l, r),
            Effect::Chorus(e) => e.process(params, ctx, l, r),
            Effect::Flanger(e) => e.process(params, ctx, l, r),
            Effect::Phaser(e) => e.process(params, ctx, l, r),
            Effect::Distortion(e) => e.process(params, ctx, l, r),
            Effect::Bitcrusher(e) => e.process(params, ctx, l, r),
            Effect::Gate(e) => e.process(params, ctx, l, r),
            Effect::DeEsser(e) => e.process(params, ctx, l, r),
            Effect::PitchCorrect(e) => e.process(params, ctx, l, r),
            Effect::Plugin(e) => e.process(
                &crate::plugin::Transport {
                    bpm: f64::from(ctx.bpm),
                    playing: true,
                    sample_rate: ctx.sample_rate,
                },
                l,
                r,
            ),
        }
    }

    /// Gain reduction in the last block, in dB (0 for effects that don't reduce).
    pub fn reduction_db(&self) -> f32 {
        match self {
            Effect::Compressor(c) => c.reduction_db(),
            Effect::Limiter(l) => l.reduction_db(),
            Effect::Gate(g) => g.reduction_db(),
            Effect::DeEsser(d) => d.reduction_db(),
            _ => 0.0,
        }
    }

    /// Forgets all sound in flight (tails, filter memory).
    pub fn reset(&mut self) {
        match self {
            Effect::Eq(e) => e.reset(),
            Effect::Compressor(e) => e.reset(),
            Effect::Reverb(e) => e.reset(),
            Effect::Delay(e) => e.reset(),
            Effect::Limiter(e) => e.reset(),
            Effect::Chorus(e) => e.reset(),
            Effect::Flanger(e) => e.reset(),
            Effect::Phaser(e) => e.reset(),
            Effect::Distortion(e) => e.reset(),
            Effect::Bitcrusher(e) => e.reset(),
            Effect::Gate(e) => e.reset(),
            Effect::DeEsser(e) => e.reset(),
            Effect::PitchCorrect(e) => e.reset(),
            Effect::Plugin(_) => {}
        }
    }
}

/// An effect in a chain, as the arrangement describes it.
#[derive(Clone, Debug)]
pub struct PlayInsert {
    /// Stable id: which `EffectBank` entry runs it.
    pub id: u64,
    pub kind: EffectKind,
    pub enabled: bool,
    pub params: [f32; MAX_PARAMS],
    /// The loaded plugin, for `EffectKind::Plugin` inserts.
    pub plugin: Option<crate::plugin::PluginRef>,
}

/// The effects the audio thread runs, keyed by insert id. Never grows past
/// `MAX_EFFECTS`, so adding one never allocates on the audio thread.
pub struct EffectBank {
    slots: Vec<(u64, Box<Effect>, Option<Arc<Meters>>)>,
    /// True after a silent block reset everything, so it isn't repeated.
    quiet: bool,
}

impl Default for EffectBank {
    fn default() -> Self {
        Self {
            slots: Vec::with_capacity(MAX_EFFECTS),
            quiet: false,
        }
    }
}

impl EffectBank {
    /// A bank holding a fresh effect for every insert in `inserts` (for export).
    pub fn for_inserts<'a>(
        inserts: impl IntoIterator<Item = &'a PlayInsert>,
        sample_rate: u32,
    ) -> Self {
        let mut bank = Self::default();
        for i in inserts {
            // Plugins render offline here, so they're waited for.
            let effect = match (&i.plugin, i.kind) {
                (Some(p), EffectKind::Plugin) => {
                    Effect::Plugin(crate::plugin::PluginEffect::new(Some(p.clone()), true))
                }
                _ => Effect::new(i.kind, sample_rate),
            };
            let _ = bank.add(i.id, Box::new(effect), None);
        }
        bank
    }

    /// Adds an effect (with its meters, if anyone is watching); gives them
    /// back if the bank is full or the id is taken.
    #[allow(clippy::type_complexity)]
    pub fn add(
        &mut self,
        id: u64,
        effect: Box<Effect>,
        meters: Option<Arc<Meters>>,
    ) -> Result<(), (Box<Effect>, Option<Arc<Meters>>)> {
        if self.slots.len() >= MAX_EFFECTS || self.slots.iter().any(|(i, ..)| *i == id) {
            return Err((effect, meters));
        }
        self.slots.push((id, effect, meters));
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn remove(&mut self, id: u64) -> Option<(Box<Effect>, Option<Arc<Meters>>)> {
        let i = self.slots.iter().position(|(s, ..)| *s == id)?;
        let (_, effect, meters) = self.slots.swap_remove(i);
        Some((effect, meters))
    }

    pub fn contains(&self, id: u64) -> bool {
        self.slots.iter().any(|(s, ..)| *s == id)
    }

    /// Runs one effect, feeding its meters.
    fn run_one(
        slot: &mut (u64, Box<Effect>, Option<Arc<Meters>>),
        params: &[f32; MAX_PARAMS],
        ctx: &Context,
        l: &mut [f32],
        r: &mut [f32],
    ) {
        let (_, effect, meters) = slot;
        let scope = effect.kind() == EffectKind::Eq;
        if let Some(m) = meters {
            m.record_input(l, r, scope);
        }
        effect.process(params, ctx, l, r);
        if let Some(m) = meters {
            m.record_output(l, r, effect.reduction_db(), scope);
        }
    }

    /// Runs a chain of inserts over a block, in order. Disabled or missing
    /// effects are skipped (the sound passes through).
    pub fn run(&mut self, chain: &[PlayInsert], ctx: &Context, l: &mut [f32], r: &mut [f32]) {
        for insert in chain.iter().filter(|i| i.enabled) {
            if let Some(slot) = self
                .slots
                .iter_mut()
                .find(|(id, e, _)| *id == insert.id && e.kind() == insert.kind)
            {
                Self::run_one(slot, &insert.params, ctx, l, r);
            }
        }
    }

    /// Like `run`, but parameters with an automation lane take the lane's
    /// value at `pos`.
    pub fn run_automated(
        &mut self,
        chain: &[PlayInsert],
        lanes: &[crate::arrangement::PlayLane],
        pos: Option<u64>,
        ctx: &Context,
        l: &mut [f32],
        r: &mut [f32],
    ) {
        use crate::arrangement::AutoTarget;
        let Some(pos) = pos.filter(|_| {
            lanes
                .iter()
                .any(|l| matches!(l.target, AutoTarget::Param { .. }))
        }) else {
            return self.run(chain, ctx, l, r);
        };
        for insert in chain.iter().filter(|i| i.enabled) {
            let mut params = insert.params;
            for lane in lanes {
                if let AutoTarget::Param { insert: id, index } = lane.target
                    && id == insert.id
                    && index < MAX_PARAMS
                    && let Some(v) = lane.value_at(pos)
                {
                    params[index] = v;
                }
            }
            if let Some(slot) = self
                .slots
                .iter_mut()
                .find(|(id, e, _)| *id == insert.id && e.kind() == insert.kind)
            {
                Self::run_one(slot, &params, ctx, l, r);
            }
        }
    }

    /// Called once per block: when nothing is playing, clear every effect
    /// once so old tails don't come back when playback starts again.
    pub fn set_quiet(&mut self, quiet: bool) {
        if quiet && !self.quiet {
            for (_, e, _) in &mut self.slots {
                e.reset();
            }
        }
        self.quiet = quiet;
    }

    pub fn is_quiet(&self) -> bool {
        self.quiet
    }
}

/// Decibels to a linear gain.
#[inline]
pub(crate) fn db(v: f32) -> f32 {
    10f32.powf(v / 20.0)
}

/// Very small values to zero, so feedback paths never slow down on denormals.
#[inline]
pub(crate) fn flush(x: f32) -> f32 {
    if x.abs() < 1e-20 { 0.0 } else { x }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const CTX: Context = Context {
        sample_rate: 48_000.0,
        bpm: 120.0,
    };

    #[test]
    fn every_kind_has_sane_parameters() {
        for kind in EffectKind::ALL {
            assert_eq!(EffectKind::from_key(kind.key()), Some(kind));
            let params = kind.params();
            assert!(!params.is_empty() && params.len() <= MAX_PARAMS, "{kind:?}");
            for d in params {
                assert!(
                    d.min < d.max && (d.min..=d.max).contains(&d.default),
                    "{kind:?} {}",
                    d.key
                );
            }
        }
    }

    #[test]
    fn every_effect_is_stable_on_noise_and_silent_after_reset() {
        let mut noise = 0x1234_5678u32;
        let mut rand = move || {
            noise ^= noise << 13;
            noise ^= noise >> 17;
            noise ^= noise << 5;
            (noise as f32 / u32::MAX as f32) * 2.0 - 1.0
        };
        for kind in EffectKind::ALL {
            let mut e = Effect::new(kind, 48_000);
            let params = kind.defaults();
            for _ in 0..200 {
                let mut l: Vec<f32> = (0..256).map(|_| rand() * 0.5).collect();
                let mut r = l.clone();
                e.process(&params, &CTX, &mut l, &mut r);
                assert!(
                    l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 8.0),
                    "{kind:?} blew up"
                );
            }
            e.reset();
            let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
            e.process(&params, &CTX, &mut l, &mut r);
            assert!(
                l.iter().chain(&r).all(|s| s.abs() < 1e-6),
                "{kind:?} not silent after reset"
            );
        }
    }

    fn sine(hz: f32, n: usize, amp: f32) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (i as f32 * hz / 48_000.0 * std::f32::consts::TAU).sin())
            .collect()
    }

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    }

    fn run(kind: EffectKind, set: &[(&str, f32)], input: &[f32]) -> Vec<f32> {
        let mut params = kind.defaults();
        for (key, v) in set {
            let i = kind
                .params()
                .iter()
                .position(|d| d.key == *key)
                .expect("known key");
            params[i] = *v;
        }
        let mut e = Effect::new(kind, 48_000);
        let (mut l, mut r) = (input.to_vec(), input.to_vec());
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            e.process(&params, &CTX, a, b);
        }
        l
    }

    #[test]
    fn the_gate_silences_quiet_parts_and_passes_loud_ones() {
        let quiet = run(
            EffectKind::Gate,
            &[("threshold", -30.0)],
            &sine(200.0, 48_000, 0.005),
        );
        assert!(
            rms(&quiet[24_000..]) < 1e-4,
            "hiss under the threshold is gone"
        );
        let loud = run(
            EffectKind::Gate,
            &[("threshold", -30.0)],
            &sine(200.0, 48_000, 0.5),
        );
        assert!(
            (rms(&loud[24_000..]) - rms(&sine(200.0, 24_000, 0.5))).abs() < 0.01,
            "a loud note passes"
        );
    }

    #[test]
    fn the_deesser_turns_down_highs_but_not_lows() {
        let set = [("threshold", -30.0), ("amount", 12.0), ("freq", 4000.0)];
        let lows = run(EffectKind::DeEsser, &set, &sine(200.0, 24_000, 0.5));
        assert!(
            (rms(&lows[12_000..]) / rms(&sine(200.0, 12_000, 0.5)) - 1.0).abs() < 0.05,
            "lows untouched"
        );
        // With nothing to cut, the split adds back up flat, even right at the crossover.
        let flat = run(
            EffectKind::DeEsser,
            &[("threshold", 0.0)],
            &sine(6000.0, 24_000, 0.1),
        );
        assert!(
            (rms(&flat[12_000..]) / rms(&sine(6000.0, 12_000, 0.1)) - 1.0).abs() < 0.05,
            "flat at the split"
        );
        let s = run(EffectKind::DeEsser, &set, &sine(8000.0, 24_000, 0.5));
        assert!(
            rms(&s[12_000..]) < rms(&sine(8000.0, 12_000, 0.5)) * 0.4,
            "loud highs turned down"
        );
    }

    #[test]
    fn distortion_adds_harmonics_and_the_crusher_steps() {
        // A clean sine through hard clipping comes out squarer: more energy than a sine of its peak.
        let out = run(
            EffectKind::Distortion,
            &[
                ("type", 1.0),
                ("drive", 24.0),
                ("output", 0.0),
                ("tone", 20_000.0),
            ],
            &sine(100.0, 9600, 0.5),
        );
        let peak = out[4800..].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(rms(&out[4800..]) > peak * 0.8, "squared off");
        let crushed = run(
            EffectKind::Bitcrusher,
            &[("bits", 2.0), ("rate", 48_000.0)],
            &sine(100.0, 4800, 0.9),
        );
        let mut steps: Vec<i32> = crushed
            .iter()
            .map(|s| (s * 1000.0).round() as i32)
            .collect();
        steps.sort_unstable();
        steps.dedup();
        assert!(
            steps.len() <= 5,
            "2 bits: a handful of levels, got {}",
            steps.len()
        );
    }

    #[test]
    fn modulation_effects_keep_level_and_move() {
        for kind in [EffectKind::Chorus, EffectKind::Flanger, EffectKind::Phaser] {
            let input = sine(440.0, 96_000, 0.5);
            let out = run(kind, &[], &input);
            let level = rms(&out[48_000..]) / rms(&input[48_000..]);
            assert!(level > 0.3 && level < 1.5, "{kind:?} level {level}");
            assert!(
                out.iter()
                    .zip(&input)
                    .skip(48_000)
                    .any(|(a, b)| (a - b).abs() > 0.05),
                "{kind:?} changes the sound"
            );
        }
    }

    #[test]
    fn pitch_correct_pulls_a_sharp_note_onto_the_nearest_one() {
        fn hz(s: &[f32]) -> f32 {
            let crossings = s.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
            crossings as f32 / (s.len() as f32 / 48_000.0)
        }
        // 452 Hz is a little sharp of A (440 Hz).
        let input = sine(452.0, 96_000, 0.4);
        let out = run(EffectKind::PitchCorrect, &[("speed", 0.0)], &input);
        let got = hz(&out[48_000..]);
        assert!((got - 440.0).abs() < 4.0, "corrected to {got} Hz");
        let gentle = run(
            EffectKind::PitchCorrect,
            &[("speed", 0.0), ("amount", 0.0)],
            &input,
        );
        assert!(
            (hz(&gentle[48_000..]) - 452.0).abs() < 4.0,
            "amount 0 leaves it alone"
        );
        // In C major, A♯ (466 Hz) isn't allowed: it lands on A or B.
        let out = run(
            EffectKind::PitchCorrect,
            &[("speed", 0.0), ("scale", 1.0)],
            &sine(466.2, 96_000, 0.4),
        );
        let got = hz(&out[48_000..]);
        assert!(
            (got - 440.0).abs() < 5.0 || (got - 493.9).abs() < 5.0,
            "{got}"
        );
    }

    #[test]
    fn bank_runs_chains_in_order_and_skips_bypassed() {
        let mut bank = EffectBank::default();
        let mut limiter = PlayInsert {
            plugin: None,
            id: 7,
            kind: EffectKind::Limiter,
            enabled: true,
            params: EffectKind::Limiter.defaults(),
        };
        let meters = Arc::new(Meters::default());
        assert!(
            bank.add(
                7,
                Box::new(Effect::new(EffectKind::Limiter, 48_000)),
                Some(meters.clone())
            )
            .is_ok()
        );
        assert!(
            bank.add(7, Box::new(Effect::new(EffectKind::Limiter, 48_000)), None)
                .is_err(),
            "ids are unique"
        );
        // A loud block: the limiter keeps it under the -1 dB ceiling.
        let mut run = |chain: &[PlayInsert]| {
            let mut peak = 0.0f32;
            for _ in 0..20 {
                let (mut l, mut r) = (vec![1.5f32; 256], vec![-1.5f32; 256]);
                bank.run(chain, &CTX, &mut l, &mut r);
                peak = l.iter().chain(&r).fold(peak, |p, s| p.max(s.abs()));
            }
            peak
        };
        assert!(run(std::slice::from_ref(&limiter)) <= db(-1.0) + 1e-4);
        let (levels, reduction) = meters.take();
        assert_eq!(levels[0], 1.5, "input level");
        assert!(
            levels[2] <= db(-1.0) + 1e-4 && reduction < -3.0,
            "output and reduction: {levels:?} {reduction}"
        );
        limiter.enabled = false;
        assert_eq!(run(std::slice::from_ref(&limiter)), 1.5);
        assert!(bank.remove(7).is_some() && !bank.contains(7));
    }
}
