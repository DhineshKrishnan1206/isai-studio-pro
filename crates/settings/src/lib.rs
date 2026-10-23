//! User preferences: performance profiles, the settings file, and hardware detection.
//!
//! The app ships with every visual feature on, but each heavy one can be turned
//! off. A profile is just a preset that fills in these options; changing any
//! single option afterwards switches the profile to `Custom`.

mod hardware;

use std::fs;
use std::path::PathBuf;

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

pub use hardware::Hardware;

/// The product name. It decides the settings folder, so keep it stable.
pub const APP_NAME: &str = "isai";

/// Bump this when the file format changes, and upgrade older files in [`load`].
pub const SETTINGS_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Smooth,
    Balanced,
    Lightweight,
    Custom,
}

impl Profile {
    pub const PRESETS: [Profile; 3] = [Profile::Smooth, Profile::Balanced, Profile::Lightweight];

    pub fn label(self) -> &'static str {
        match self {
            Profile::Smooth => "Smooth",
            Profile::Balanced => "Balanced",
            Profile::Lightweight => "Lightweight",
            Profile::Custom => "Custom",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Profile::Smooth => {
                "All animations, high frame rate and detailed waveforms. For modern computers."
            }
            Profile::Balanced => "Short animations and medium detail. Good for most laptops.",
            Profile::Lightweight => {
                "No animations, lower frame rate and simple visuals. For older or low-spec computers."
            }
            Profile::Custom => "Your own mix of options.",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum View {
    /// Hides advanced options until the user wants them.
    Simple,
    Pro,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the operating system's light/dark setting.
    #[default]
    System,
    Dark,
    Light,
}

/// The overall look. Studio is a classic console style; Midnight is violet and
/// glowing; Vivid is pure black with bright, saturated track colours.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[default]
    Studio,
    Midnight,
    Vivid,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Animations {
    Full,
    Reduced,
    Off,
}

impl Animations {
    /// Seconds a standard UI transition takes.
    pub fn duration(self) -> f32 {
        match self {
            Animations::Full => 0.2,
            Animations::Reduced => 0.08,
            Animations::Off => 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Detail {
    Low,
    Medium,
    High,
}

/// Gives an enum stable lowercase keys (the same ones used in `settings.toml`),
/// so UI code can pass choices around as plain strings.
macro_rules! keyed {
    ($ty:ty { $($variant:ident => $key:literal),+ $(,)? }) => {
        impl $ty {
            pub fn key(self) -> &'static str {
                match self { $(Self::$variant => $key),+ }
            }
            pub fn from_key(key: &str) -> Option<Self> {
                match key { $($key => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

keyed!(Profile { Smooth => "smooth", Balanced => "balanced", Lightweight => "lightweight", Custom => "custom" });
keyed!(View { Simple => "simple", Pro => "pro" });
keyed!(Theme { System => "system", Dark => "dark", Light => "light" });
keyed!(Style { Studio => "studio", Midnight => "midnight", Vivid => "vivid" });
keyed!(Animations { Full => "full", Reduced => "reduced", Off => "off" });
keyed!(Detail { Low => "low", Medium => "medium", High => "high" });

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Visual {
    pub animations: Animations,
    /// Highest UI redraw rate while something is moving. 0 means match the display.
    pub ui_fps: u32,
    /// Level meter updates per second.
    pub meter_hz: u32,
    pub waveform_detail: Detail,
    /// Shadows and other purely decorative effects.
    pub effects: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Audio {
    /// `None` follows the system default output.
    pub device_id: Option<String>,
    /// Frames per audio callback. Bigger is safer on slow PCs but adds delay.
    pub buffer_size: u32,
    /// Eco mode: instruments play in their lightest quality (exports stay full quality).
    pub eco: bool,
}

/// Recording preferences.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Recording {
    /// `None` follows the system default input.
    pub input_device: Option<String>,
    /// Record two channels (a stereo pair) instead of one.
    pub stereo: bool,
    /// First input channel, counting from 0.
    pub channel: u16,
    /// Extra correction for recording latency, in milliseconds (+ moves takes earlier).
    pub latency_offset_ms: i32,
}

/// Metronome preferences. Whether the click is on is part of each project.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Metronome {
    /// Click loudness in percent of normal (0–150).
    pub volume: u32,
    /// Bars counted in before playback when count-in is on.
    pub count_in_bars: u32,
}

impl Default for Metronome {
    fn default() -> Self {
        Self {
            volume: 80,
            count_in_bars: 1,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Helpers {
    /// Offer a lighter profile when the audio starts crackling.
    pub suggest_lighter_profile: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    /// False while first-launch setup is in progress, so it resumes if the app
    /// closes halfway. Files from before this field existed count as done.
    #[serde(default = "yes")]
    pub setup_done: bool,
    pub profile: Profile,
    pub view: View,
    /// Look settings are not part of any profile: they cost nothing, so
    /// changing them never switches the profile to Custom.
    pub theme: Theme,
    pub style: Style,
    /// "default" (the style's own accent) or a colour name such as "teal".
    pub accent: String,
    /// Interface size in percent; 100 is the design size. Not part of a profile.
    pub ui_scale: u32,
    pub visual: Visual,
    pub audio: Audio,
    pub metronome: Metronome,
    pub recording: Recording,
    pub helpers: Helpers,
    /// Recently opened or saved project files, newest first.
    pub recent: Vec<String>,
}

fn yes() -> bool {
    true
}

impl Default for Visual {
    fn default() -> Self {
        Self::for_profile(Profile::Balanced)
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            device_id: None,
            buffer_size: buffer_for(Profile::Balanced),
            eco: false,
        }
    }
}

impl Default for Helpers {
    fn default() -> Self {
        Self {
            suggest_lighter_profile: true,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            setup_done: false,
            profile: Profile::Balanced,
            view: View::Simple,
            theme: Theme::System,
            style: Style::Studio,
            accent: "default".into(),
            ui_scale: 100,
            visual: Visual::default(),
            audio: Audio::default(),
            metronome: Metronome::default(),
            recording: Recording::default(),
            helpers: Helpers::default(),
            recent: Vec::new(),
        }
    }
}

impl Visual {
    pub fn for_profile(profile: Profile) -> Self {
        match profile {
            Profile::Smooth => Self {
                animations: Animations::Full,
                ui_fps: 0,
                meter_hz: 60,
                waveform_detail: Detail::High,
                effects: true,
            },
            Profile::Balanced | Profile::Custom => Self {
                animations: Animations::Reduced,
                ui_fps: 60,
                meter_hz: 30,
                waveform_detail: Detail::Medium,
                effects: true,
            },
            Profile::Lightweight => Self {
                animations: Animations::Off,
                ui_fps: 30,
                meter_hz: 15,
                waveform_detail: Detail::Low,
                effects: false,
            },
        }
    }
}

pub fn buffer_for(profile: Profile) -> u32 {
    match profile {
        Profile::Smooth => 256,
        Profile::Balanced | Profile::Custom => 512,
        Profile::Lightweight => 1024,
    }
}

impl Settings {
    const RECENT_LIMIT: usize = 10;

    /// Moves `file` to the top of the recent projects list.
    pub fn remember_project(&mut self, file: &str) {
        self.recent.retain(|f| f != file);
        self.recent.insert(0, file.to_owned());
        self.recent.truncate(Self::RECENT_LIMIT);
    }

    /// Fills in the options a profile controls. The audio device, view and
    /// helper choices are kept.
    pub fn apply_profile(&mut self, profile: Profile) {
        self.profile = profile;
        if profile != Profile::Custom {
            self.visual = Visual::for_profile(profile);
            self.audio.buffer_size = buffer_for(profile);
        }
    }

    /// Call after the user edits any single option: if the options no longer
    /// match the chosen preset, the profile becomes `Custom`, so a preset never
    /// silently overwrites the user's own choices.
    pub fn sync_profile(&mut self) {
        if self.profile != Profile::Custom
            && (self.visual != Visual::for_profile(self.profile)
                || self.audio.buffer_size != buffer_for(self.profile))
        {
            self.profile = Profile::Custom;
        }
    }
}

/// `<config dir>/<APP_NAME>/settings.toml`, e.g. `~/.config/daw/settings.toml` on Linux.
///
/// Setting `ISAI_CONFIG_DIR` uses that folder instead, so tests and screenshots
/// never touch the user's real settings.
pub fn path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("ISAI_CONFIG_DIR") {
        return Some(PathBuf::from(dir).join("settings.toml"));
    }
    directories::ProjectDirs::from("", "", APP_NAME).map(|d| d.config_dir().join("settings.toml"))
}

/// `Ok(None)` means there is no settings file yet, i.e. this is the first launch.
///
/// A damaged file is moved aside to `settings.toml.bak` and reported as an
/// error, so the app can run setup again instead of refusing to start.
pub fn load() -> anyhow::Result<Option<Settings>> {
    let path = path().context("could not find the settings folder")?;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).context("could not read settings"),
    };
    match toml::from_str::<Settings>(&text) {
        Ok(mut settings) => {
            settings.version = SETTINGS_VERSION;
            Ok(Some(settings))
        }
        Err(err) => {
            let backup = path.with_extension("toml.bak");
            let _ = fs::rename(&path, &backup);
            Err(anyhow::anyhow!(
                "settings file was damaged ({err}); it was moved to {}",
                backup.display()
            ))
        }
    }
}

/// Writes to a temporary file and renames it over the old one, so a crash
/// mid-save can never leave a half-written settings file.
pub fn save(settings: &Settings) -> anyhow::Result<()> {
    let path = path().context("could not find the settings folder")?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).context("could not create the settings folder")?;
    }
    let text = toml::to_string_pretty(settings)?;
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, text).context("could not write settings")?;
    fs::rename(&tmp, &path).context("could not replace the settings file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_toml() {
        let mut s = Settings::default();
        s.apply_profile(Profile::Lightweight);
        s.audio.device_id = Some("coreaudio:abc".into());
        let text = toml::to_string_pretty(&s).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s);
    }

    #[test]
    fn keys_match_the_settings_file() {
        for p in [
            Profile::Smooth,
            Profile::Balanced,
            Profile::Lightweight,
            Profile::Custom,
        ] {
            assert_eq!(Profile::from_key(p.key()), Some(p));
            assert_eq!(toml::Value::try_from(p).unwrap().as_str(), Some(p.key()));
        }
        assert_eq!(Theme::from_key("nope"), None);
    }

    #[test]
    fn missing_fields_use_defaults() {
        // Files written by older versions lack newer fields and must still load.
        let s: Settings =
            toml::from_str("profile = \"lightweight\"\n[visual]\nui_fps = 30\n").unwrap();
        assert_eq!(s.profile, Profile::Lightweight);
        assert_eq!(s.visual.ui_fps, 30);
        assert_eq!(s.audio, Audio::default());
        assert!(
            s.setup_done,
            "files written before setup tracking count as set up"
        );
        assert_eq!(s.ui_scale, 100, "interface size defaults to 100%");
    }

    #[test]
    fn recent_projects_are_unique_newest_first_and_capped() {
        let mut s = Settings::default();
        for i in 0..12 {
            s.remember_project(&format!("/p{i}.isai"));
        }
        s.remember_project("/p5.isai");
        assert_eq!(s.recent.len(), 10);
        assert_eq!(s.recent[0], "/p5.isai");
        assert_eq!(s.recent.iter().filter(|f| *f == "/p5.isai").count(), 1);
    }

    #[test]
    fn editing_one_option_switches_to_custom() {
        let mut s = Settings::default();
        s.apply_profile(Profile::Smooth);
        s.sync_profile();
        assert_eq!(s.profile, Profile::Smooth);

        s.visual.effects = false;
        s.sync_profile();
        assert_eq!(s.profile, Profile::Custom);
        // A custom setup survives later syncs.
        s.sync_profile();
        assert!(!s.visual.effects);
    }
}
