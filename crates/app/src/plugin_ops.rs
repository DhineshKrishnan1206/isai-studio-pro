//! Third-party plugins in the app: the list of installed ones (scanned in a
//! separate process, so a plugin that crashes while loading can't take Isai
//! down), loading the ones a song uses, their windows, and their state.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::mpsc;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use isai_engine::arrangement::Arrangement;
use isai_plugins::{Format, PluginInfo, Vst3Plugin};
use isai_project::{Chain, PluginSlot, Project};

use crate::controller::qobject;
use crate::project_ops::app_dir;

#[derive(Default)]
pub struct Plugins {
    /// Every plugin installed on this computer, from the last scan.
    pub catalog: Vec<PluginInfo>,
    scan: Option<mpsc::Receiver<Vec<PluginInfo>>>,
    /// Loaded plugins, by insert or instrument id.
    loaded: HashMap<u64, Vst3Plugin>,
    /// Ones that failed to load (with why), so they aren't retried on every edit.
    failed: HashMap<u64, String>,
    /// The sample rate they were loaded at.
    rate: f64,
}

fn cache_path() -> Option<PathBuf> {
    app_dir().map(|d| d.join("plugins.json"))
}

impl Plugins {
    /// Reads the list from the last scan; scans now if there isn't one.
    pub fn start(&mut self) {
        match cache_path().and_then(|p| std::fs::read_to_string(p).ok()) {
            Some(json) => self.catalog = isai_plugins::parse_scan(&json),
            None => self.rescan(),
        }
    }

    pub fn scanning(&self) -> bool {
        self.scan.is_some()
    }

    /// Scans in a child process (`isai --scan-plugins`) on a background thread.
    pub fn rescan(&mut self) {
        if self.scan.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let out = std::env::temp_dir().join(format!("isai-plugin-scan-{}.json", std::process::id()));
        std::thread::spawn(move || {
            let ran = std::env::current_exe()
                .ok()
                .and_then(|exe| std::process::Command::new(exe).arg("--scan-plugins").arg(&out).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().ok());
            let found = ran.and_then(|_| std::fs::read_to_string(&out).ok()).map(|json| isai_plugins::parse_scan(&json)).unwrap_or_default();
            let _ = std::fs::remove_file(&out);
            let _ = tx.send(found);
        });
        self.scan = Some(rx);
    }

    /// True when a scan just finished.
    pub fn poll_scan(&mut self) -> bool {
        let Some(rx) = &self.scan else { return false };
        match rx.try_recv() {
            Ok(found) => {
                if let (Some(path), Ok(json)) = (cache_path(), serde_json::to_string(&found)) {
                    let _ = std::fs::write(path, json);
                }
                self.catalog = found;
                self.scan = None;
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.scan = None;
                true
            }
        }
    }

    /// The list for QML: [{ index, name, vendor, format, instrument }].
    pub fn catalog_json(&self) -> String {
        let list: Vec<serde_json::Value> = self
            .catalog
            .iter()
            .enumerate()
            .map(|(i, p)| serde_json::json!({ "index": i, "name": p.name, "vendor": p.vendor, "format": p.format.label(), "instrument": p.instrument, "category": p.category }))
            .collect();
        serde_json::Value::Array(list).to_string()
    }

    pub fn slot(&self, index: usize) -> Option<PluginSlot> {
        let p = self.catalog.get(index)?;
        Some(PluginSlot {
            format: "vst3".into(),
            path: p.path.display().to_string(),
            uid: p.uid.clone(),
            name: p.name.clone(),
            vendor: p.vendor.clone(),
            state: None,
        })
    }

    /// Loads every plugin the song uses (restoring its saved state) and lets
    /// go of ones it no longer uses. Returns problems to show, if any.
    pub fn ensure(&mut self, project: &Project, rate: f64) -> Vec<String> {
        if (self.rate - rate).abs() > 0.5 {
            // A new sample rate: reload everything at it (keeping their current sound).
            let states: Vec<(u64, Vec<u8>)> = self.loaded.iter().filter_map(|(id, p)| Some((*id, p.save_state()?))).collect();
            self.loaded.clear();
            self.failed.clear();
            self.rate = rate;
            let problems = self.ensure(project, rate);
            for (id, state) in states {
                if let Some(p) = self.loaded.get(&id) {
                    let _ = p.load_state(&state);
                }
            }
            return problems;
        }
        let used: Vec<(u64, &PluginSlot)> = project.plugins();
        let keep: HashSet<u64> = used.iter().map(|(id, _)| *id).collect();
        self.loaded.retain(|id, _| keep.contains(id));
        self.failed.retain(|id, _| keep.contains(id));
        let mut problems = Vec::new();
        for (id, slot) in used {
            if self.loaded.contains_key(&id) || self.failed.contains_key(&id) {
                continue;
            }
            let info = PluginInfo {
                format: Format::Vst3,
                path: PathBuf::from(&slot.path),
                uid: slot.uid.clone(),
                name: slot.name.clone(),
                vendor: slot.vendor.clone(),
                category: String::new(),
                instrument: false,
                has_gui: true,
            };
            if !isai_plugins::exists(&info.path) {
                let why = format!("{} isn't installed on this computer", slot.name);
                problems.push(why.clone());
                self.failed.insert(id, why);
                continue;
            }
            match isai_plugins::load(&info, rate) {
                Ok(plugin) => {
                    if let Some(bytes) = slot.state.as_deref().and_then(isai_plugins::decode_state) {
                        let _ = plugin.load_state(&bytes);
                    }
                    self.loaded.insert(id, plugin);
                }
                Err(e) => {
                    let why = format!("{} couldn't be loaded: {e}", slot.name);
                    problems.push(why.clone());
                    self.failed.insert(id, why);
                }
            }
        }
        problems
    }

    /// Gives each plugin insert and instrument in `arrangement` its loaded plugin.
    pub fn attach(&self, arrangement: &mut Arrangement) {
        let handle = |id: u64| self.loaded.get(&id).map(Vst3Plugin::handle);
        let inserts = arrangement
            .tracks
            .iter_mut()
            .flat_map(|t| t.inserts.iter_mut())
            .chain(arrangement.buses.iter_mut().flat_map(|b| b.inserts.iter_mut()))
            .chain(arrangement.master.iter_mut());
        for i in inserts {
            i.plugin = handle(i.id);
        }
        for inst in arrangement.tracks.iter_mut().filter_map(|t| t.instrument.as_mut()) {
            inst.plugin = handle(inst.id);
            inst.bpm = f64::from(arrangement.bpm);
        }
    }

    /// Copies every loaded plugin's state into the song, ready to save.
    pub fn store_states(&self, project: &mut Project) {
        for (id, plugin) in &self.loaded {
            if let Some(state) = plugin.save_state() {
                project.set_plugin_state(*id, isai_plugins::encode_state(&state));
            }
        }
    }

    pub fn open_window(&mut self, id: u64) -> Result<(), String> {
        match self.loaded.get_mut(&id) {
            Some(p) => p.open_window(),
            None => Err(self.failed.get(&id).cloned().unwrap_or_else(|| "The plugin isn't loaded".into())),
        }
    }

    /// Keeps open windows responsive; returns how many are open.
    pub fn service_windows(&mut self) -> usize {
        self.loaded.values_mut().filter(|p| p.window_open()).map(|p| p.service_window()).filter(|&open| open).count()
    }
}

impl qobject::AppController {
    fn plugins(self: Pin<&mut Self>) -> &mut Plugins {
        &mut self.rust_mut().get_mut().state.plugins
    }

    /// Publishes the plugin list and whether a scan is running.
    pub(crate) fn publish_plugins(mut self: Pin<&mut Self>) {
        let (json, scanning) = {
            let p = self.as_mut().plugins();
            (p.catalog_json(), p.scanning())
        };
        self.as_mut().set_plugins_json(QString::from(json.as_str()));
        self.set_plugin_scanning(scanning);
    }

    pub fn rescan_plugins(mut self: Pin<&mut Self>) {
        self.as_mut().plugins().rescan();
        self.publish_plugins();
    }

    /// Checks on a running scan and keeps plugin windows responsive.
    pub fn poll_plugins(mut self: Pin<&mut Self>) {
        let done = self.as_mut().plugins().poll_scan();
        let open = self.as_mut().plugins().service_windows() as i32;
        if open != *self.plugin_windows() {
            self.as_mut().set_plugin_windows(open);
        }
        if done {
            self.publish_plugins();
        }
    }

    /// Adds catalog plugin `index` to a chain ("track", "bus" or "master"), and opens its window.
    pub fn add_plugin_effect(mut self: Pin<&mut Self>, chain_type: &QString, chain_id: i32, index: i32) -> i32 {
        let Some(slot) = self.as_mut().plugins().slot(index.max(0) as usize) else { return -1 };
        let chain = match chain_type.to_string().as_str() {
            "bus" => Chain::Bus(chain_id as u64),
            "master" => Chain::Master,
            _ => Chain::Track(chain_id as u64),
        };
        let mut new_id = None;
        let label = format!("Add {}", slot.name);
        self.as_mut().edit(&label, true, |p| {
            new_id = p.add_plugin_insert(chain, slot);
            new_id.is_some()
        });
        let Some(id) = new_id else { return -1 };
        self.open_plugin(id as i32);
        id as i32
    }

    /// Adds a track playing catalog plugin `index`, selects it and opens the plugin's window.
    pub fn add_plugin_track(mut self: Pin<&mut Self>, index: i32) -> i32 {
        let Some(slot) = self.as_mut().plugins().slot(index.max(0) as usize) else { return -1 };
        let mut track = 0;
        let label = format!("Add {}", slot.name);
        self.as_mut().edit(&label, true, |p| {
            track = p.add_plugin_track(slot);
            true
        });
        self.as_mut().select_track(track as i32);
        let inst = self
            .as_mut()
            .st()
            .session
            .as_ref()
            .and_then(|s| s.project.tracks.iter().find(|t| t.id == track).and_then(|t| t.instrument.as_ref()).map(|i| i.id));
        if let Some(id) = inst {
            self.as_mut().open_plugin(id as i32);
        }
        track as i32
    }

    /// Opens a plugin's own window, by its insert or instrument id.
    pub fn open_plugin(mut self: Pin<&mut Self>, id: i32) {
        let result = self.as_mut().plugins().open_window(id as u64);
        match result {
            Ok(()) => {
                let open = self.as_mut().plugins().service_windows() as i32;
                self.as_mut().set_plugin_windows(open.max(1));
            }
            Err(e) => self.show_problem(&e),
        }
    }
}
