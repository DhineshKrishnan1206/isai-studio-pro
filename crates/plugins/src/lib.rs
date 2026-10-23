//! Third-party plugins for Isai: finding them, loading them, and running
//! them inside the engine.
//!
//! Each loaded plugin implements the engine's [`ExternalPlugin`], so the
//! engine never sees a plugin SDK. VST3 goes through the MIT-licensed
//! `vst3-host`, CLAP through `clack-host` (MIT/Apache-2.0).
//!
//! A plugin is shared between the UI thread (its window, its saved state)
//! and the audio thread behind one lock. The audio thread only ever *tries*
//! that lock: if the UI is busy with the plugin, that one block skips it.

#[cfg(target_os = "macos")]
mod au;
mod base64;
mod clap;
#[cfg(target_os = "macos")]
mod key_guard;
#[cfg(target_os = "macos")]
mod macwin;
mod vst3;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
pub use au::AuPlugin;
pub use base64::{decode as decode_state, encode as encode_state};
#[cfg(target_os = "macos")]
pub fn set_plugin_key_handler(handler: impl Fn(u16, bool) + Send + Sync + 'static) {
    key_guard::set_handler(handler);
}
pub use clap::ClapPlugin;
use isai_engine::plugin::PluginRef;
pub use vst3::Vst3Plugin;

/// A plugin format.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Vst3,
    Clap,
    /// Audio Units (macOS).
    Au,
}

impl Format {
    pub fn label(self) -> &'static str {
        match self {
            Format::Vst3 => "VST3",
            Format::Clap => "CLAP",
            Format::Au => "AU",
        }
    }

    /// The name saved in songs.
    pub fn key(self) -> &'static str {
        match self {
            Format::Vst3 => "vst3",
            Format::Clap => "clap",
            Format::Au => "au",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "vst3" => Some(Format::Vst3),
            "clap" => Some(Format::Clap),
            "au" => Some(Format::Au),
            _ => None,
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

/// Finds the VST3 and CLAP plugins in the standard folders. Loading a
/// plugin's code can crash, so the app runs this in a separate process
/// (`isai --scan-plugins`).
pub fn scan() -> Vec<PluginInfo> {
    let mut found = vst3::scan();
    found.extend(clap::scan());
    #[cfg(target_os = "macos")]
    found.extend(au::scan());
    found.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.vendor.cmp(&b.vendor))
    });
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

/// The plugins in one CLAP file (for tests and the probe).
pub fn describe_clap(path: &Path) -> Vec<PluginInfo> {
    clap::describe(path)
}

/// A loaded plugin of any format, owned by the UI thread.
pub enum LoadedPlugin {
    Vst3(Vst3Plugin),
    Clap(ClapPlugin),
    #[cfg(target_os = "macos")]
    Au(AuPlugin),
}

/// Calls the same method on whichever format it is.
macro_rules! each {
    ($self:ident, $p:ident => $e:expr) => {
        match $self {
            LoadedPlugin::Vst3($p) => $e,
            LoadedPlugin::Clap($p) => $e,
            #[cfg(target_os = "macos")]
            LoadedPlugin::Au($p) => $e,
        }
    };
}

impl LoadedPlugin {
    /// The handle the engine processes through.
    pub fn handle(&self) -> PluginRef {
        each!(self, p => p.handle())
    }

    pub fn save_state(&mut self) -> Option<Vec<u8>> {
        each!(self, p => p.save_state())
    }

    pub fn load_state(&mut self, data: &[u8]) -> Result<(), String> {
        each!(self, p => p.load_state(data))
    }

    pub fn open_window(&mut self) -> Result<(), String> {
        each!(self, p => p.open_window())
    }

    pub fn window_open(&self) -> bool {
        each!(self, p => p.window_open())
    }

    /// Keeps its window responsive; returns whether it's open.
    pub fn service_window(&mut self) -> bool {
        each!(self, p => p.service_window())
    }
}

/// Loads a plugin, ready to process at `sample_rate`.
pub fn load(info: &PluginInfo, sample_rate: f64) -> Result<LoadedPlugin, String> {
    match info.format {
        Format::Vst3 => {
            Vst3Plugin::load(&info.path, &info.uid, sample_rate).map(LoadedPlugin::Vst3)
        }
        Format::Clap => {
            ClapPlugin::load(&info.path, &info.uid, sample_rate).map(LoadedPlugin::Clap)
        }
        #[cfg(target_os = "macos")]
        Format::Au => AuPlugin::load(&info.uid, sample_rate).map(LoadedPlugin::Au),
        #[cfg(not(target_os = "macos"))]
        Format::Au => Err("Audio Units only run on macOS".into()),
    }
}

/// Whether a plugin is still installed.
pub fn exists(info: &PluginInfo) -> bool {
    match info.format {
        #[cfg(target_os = "macos")]
        Format::Au => au::exists(&info.uid),
        #[cfg(not(target_os = "macos"))]
        Format::Au => false,
        _ => info.path.exists(),
    }
}
