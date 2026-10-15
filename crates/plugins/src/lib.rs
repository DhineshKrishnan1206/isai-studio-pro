//! Third-party plugins for Isai: finding them, loading them, and running
//! them inside the engine.
//!
//! Each loaded plugin implements the engine's [`ExternalPlugin`], so the
//! engine never sees a plugin SDK. VST3 is supported now (through the
//! MIT-licensed `vst3-host`); CLAP and Audio Units come next.
//!
//! A plugin is shared between the UI thread (its window, its saved state)
//! and the audio thread behind one lock. The audio thread only ever *tries*
//! that lock: if the UI is busy with the plugin, that one block skips it.

mod base64;
mod vst3;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use base64::{decode as decode_state, encode as encode_state};
pub use vst3::Vst3Plugin;

/// A plugin format.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Vst3,
}

impl Format {
    pub fn label(self) -> &'static str {
        match self {
            Format::Vst3 => "VST3",
        }
    }
}

/// A plugin found on this computer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PluginInfo {
    pub format: Format,
    /// The plugin's bundle or file.
    pub path: PathBuf,
    /// The plugin's own id inside the bundle (a bundle can hold several).
    pub uid: String,
    pub name: String,
    pub vendor: String,
    /// The plugin's category, e.g. "Instrument|Synth" or "Fx|Reverb".
    pub category: String,
    /// Plays notes (an instrument) rather than processing sound (an effect).
    pub instrument: bool,
    pub has_gui: bool,
}

/// Finds the VST3 plugins in the standard folders. Loading a plugin's code
/// can crash, so the app runs this in a separate process (`isai --scan-plugins`).
pub fn scan() -> Vec<PluginInfo> {
    let mut found = vst3::scan();
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.vendor.cmp(&b.vendor)));
    found.dedup_by(|a, b| a.path == b.path && a.uid == b.uid);
    found
}

/// What the scan prints and the app reads back.
pub fn scan_json() -> String {
    serde_json::to_string(&scan()).unwrap_or_else(|_| "[]".into())
}

pub fn parse_scan(json: &str) -> Vec<PluginInfo> {
    serde_json::from_str(json).unwrap_or_default()
}

/// Loads a plugin, ready to process at `sample_rate`.
pub fn load(info: &PluginInfo, sample_rate: f64) -> Result<Vst3Plugin, String> {
    match info.format {
        Format::Vst3 => Vst3Plugin::load(&info.path, &info.uid, sample_rate),
    }
}

/// Whether a plugin's file is still where it was.
pub fn exists(path: &Path) -> bool {
    path.exists()
}
