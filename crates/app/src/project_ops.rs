//! The controller's project side: transport, tracks, regions, undo, and
//! saving/opening. The editing rules themselves live in `isai-project` and
//! `session.rs`; this file connects them to QML and the engine.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cxx_qt_lib::{QString, QUrl};
use isai_engine::Meter;
use isai_engine::instruments::LiveNote;
use isai_project::{Autosave, Chain, EffectKind, InstrumentKind, MAX_PARAMS, Project, ProjectPaths, Snap};

use crate::controller::{qlist, qobject::AppController, qs};
use crate::format::format_time;
use crate::session::Session;

/// QML passes ids as `int`; tracks and regions never get anywhere near the limit.
pub(crate) fn id(v: i32) -> u64 {
    v.max(0) as u64
}

pub(crate) fn url_path(url: &QUrl) -> Option<PathBuf> {
    url.to_local_file().map(|p| PathBuf::from(p.to_string()))
}

fn file_stem(path: &Path) -> String {
    path.file_stem().map_or_else(|| "Audio".into(), |s| s.to_string_lossy().into_owned())
}

/// The app's own folder (next to settings.toml), used for autosave.
pub(crate) fn app_dir() -> Option<PathBuf> {
    isai_settings::path().and_then(|p| p.parent().map(Path::to_path_buf))
}

fn ago(secs_since_epoch: u64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let mins = now.saturating_sub(secs_since_epoch) / 60;
    match mins {
        0 => "just now".into(),
        1 => "a minute ago".into(),
        m if m < 60 => format!("{m} minutes ago"),
        m if m < 120 => "an hour ago".into(),
        m if m < 60 * 24 => format!("{} hours ago", m / 60),
        m => format!("{} days ago", m / (60 * 24)),
    }
}

impl AppController {
    // ------------------------------------------------------------- helpers

    /// Applies an undoable edit to the open project. With `publish`, QML gets
    /// the new track list; live drags pass false and publish on release.
    pub(crate) fn edit(mut self: Pin<&mut Self>, label: &str, publish: bool, f: impl FnOnce(&mut Project) -> bool) -> bool {
        let Some(session) = self.as_mut().st().session.as_mut() else { return false };
        if !session.edit(label, f) {
            // The last step of a drag often changes nothing (the drag already
            // set the value), but the UI still needs the final state, or it
            // falls back to what was last published.
            if publish {
                self.publish_project();
            }
            return false;
        }
        self.as_mut().push_project_to_engine();
        self.as_mut().push_arrangement();
        if publish {
            self.publish_project();
        } else {
            self.publish_file_state();
        }
        true
    }

    /// Sends tempo, time signature, metronome and cycle to the engine.
    pub(crate) fn push_project_to_engine(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let click = st.settings.metronome.volume as f32 / 100.0;
        let (Some(engine), Some(s)) = (&mut st.audio.engine, &st.session) else { return };
        let p = &s.project;
        let rate = engine.info().sample_rate;
        let meter = p.meter();
        engine.set_meter(meter);
        engine.set_metronome(p.metronome, click);
        engine.set_cycle(p.cycle, meter.beats_to_samples(p.cycle_start, rate), meter.beats_to_samples(p.cycle_end, rate));
    }

    /// Sends the tracks and regions to the engine.
    /// Edits that may arm/disarm tracks or change monitoring call this.
    pub(crate) fn after_arming(self: Pin<&mut Self>) {
        self.sync_input();
    }

    pub(crate) fn push_arrangement(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        // Regions following the tempo need their audio stretched to fit (made in the background).
        if let Some(s) = &st.session {
            for (path, ratio) in s.stretches() {
                st.pool.request_stretch(&path, ratio);
            }
        }
        let (Some(_), Some(s)) = (&st.audio.engine, &st.session) else { return };
        let arrangement = s.arrangement(&st.pool, st.settings.audio.eco);
        // Effects first (commands arrive in order), so none is missing when the arrangement lands.
        let wanted: Vec<_> = arrangement.inserts().map(|i| (i.id, i.kind)).collect();
        st.audio.sync_effects(&wanted);
        let instruments: Vec<_> = arrangement.instruments().map(|i| (i.id, i.kind)).collect();
        st.audio.sync_instruments(&instruments);
        if let Some(engine) = &mut st.audio.engine {
            engine.set_arrangement(std::sync::Arc::new(arrangement));
        }
        // Track order may have changed: route live notes again.
        st.live_track = None;
        self.route_live();
    }

    /// Everything QML shows about the project.
    pub(crate) fn publish_project(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(s) = &st.session else { return };
        let p = s.project.clone();
        let json = s.tracks_json(&st.pool);
        let buses = s.buses_json();
        let master = s.master_inserts_json();
        let markers = s.markers_json();
        self.as_mut().set_project_name(qs(&p.name));
        self.as_mut().set_tempo(p.tempo);
        self.as_mut().set_time_sig_num(p.numerator as i32);
        self.as_mut().set_time_sig_den(p.denominator as i32);
        self.as_mut().set_key_name(qs(&p.key));
        self.as_mut().set_metronome(p.metronome);
        self.as_mut().set_count_in(p.count_in);
        self.as_mut().set_cycle(p.cycle);
        self.as_mut().set_cycle_start(p.cycle_start);
        self.as_mut().set_cycle_end(p.cycle_end);
        self.as_mut().set_snap(p.snap);
        self.as_mut().set_snap_value(qs(p.snap_value.key()));
        self.as_mut().set_length_beats(p.length_beats());
        self.as_mut().set_track_count(p.tracks.len() as i32);
        self.as_mut().set_tracks_json(qs(&json));
        self.as_mut().set_buses_json(qs(&buses));
        self.as_mut().set_master_inserts_json(qs(&master));
        if markers != self.markers_json().to_string() {
            self.as_mut().set_markers_json(qs(&markers));
        }
        self.as_mut().publish_file_state();
        self.prune_selection();
    }

    /// Unsaved-changes dot, file path and the Undo/Redo menu labels.
    fn publish_file_state(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(s) = &st.session else { return };
        let dirty = s.dirty;
        let path = s.paths.as_ref().map(|p| p.file.display().to_string()).unwrap_or_default();
        let undo = s.history.undo_label().unwrap_or("").to_owned();
        let redo = s.history.redo_label().unwrap_or("").to_owned();
        let history = serde_json::json!({
            "undo": s.history.undo_labels().collect::<Vec<_>>(),
            "redo": s.history.redo_labels().collect::<Vec<_>>(),
        })
        .to_string();
        if dirty != *self.dirty() {
            self.as_mut().set_dirty(dirty);
        }
        self.as_mut().set_project_path(qs(&path));
        self.as_mut().set_undo_label(qs(&undo));
        if history != self.history_json().to_string() {
            self.as_mut().set_history_json(qs(&history));
        }
        self.set_redo_label(qs(&redo));
    }

    pub(crate) fn publish_recent(mut self: Pin<&mut Self>) {
        let recent: Vec<String> = self
            .as_mut()
            .st()
            .settings
            .recent
            .iter()
            .filter(|f| Path::new(f).exists())
            .cloned()
            .collect();
        let names = recent.iter().map(|f| file_stem(Path::new(f)));
        self.as_mut().set_recent_names(qlist(names));
        self.set_recent_paths(qlist(recent.iter().cloned()));
    }

    /// Opens `session` as the current project: engine, audio and QML.
    fn open_session(mut self: Pin<&mut Self>, session: Session) {
        let st = self.as_mut().st();
        if let Some(engine) = &mut st.audio.engine {
            engine.stop();
            engine.seek(0);
        }
        let files = session.audio_paths();
        st.pool.retain(&files);
        for f in &files {
            st.pool.request(f);
        }
        st.pending_imports.clear();
        st.session = Some(session);
        self.as_mut().set_selected_track(-1);
        self.as_mut().set_selection(Vec::new());
        self.as_mut().set_status(QString::default());
        self.as_mut().push_project_to_engine();
        self.as_mut().push_arrangement();
        self.as_mut().publish_project();
        self.as_mut().seek_beats(0.0);
        self.as_mut().set_project_open(true);
        self.as_mut().sync_input();
        self.update_busy();
    }

    // ------------------------------------------------------------ transport

    pub fn toggle_play(mut self: Pin<&mut Self>) {
        if *self.recording() {
            self.stop_recording();
            return;
        }
        let st = self.as_mut().st();
        let count_in = st.session.as_ref().is_some_and(|s| s.project.count_in);
        let bars = st.settings.metronome.count_in_bars;
        let has_session = st.session.is_some();
        let Some(engine) = &mut st.audio.engine else { return };
        if !has_session {
            return;
        }
        let playing = engine.is_playing();
        if playing {
            engine.stop();
        } else {
            engine.play(if count_in { bars } else { 0 });
        }
        // Reflect the change now; the engine confirms it on the next tick.
        self.as_mut().set_playing(!playing);
        self.update_busy();
    }

    pub fn rewind(self: Pin<&mut Self>) {
        self.seek_beats(0.0);
    }

    pub fn stop(self: Pin<&mut Self>) {
        if *self.playing() || *self.recording() {
            self.toggle_play();
        } else {
            self.rewind();
        }
    }

    pub fn seek_beats(mut self: Pin<&mut Self>, beats: f64) {
        let beats = beats.max(0.0);
        let st = self.as_mut().st();
        let Some(meter) = st.session.as_ref().map(|s| s.project.meter()) else { return };
        if let Some(engine) = &mut st.audio.engine {
            let rate = engine.info().sample_rate;
            engine.seek(meter.beats_to_samples(beats, rate));
        }
        let secs = meter.beats_to_secs(beats);
        self.as_mut().set_position_beats(beats);
        self.as_mut().set_bar_text(qs(&meter.bar_beat(beats).to_string()));
        self.as_mut().set_position(secs);
        self.set_position_text(qs(&format_time(secs)));
    }

    pub fn snap_beats(&self, beats: f64) -> f64 {
        use cxx_qt::CxxQtType;
        self.rust().state.session.as_ref().map_or(beats, |s| s.project.snap(beats))
    }

    pub fn set_volume_level(mut self: Pin<&mut Self>, volume: f64) {
        let volume = volume.clamp(0.0, 1.5);
        if let Some(engine) = &mut self.as_mut().st().audio.engine {
            engine.set_volume(volume as f32);
        }
        self.set_volume(volume);
    }

    pub fn clear_status(self: Pin<&mut Self>) {
        self.set_status(QString::default());
    }

    pub fn clear_clip(self: Pin<&mut Self>) {
        self.set_master_clip(false);
    }

    /// Shows a message about something that went wrong.
    pub(crate) fn show_problem(mut self: Pin<&mut Self>, message: &str) {
        self.as_mut().set_status_ok(false);
        self.set_status(qs(message));
    }

    /// Shows good news, such as "Saved".
    pub(crate) fn show_info(mut self: Pin<&mut Self>, message: &str) {
        self.as_mut().set_status_ok(true);
        self.set_status(qs(message));
    }

    // -------------------------------------------------------------- project

    pub fn new_project(self: Pin<&mut Self>, name: &QString, tempo: f64, num: i32, den: i32, key: &QString) {
        let meter = Meter {
            bpm: tempo.clamp(Meter::MIN_BPM, Meter::MAX_BPM),
            numerator: num.clamp(1, 32) as u32,
            denominator: if matches!(den, 2 | 4 | 8 | 16) { den as u32 } else { 4 },
        };
        let project = Project::new(&name.to_string(), meter, &key.to_string());
        self.open_session(Session::new(project, None));
    }

    pub fn close_project(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        if let Some(engine) = &mut st.audio.engine {
            engine.stop();
        }
        st.session = None;
        st.pending_imports.clear();
        self.as_mut().push_arrangement();
        self.as_mut().set_playing(false);
        self.set_project_open(false);
    }

    pub fn set_project_tempo(mut self: Pin<&mut Self>, bpm: f64) {
        let beats = *self.position_beats();
        // Keep one decimal place, like a hardware tempo display.
        let bpm = (bpm.clamp(Meter::MIN_BPM, Meter::MAX_BPM) * 10.0).round() / 10.0;
        self.as_mut().edit("Tempo", true, |p| {
            p.tempo = bpm;
            true
        });
        // The playhead stays on the same beat when the tempo changes.
        self.seek_beats(beats);
    }

    pub fn tap_tempo(mut self: Pin<&mut Self>) {
        if let Some(bpm) = self.as_mut().st().tap.tap(Instant::now()) {
            self.set_project_tempo(bpm);
        }
    }

    pub fn set_time_signature(self: Pin<&mut Self>, num: i32, den: i32) {
        self.edit("Time Signature", true, |p| {
            p.numerator = num.clamp(1, 32) as u32;
            p.denominator = if matches!(den, 2 | 4 | 8 | 16) { den as u32 } else { 4 };
            true
        });
    }

    pub fn set_project_key(self: Pin<&mut Self>, key: &QString) {
        let key = key.to_string();
        self.edit("Key", true, |p| {
            p.key = key;
            true
        });
    }

    pub fn toggle_metronome(self: Pin<&mut Self>) {
        self.edit("Metronome", true, |p| {
            p.metronome = !p.metronome;
            true
        });
    }

    pub fn toggle_count_in(self: Pin<&mut Self>) {
        self.edit("Count-in", true, |p| {
            p.count_in = !p.count_in;
            true
        });
    }

    pub fn toggle_cycle(self: Pin<&mut Self>) {
        self.edit("Cycle", true, |p| {
            p.cycle = !p.cycle;
            true
        });
    }

    pub fn cycle_selection(mut self: Pin<&mut Self>) {
        let mut ids = self.as_mut().selection();
        // Nothing selected: everything on the selected track.
        if ids.is_empty() {
            let track = *self.selected_track();
            ids = self.as_mut().st().session.as_ref().and_then(|s| s.project.track(id(track))).map(|t| t.regions.iter().map(|r| r.id).collect()).unwrap_or_default();
        }
        if ids.is_empty() {
            self.show_problem("Select some regions first, then ⌘U repeats them");
            return;
        }
        let at = *self.position_beats();
        let mut start = 0.0;
        let mut end = 0.0;
        if self.as_mut().edit("Cycle Regions", true, |p| {
            let done = p.cycle_regions(&ids);
            (start, end) = (p.cycle_start, p.cycle_end);
            done
        }) && (at < start || at >= end)
        {
            // Outside the new range: start playing from its beginning.
            self.seek_beats(start);
        }
    }

    pub fn set_cycle_range(self: Pin<&mut Self>, start: f64, end: f64) {
        self.edit("Cycle Range", true, |p| {
            p.set_cycle(start, end);
            p.cycle = true;
            true
        });
    }

    pub fn toggle_snap(self: Pin<&mut Self>) {
        self.edit("Snap", true, |p| {
            p.snap = !p.snap;
            true
        });
    }

    pub fn set_snap_grid(self: Pin<&mut Self>, value: &QString) {
        let Some(snap) = Snap::from_key(&value.to_string()) else { return };
        self.edit("Snap Grid", true, |p| {
            p.snap_value = snap;
            p.snap = true;
            true
        });
    }

    // --------------------------------------------------------------- tracks

    pub fn import_url(self: Pin<&mut Self>, url: &QUrl) {
        if let Some(path) = url_path(url) {
            if path.extension().is_some_and(|e| e == isai_project::EXTENSION) {
                // Dropping a project file opens it instead (QML asks about unsaved work first).
                self.open_path(&path);
            } else {
                self.import_path(path);
            }
        }
    }

    /// Starts decoding `path`; when ready it lands on a new track at the playhead.
    pub(crate) fn import_path(mut self: Pin<&mut Self>, path: PathBuf) {
        let st = self.as_mut().st();
        st.pool.request(&path);
        st.pending_imports.push((path, None));
        self.as_mut().set_loading(true);
        self.as_mut().set_status(QString::default());
        self.update_busy();
    }

    pub fn add_track(mut self: Pin<&mut Self>) {
        let mut new_id = 0;
        self.as_mut().edit("New Track", true, |p| {
            new_id = p.add_track(None);
            true
        });
        self.set_selected_track(new_id as i32);
    }

    pub fn remove_track(mut self: Pin<&mut Self>, track: i32) {
        if self.as_mut().edit("Delete Track", true, |p| p.remove_track(id(track))) {
            self.set_selected_track(-1);
        }
    }

    pub fn rename_track(self: Pin<&mut Self>, track: i32, name: &QString) {
        let name = name.to_string().trim().to_owned();
        if name.is_empty() {
            return;
        }
        self.edit("Rename Track", true, |p| p.track_mut(id(track)).map(|t| t.name = name).is_some());
    }

    pub fn set_track_color(self: Pin<&mut Self>, track: i32, color: &QString) {
        let color = color.to_string();
        self.edit("Track Colour", true, |p| p.track_mut(id(track)).map(|t| t.color = color).is_some());
    }

    pub fn toggle_mute(self: Pin<&mut Self>, track: i32) {
        self.edit("Mute", true, |p| p.track_mut(id(track)).map(|t| t.mute = !t.mute).is_some());
    }

    pub fn toggle_solo(self: Pin<&mut Self>, track: i32) {
        self.edit("Solo", true, |p| p.track_mut(id(track)).map(|t| t.solo = !t.solo).is_some());
    }

    pub fn set_track_volume(self: Pin<&mut Self>, track: i32, db: f64, commit: bool) {
        let db = (db as f32).clamp(isai_project::Track::MIN_DB, isai_project::Track::MAX_DB);
        self.edit("Volume", commit, |p| p.track_mut(id(track)).map(|t| t.volume_db = db).is_some());
    }

    pub fn set_track_pan(self: Pin<&mut Self>, track: i32, pan: f64, commit: bool) {
        let pan = (pan as f32).clamp(-1.0, 1.0);
        self.edit("Pan", commit, |p| p.track_mut(id(track)).map(|t| t.pan = pan).is_some());
    }

    // ---------------------------------------------------------------- buses

    pub fn add_bus(self: Pin<&mut Self>) {
        self.edit("New Bus", true, |p| {
            p.add_bus(None);
            true
        });
    }

    pub fn remove_bus(self: Pin<&mut Self>, bus: i32) {
        self.edit("Delete Bus", true, |p| p.remove_bus(id(bus)));
    }

    pub fn rename_bus(self: Pin<&mut Self>, bus: i32, name: &QString) {
        let name = name.to_string().trim().to_owned();
        if !name.is_empty() {
            self.edit("Rename Bus", true, |p| p.bus_mut(id(bus)).map(|b| b.name = name).is_some());
        }
    }

    pub fn set_bus_volume(self: Pin<&mut Self>, bus: i32, db: f64, commit: bool) {
        let db = (db as f32).clamp(isai_project::Track::MIN_DB, isai_project::Track::MAX_DB);
        self.edit("Bus Volume", commit, |p| p.bus_mut(id(bus)).map(|b| b.volume_db = db).is_some());
    }

    pub fn set_bus_pan(self: Pin<&mut Self>, bus: i32, pan: f64, commit: bool) {
        let pan = (pan as f32).clamp(-1.0, 1.0);
        self.edit("Bus Pan", commit, |p| p.bus_mut(id(bus)).map(|b| b.pan = pan).is_some());
    }

    pub fn toggle_bus_mute(self: Pin<&mut Self>, bus: i32) {
        self.edit("Mute Bus", true, |p| p.bus_mut(id(bus)).map(|b| b.mute = !b.mute).is_some());
    }

    pub fn set_send(self: Pin<&mut Self>, track: i32, bus: i32, db: f64, commit: bool) {
        self.edit("Send Level", commit, |p| p.set_send(id(track), id(bus), db as f32));
    }

    // -------------------------------------------------------------- effects

    pub fn add_insert(mut self: Pin<&mut Self>, chain: &QString, owner: i32, kind: &QString) -> i32 {
        let chain = match chain.to_string().as_str() {
            "track" => Chain::Track(id(owner)),
            "bus" => Chain::Bus(id(owner)),
            "master" => Chain::Master,
            _ => return -1,
        };
        let Some(kind) = EffectKind::from_key(&kind.to_string()) else { return -1 };
        let mut added = None;
        self.as_mut().edit(&format!("Add {}", kind.label()), true, |p| {
            added = p.add_insert(chain, kind);
            added.is_some()
        });
        added.map_or(-1, |i| i as i32)
    }

    pub fn effect_meters(mut self: Pin<&mut Self>, insert: i32) -> QString {
        let Some(m) = self.as_mut().st().audio.meters(id(insert)) else { return QString::default() };
        let (levels, gr) = m.take();
        let scale = |v: f32| crate::controller::meter_scale(v);
        qs(&serde_json::json!({
            "in": [scale(levels[0]), scale(levels[1])],
            "out": [scale(levels[2]), scale(levels[3])],
            "gr": gr,
        })
        .to_string())
    }

    pub fn effect_spectrum(mut self: Pin<&mut Self>, insert: i32, bands: i32) -> QString {
        let st = self.as_mut().st();
        let rate = st.audio.engine.as_ref().map_or(48_000, |e| e.info().sample_rate) as f32;
        let Some(m) = st.audio.meters(id(insert)) else { return QString::default() };
        let levels = crate::spectrum::bands(&m.scope(), rate, bands.clamp(8, 256) as usize);
        qs(&serde_json::to_string(&levels).unwrap_or_default())
    }

    pub fn eq_curve(&self, values: &QString, points: i32) -> QString {
        let params = eq_params(values);
        let curve: Vec<f32> = curve_freqs(points).into_iter().map(|f| isai_engine::effects::Eq::response_db(&params, f, 48_000.0)).collect();
        qs(&serde_json::to_string(&curve).unwrap_or_default())
    }

    pub fn eq_curves(&self, values: &QString, points: i32) -> QString {
        let params = eq_params(values);
        let freqs = curve_freqs(points);
        let total: Vec<f32> = freqs.iter().map(|&f| isai_engine::effects::Eq::response_db(&params, f, 48_000.0)).collect();
        let bands = isai_engine::effects::Eq::band_curves(&params, &freqs, 48_000.0);
        qs(&serde_json::json!({ "total": total, "bands": bands }).to_string())
    }

    pub fn effect_spectra(mut self: Pin<&mut Self>, insert: i32, bands: i32) -> QString {
        let st = self.as_mut().st();
        let rate = st.audio.engine.as_ref().map_or(48_000, |e| e.info().sample_rate) as f32;
        let Some(m) = st.audio.meters(id(insert)) else { return QString::default() };
        let n = bands.clamp(8, 256) as usize;
        let pre = crate::spectrum::bands(&m.pre_scope(), rate, n);
        let post = crate::spectrum::bands(&m.scope(), rate, n);
        qs(&serde_json::json!({ "pre": pre, "post": post }).to_string())
    }

    // ------------------------------------------------------ instruments, MIDI

    pub fn add_instrument_track(mut self: Pin<&mut Self>, kind: &QString) {
        let Some(kind) = InstrumentKind::from_key(&kind.to_string()) else { return };
        let mut new_id = 0;
        self.as_mut().edit("New Instrument Track", true, |p| {
            new_id = p.add_instrument_track(kind);
            true
        });
        self.select_track(new_id as i32);
    }

    pub fn add_midi_region(mut self: Pin<&mut Self>, track: i32, start: f64, length: f64) -> i32 {
        let mut added = None;
        self.as_mut().edit("New MIDI Region", true, |p| {
            added = p.add_midi_region(id(track), start, length);
            added.is_some()
        });
        match added {
            Some(r) => {
                self.set_selection(vec![r]);
                r as i32
            }
            None => -1,
        }
    }

    pub fn add_note(self: Pin<&mut Self>, region: i32, start: f64, length: f64, key: i32, velocity: i32) -> i32 {
        let mut index = None;
        self.edit("Add Note", true, |p| {
            index = p.add_note(id(region), start, length, key.clamp(0, 127) as u8, velocity.clamp(1, 127) as u8);
            index.is_some()
        });
        index.map_or(-1, |i| i as i32)
    }

    pub fn update_note(self: Pin<&mut Self>, region: i32, index: i32, start: f64, length: f64, key: i32, velocity: i32) -> i32 {
        let mut new_index = None;
        self.edit("Edit Note", true, |p| {
            new_index = p.update_note(id(region), index.max(0) as usize, start, length, key.clamp(0, 127) as u8, velocity.clamp(1, 127) as u8);
            new_index.is_some()
        });
        new_index.map_or(-1, |i| i as i32)
    }

    pub fn remove_note(self: Pin<&mut Self>, region: i32, index: i32) {
        self.edit("Delete Note", true, |p| p.remove_note(id(region), index.max(0) as usize));
    }

    pub fn transform_notes(self: Pin<&mut Self>, region: i32, indices: &QString, d_start: f64, d_key: i32, d_length: f64) -> QString {
        let indices = index_list(indices);
        let mut moved = None;
        let name = if d_length != 0.0 { "Resize Notes" } else { "Move Notes" };
        self.edit(name, true, |p| {
            moved = p.transform_notes(id(region), &indices, d_start, d_key, d_length);
            moved.is_some()
        });
        qs(&moved.unwrap_or_default().iter().map(usize::to_string).collect::<Vec<_>>().join(","))
    }

    pub fn remove_notes(self: Pin<&mut Self>, region: i32, indices: &QString) {
        let indices = index_list(indices);
        self.edit("Delete Notes", true, |p| p.remove_notes(id(region), &indices));
    }

    pub fn quantize_notes(self: Pin<&mut Self>, region: i32, indices: &QString, grid: f64, strength: f64, ends: bool, swing: f64) {
        let indices = index_list(indices);
        let label = if swing > 0.0 { "Swing" } else { "Quantize" };
        self.edit(label, true, |p| p.quantize_notes(id(region), &indices, grid, strength, ends, swing));
    }

    pub fn add_notes(self: Pin<&mut Self>, region: i32, notes: &QString) -> QString {
        let parsed: Vec<[f64; 4]> = serde_json::from_str(&notes.to_string()).unwrap_or_default();
        let notes: Vec<isai_project::Note> = parsed
            .iter()
            .map(|n| isai_project::Note { start: n[0], length: n[1], key: n[2].clamp(0.0, 127.0) as u8, velocity: n[3].clamp(1.0, 127.0) as u8 })
            .collect();
        let mut added = None;
        self.edit(if notes.len() > 1 { "Add Chord" } else { "Add Note" }, true, |p| {
            added = p.add_notes(id(region), &notes);
            added.is_some()
        });
        index_text(added)
    }

    pub fn strum_notes(self: Pin<&mut Self>, region: i32, indices: &QString, spread: f64, down: bool) -> QString {
        let indices = index_list(indices);
        let mut out = None;
        self.edit("Strum", true, |p| {
            out = p.strum_notes(id(region), &indices, spread, down);
            out.is_some()
        });
        index_text(out)
    }

    pub fn arpeggiate_notes(self: Pin<&mut Self>, region: i32, indices: &QString, step: f64, pattern: &QString) -> QString {
        let indices = index_list(indices);
        let pattern = pattern.to_string();
        let mut out = None;
        self.edit("Arpeggiate", true, |p| {
            out = p.arpeggiate_notes(id(region), &indices, step, &pattern);
            out.is_some()
        });
        index_text(out)
    }

    // -------------------------------------------------------------- markers

    pub fn add_marker(self: Pin<&mut Self>, beats: f64) -> i32 {
        let mut added = 0;
        self.edit("Add Marker", true, |p| {
            added = p.add_marker(beats, None);
            true
        });
        added as i32
    }

    pub fn move_marker(self: Pin<&mut Self>, marker: i32, beats: f64) {
        self.edit("Move Marker", true, |p| p.move_marker(id(marker), beats));
    }

    pub fn rename_marker(self: Pin<&mut Self>, marker: i32, name: &QString) {
        let name = name.to_string();
        self.edit("Rename Marker", true, |p| p.rename_marker(id(marker), &name));
    }

    pub fn remove_marker(self: Pin<&mut Self>, marker: i32) {
        self.edit("Delete Marker", true, |p| p.remove_marker(id(marker)));
    }

    pub fn jump_marker(mut self: Pin<&mut Self>, forward: bool) {
        let at = *self.position_beats();
        let Some(p) = self.as_mut().st().session.as_ref().map(|s| &s.project) else { return };
        // Going back from just after a marker goes to the one before it, as a
        // second press would feel stuck otherwise.
        let target = if forward { p.marker_after(at) } else { Some(p.marker_before(at - 0.25)) };
        if let Some(beats) = target {
            self.seek_beats(beats);
        }
    }

    pub fn cycle_marker(self: Pin<&mut Self>, marker: i32) {
        self.edit("Cycle Section", true, |p| {
            let Some((start, end)) = p.marker_section(id(marker)) else { return false };
            p.cycle_start = start;
            p.cycle_end = end;
            p.cycle = true;
            true
        });
    }

    // --------------------------------------------------------- region tools

    pub fn set_region_gain(self: Pin<&mut Self>, region: i32, db: f64, commit: bool) {
        self.edit("Region Gain", commit, |p| p.set_region_gain(id(region), db as f32));
    }

    pub fn reverse_selected(mut self: Pin<&mut Self>) {
        let ids = self.as_mut().selection();
        self.edit("Reverse", true, |p| {
            let audio: Vec<u64> = ids.iter().copied().filter(|&r| p.find_region(r).is_some_and(|(_, r)| r.midi.is_none())).collect();
            // Reverse them all, unless they all already are: then play them forwards again.
            let on = !audio.iter().all(|&r| p.find_region(r).is_some_and(|(_, r)| r.reverse));
            audio.iter().fold(false, |any, &r| p.set_region_reverse(r, on) || any)
        });
    }

    pub fn normalize_selected(mut self: Pin<&mut Self>) {
        let ids = self.as_mut().selection();
        let st = self.as_mut().st();
        let Some(session) = &st.session else { return };
        // The loudest sample in each region's slice of its audio.
        let gains: Vec<(u64, f32)> = ids
            .iter()
            .filter_map(|&r| {
                let (_, region) = session.project.find_region(r)?;
                if region.midi.is_some() {
                    return None;
                }
                let clip = st.pool.ready(&session.resolve(&region.file))?;
                let rate = f64::from(clip.sample_rate);
                let from = ((region.offset * rate) as usize).min(clip.left.len());
                let to = (((region.offset + region.length) * rate) as usize).clamp(from, clip.left.len());
                let peak = clip.left[from..to].iter().chain(&clip.right[from..to]).fold(0.0f32, |m, s| m.max(s.abs()));
                (peak > 1e-5).then(|| (r, -1.0 - 20.0 * peak.log10()))
            })
            .collect();
        if gains.is_empty() {
            self.show_problem("Select audio regions with sound in them to normalize.");
            return;
        }
        self.edit("Normalize", true, |p| gains.iter().fold(false, |any, &(r, db)| p.set_region_gain(r, db) || any));
    }

    pub fn detect_region_tempo(mut self: Pin<&mut Self>, region: i32) -> f64 {
        let st = self.as_mut().st();
        let Some(session) = &st.session else { return 0.0 };
        let Some((_, r)) = session.project.find_region(id(region)).filter(|(_, r)| r.midi.is_none()) else { return 0.0 };
        let Some(clip) = st.pool.ready(&session.resolve(&r.file)) else { return 0.0 };
        let rate = f64::from(clip.sample_rate);
        let from = ((r.offset * rate) as usize).min(clip.left.len());
        let to = (((r.offset + r.length) * rate) as usize).clamp(from, clip.left.len());
        let mono: Vec<f32> = clip.left[from..to].iter().zip(&clip.right[from..to]).map(|(a, b)| (a + b) * 0.5).collect();
        crate::slicer::tempo(&mono, clip.sample_rate).unwrap_or(0.0)
    }

    pub fn set_region_tempo(self: Pin<&mut Self>, region: i32, bpm: f64) {
        let label = if bpm > 0.0 { "Follow Tempo" } else { "Stop Following Tempo" };
        self.edit(label, true, |p| p.set_region_tempo(id(region), (bpm > 0.0).then_some(bpm)));
    }

    pub fn set_region_loop(self: Pin<&mut Self>, region: i32, end: f64) {
        self.edit("Loop Region", true, |p| p.set_region_loop(id(region), end));
    }

    pub fn join_selected(mut self: Pin<&mut Self>) {
        let ids = self.as_mut().selection();
        let mut joined = None;
        self.as_mut().edit("Join Regions", true, |p| {
            joined = p.join_regions(&ids);
            joined.is_some()
        });
        match joined {
            Some(r) => self.set_selection(vec![r]),
            None => self.show_problem(
                "These regions can't be joined: pick regions on one track, either MIDI, or pieces of one recording side by side.",
            ),
        }
    }

    pub fn set_instrument_params(self: Pin<&mut Self>, track: i32, values: &QString, label: &QString) {
        let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&values.to_string()).unwrap_or_default();
        let label = label.to_string();
        self.edit(if label.is_empty() { "Instrument Settings" } else { &label }, true, |p| {
            let mut changed = false;
            for (key, v) in &map {
                if let Some(v) = v.as_f64() {
                    changed |= p.set_instrument_param(id(track), key, v as f32);
                }
            }
            changed
        });
    }

    pub fn set_instrument_param(self: Pin<&mut Self>, track: i32, key: &QString, value: f64, commit: bool) {
        let key = key.to_string();
        self.edit("Instrument Setting", commit, |p| p.set_instrument_param(id(track), &key, value as f32));
    }

    /// Points live notes (MIDI keyboards, the UI) at the running engine, and
    /// at the selected track when it is an instrument track.
    pub(crate) fn route_live(mut self: Pin<&mut Self>) {
        let selected = *self.selected_track();
        let st = self.as_mut().st();
        let Some(engine) = &mut st.audio.engine else { return };
        let queue = engine.live_notes();
        if let Ok(mut slot) = st.live_slot.lock()
            && !slot.as_ref().is_some_and(|q| std::sync::Arc::ptr_eq(q, &queue))
        {
            *slot = Some(queue);
        }
        let live = st.session.as_ref().and_then(|s| {
            s.project.tracks.iter().position(|t| selected >= 0 && t.id == selected as u64 && t.instrument.is_some())
        });
        if st.live_track != Some(live) {
            engine.set_live_track(live);
            st.live_track = Some(live);
        }
    }

    pub fn note_on(mut self: Pin<&mut Self>, key: i32, velocity: i32) {
        let note = LiveNote { on: true, key: key.clamp(0, 127) as u8, velocity: velocity.clamp(1, 127) as u8 };
        crate::midi_input::send(&self.as_mut().st().live_slot, note);
    }

    pub fn user_presets(&self, kind: &QString) -> QString {
        qs(&crate::user_presets::list(&kind.to_string()))
    }

    pub fn save_user_preset(self: Pin<&mut Self>, kind: &QString, name: &QString, values: &QString) {
        if let Err(err) = crate::user_presets::save(&kind.to_string(), name.to_string().trim(), &values.to_string()) {
            self.show_problem(&format!("Couldn't save the preset: {err:#}"));
        }
    }

    pub fn delete_user_preset(self: Pin<&mut Self>, kind: &QString, name: &QString) {
        if let Err(err) = crate::user_presets::delete(&kind.to_string(), &name.to_string()) {
            self.show_problem(&format!("Couldn't delete the preset: {err:#}"));
        }
    }

    pub fn sustain(mut self: Pin<&mut Self>, down: bool) {
        let note = LiveNote { on: down, key: isai_engine::instruments::SUSTAIN_PEDAL, velocity: 0 };
        crate::midi_input::send(&self.as_mut().st().live_slot, note);
    }

    pub fn note_off(mut self: Pin<&mut Self>, key: i32) {
        let note = LiveNote { on: false, key: key.clamp(0, 127) as u8, velocity: 0 };
        crate::midi_input::send(&self.as_mut().st().live_slot, note);
    }

    pub fn instrument_keys(mut self: Pin<&mut Self>) -> QString {
        let Some(engine) = &self.as_mut().st().audio.engine else { return QString::default() };
        let (held, struck, velocities) = engine.take_keys();
        let list = |m: u128| (0..128).filter(|k| m >> k & 1 == 1).collect::<Vec<u32>>();
        // How hard each held or struck key was played.
        let vel: serde_json::Map<String, serde_json::Value> =
            list(held | struck).into_iter().map(|k| (k.to_string(), velocities[k as usize].into())).collect();
        qs(&serde_json::json!({ "held": list(held), "struck": list(struck), "vel": vel }).to_string())
    }

    pub fn refresh_midi(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        st.midi = None; // close the old connections first
        let inputs = crate::midi_input::MidiInputs::open_all(&st.live_slot);
        let names = inputs.names.join(", ");
        st.midi = Some(inputs);
        self.set_midi_inputs(qs(&names));
    }

    // ----------------------------------------------------------- automation

    pub fn set_automation_view(mut self: Pin<&mut Self>, track: i32, target: &QString) {
        let target = target.to_string();
        let Some(s) = self.as_mut().st().session.as_mut() else { return };
        let Some(t) = s.project.track_mut(id(track)) else { return };
        t.automation_view = (!target.is_empty()).then_some(target);
        s.dirty = true;
        self.publish_project();
    }

    pub fn add_auto_point(mut self: Pin<&mut Self>, track: i32, target: &QString, beat: f64, value: f64) -> i32 {
        let target = target.to_string();
        let mut index = None;
        self.as_mut().edit("Add Automation Point", true, |p| {
            index = p.add_auto_point(id(track), &target, beat, value as f32);
            index.is_some()
        });
        index.map_or(-1, |i| i as i32)
    }

    pub fn move_auto_point(self: Pin<&mut Self>, track: i32, target: &QString, index: i32, beat: f64, value: f64) {
        let target = target.to_string();
        self.edit("Move Automation Point", true, |p| {
            p.move_auto_point(id(track), &target, index.max(0) as usize, beat, value as f32).is_some()
        });
    }

    pub fn remove_auto_point(self: Pin<&mut Self>, track: i32, target: &QString, index: i32) {
        let target = target.to_string();
        self.edit("Delete Automation Point", true, |p| p.remove_auto_point(id(track), &target, index.max(0) as usize));
    }

    pub fn clear_lane(self: Pin<&mut Self>, track: i32, target: &QString) {
        let target = target.to_string();
        self.edit("Clear Automation", true, |p| p.clear_lane(id(track), &target));
    }

    pub fn remove_insert(self: Pin<&mut Self>, insert: i32) {
        self.edit("Remove Effect", true, |p| p.remove_insert(id(insert)));
    }

    pub fn toggle_insert(self: Pin<&mut Self>, insert: i32) {
        self.edit("Bypass Effect", true, |p| p.insert_mut(id(insert)).map(|i| i.enabled = !i.enabled).is_some());
    }

    pub fn move_insert(self: Pin<&mut Self>, insert: i32, to: i32) {
        self.edit("Move Effect", true, |p| p.move_insert(id(insert), to.max(0) as usize));
    }

    pub fn set_insert_params(self: Pin<&mut Self>, insert: i32, values: &QString, label: &QString) {
        let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&values.to_string()).unwrap_or_default();
        let label = label.to_string();
        self.edit(if label.is_empty() { "Effect Settings" } else { &label }, true, |p| {
            let mut changed = false;
            for (key, v) in &map {
                if let Some(v) = v.as_f64() {
                    changed |= p.set_insert_param(id(insert), key, v as f32);
                }
            }
            changed
        });
    }

    pub fn set_insert_param(self: Pin<&mut Self>, insert: i32, key: &QString, value: f64, commit: bool) {
        let key = key.to_string();
        self.edit("Effect Setting", commit, |p| p.set_insert_param(id(insert), &key, value as f32));
    }

    pub fn select_track(mut self: Pin<&mut Self>, track: i32) {
        self.as_mut().set_selection(Vec::new());
        self.as_mut().set_selected_track(track);
        self.route_live();
    }

    /// Selects just this region (-1 clears the selection).
    pub fn select_region(self: Pin<&mut Self>, region: i32) {
        self.set_selection(if region >= 0 { vec![id(region)] } else { Vec::new() });
    }

    /// Moves a region to `start` on `track`. When it is part of a bigger
    /// selection, the whole selection moves along with it.
    pub fn move_region(mut self: Pin<&mut Self>, region: i32, start: f64, track: i32) {
        let ids = self.as_mut().selection();
        let to = (track >= 0).then(|| id(track));
        let region = id(region);
        if ids.len() > 1 && ids.contains(&region) {
            self.edit("Move Regions", true, |p| {
                let row = |t: u64| p.tracks.iter().position(|x| x.id == t).map_or(0, |r| r as i64);
                let Some((from_track, r)) = p.find_region(region) else { return false };
                let beats = p.snap(start) - r.start;
                let rows = to.map_or(0, |t| row(t) - row(from_track.id));
                p.move_regions(&ids, beats, rows)
            });
        } else {
            self.edit("Move Region", true, |p| p.move_region(region, start, to));
        }
    }

    /// Deletes the selected regions (Backspace / Delete).
    pub fn delete_selected(mut self: Pin<&mut Self>) {
        let ids = self.as_mut().selection();
        let label = if ids.len() > 1 { "Delete Regions" } else { "Delete Region" };
        if !ids.is_empty() && self.as_mut().edit(label, true, |p| p.remove_regions(&ids)) {
            self.set_selection(Vec::new());
        }
    }

    pub fn trim_region(self: Pin<&mut Self>, region: i32, left: bool, beats: f64) {
        self.edit("Trim Region", true, |p| p.trim_region(id(region), left, beats));
    }

    pub fn split_region(mut self: Pin<&mut Self>, region: i32, beats: f64) {
        let mut right = None;
        self.as_mut().edit("Split Region", true, |p| {
            right = p.split_region(id(region), beats);
            right.is_some()
        });
        if let Some(r) = right {
            self.set_selection(vec![r]);
        }
    }

    pub fn split_at_playhead(mut self: Pin<&mut Self>) {
        let at = *self.position_beats();
        let selected_region = *self.selected_region();
        let selected_track = *self.selected_track();
        let Some(session) = &self.as_mut().st().session else { return };
        let p = &session.project;
        // The selected region if the playhead crosses it; otherwise whatever the
        // playhead crosses on the selected track.
        let crosses = |r: &isai_project::Region| at > r.start && at < r.start + p.region_beats(r);
        let target = p
            .find_region(id(selected_region))
            .filter(|(_, r)| selected_region >= 0 && crosses(r))
            .map(|(_, r)| r.id)
            .or_else(|| {
                p.track(id(selected_track))?.regions.iter().find(|r| crosses(r)).map(|r| r.id)
            });
        if let Some(region) = target {
            self.split_region(region as i32, at);
        }
    }

    pub fn set_region_fades(self: Pin<&mut Self>, region: i32, fade_in: f64, fade_out: f64, commit: bool) {
        self.edit("Fade", commit, |p| {
            let Some(meter) = p.find_region(id(region)).map(|(_, r)| p.region_meter(r)) else { return false };
            p.set_fades(id(region), meter.beats_to_secs(fade_in.max(0.0)), meter.beats_to_secs(fade_out.max(0.0)))
        });
    }

    pub fn move_track(self: Pin<&mut Self>, track: i32, index: i32) {
        self.edit("Move Track", true, |p| p.move_track(id(track), index.max(0) as usize));
    }

    pub fn undo(mut self: Pin<&mut Self>) {
        let done = self.as_mut().st().session.as_mut().and_then(Session::undo);
        if let Some(label) = done {
            self.as_mut().after_history(&format!("Undid {label}"));
        }
    }

    pub fn redo(mut self: Pin<&mut Self>) {
        let done = self.as_mut().st().session.as_mut().and_then(Session::redo);
        if let Some(label) = done {
            self.as_mut().after_history(&format!("Redid {label}"));
        }
    }

    pub fn jump_history(mut self: Pin<&mut Self>, steps: i32) {
        let Some(session) = self.as_mut().st().session.as_mut() else { return };
        let mut done = 0;
        for _ in 0..steps.unsigned_abs() {
            let step = if steps < 0 { session.undo() } else { session.redo() };
            if step.is_none() {
                break;
            }
            done += 1;
        }
        if done > 0 {
            let what = if steps < 0 { "Undid" } else { "Redid" };
            self.after_history(&format!("{what} {done} step{}", if done == 1 { "" } else { "s" }));
        }
    }

    fn after_history(mut self: Pin<&mut Self>, message: &str) {
        let beats = *self.position_beats();
        self.as_mut().push_project_to_engine();
        self.as_mut().push_arrangement();
        self.as_mut().publish_project();
        self.as_mut().seek_beats(beats);
        self.show_info(message);
    }

    // ---------------------------------------------------------------- files

    pub fn save_project(mut self: Pin<&mut Self>) -> bool {
        let paths = self.as_mut().st().session.as_ref().and_then(|s| s.paths.clone());
        match paths {
            Some(paths) => self.save_to(paths),
            None => false,
        }
    }

    pub fn save_project_as(self: Pin<&mut Self>, url: &QUrl) -> bool {
        let Some(chosen) = url_path(url) else { return false };
        self.save_to(ProjectPaths::for_new(&chosen))
    }

    fn save_to(mut self: Pin<&mut Self>, paths: ProjectPaths) -> bool {
        let file = paths.file.display().to_string();
        let result = match self.as_mut().st().session.as_mut() {
            Some(session) => session.save_to(paths),
            None => return false,
        };
        match result {
            Ok(()) => {
                let st = self.as_mut().st();
                st.settings.remember_project(&file);
                if let Some(dir) = app_dir() {
                    Autosave::clear(&dir);
                }
                // Saving moved audio into the project folder: point the pool at it.
                let files = st.session.as_ref().map(Session::audio_paths).unwrap_or_default();
                for f in &files {
                    st.pool.request(f);
                }
                self.as_mut().save();
                self.as_mut().publish_recent();
                self.as_mut().publish_project();
                self.as_mut().show_info("Saved");
                self.update_busy();
                true
            }
            Err(err) => {
                self.show_problem(&format!("Couldn't save: {err:#}"));
                false
            }
        }
    }

    pub fn open_project(self: Pin<&mut Self>, url: &QUrl) {
        if let Some(path) = url_path(url) {
            self.open_path(&path);
        }
    }

    pub fn open_recent(self: Pin<&mut Self>, path: &QString) {
        self.open_path(Path::new(&path.to_string()));
    }

    pub(crate) fn open_path(mut self: Pin<&mut Self>, path: &Path) {
        match isai_project::load(path) {
            Ok((project, paths)) => {
                let st = self.as_mut().st();
                st.settings.remember_project(&path.display().to_string());
                self.as_mut().save();
                self.as_mut().publish_recent();
                self.open_session(Session::new(project, Some(paths)));
            }
            Err(err) => self.show_problem(&format!("Couldn't open {}: {err:#}", path.display())),
        }
    }

    pub(crate) fn check_recovery(mut self: Pin<&mut Self>) {
        let found = app_dir().and_then(|d| Autosave::read(&d));
        match found {
            Some(a) => {
                self.as_mut().set_recovery_age(qs(&ago(a.saved_at)));
                self.set_recovery_name(qs(&a.project.name));
            }
            None => self.set_recovery_name(QString::default()),
        }
    }

    pub fn recover_autosave(mut self: Pin<&mut Self>) {
        let Some(found) = app_dir().and_then(|d| Autosave::read(&d)) else { return };
        let paths = found.paths();
        let mut session = Session::new(found.project, paths);
        // Recovered work hasn't been saved to its file yet.
        session.dirty = true;
        self.as_mut().set_recovery_name(QString::default());
        self.as_mut().open_session(session);
        self.show_info("Recovered your unsaved work. Save it to keep it.");
    }

    pub fn discard_autosave(self: Pin<&mut Self>) {
        if let Some(dir) = app_dir() {
            Autosave::clear(&dir);
        }
        self.set_recovery_name(QString::default());
    }

    pub fn autosave(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(s) = &st.session else { return };
        if !s.dirty {
            return;
        }
        if let Some(dir) = app_dir()
            && let Err(err) = Autosave::new(&s.project, s.paths.as_ref()).write(&dir)
        {
            eprintln!("autosave failed: {err:#}");
        }
    }

    pub fn discard_changes(mut self: Pin<&mut Self>) {
        if let Some(dir) = app_dir() {
            Autosave::clear(&dir);
        }
        if let Some(s) = self.as_mut().st().session.as_mut() {
            s.dirty = false;
        }
        self.publish_file_state();
    }

    // ---------------------------------------------------------- audio loading

    /// Takes finished audio loads: imports become tracks, and regions that were
    /// waiting for their audio start playing.
    pub(crate) fn poll_audio(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let done = st.pool.poll();
        if done.is_empty() {
            return;
        }
        let mut failures = Vec::new();
        let mut imports = Vec::new();
        for path in &done {
            while let Some(i) = st.pending_imports.iter().position(|(p, _)| p == path) {
                let (_, target) = st.pending_imports.remove(i);
                match st.pool.ready(path) {
                    Some(clip) => imports.push((path.clone(), clip.duration_secs(), target)),
                    None => failures.push(path.clone()),
                }
            }
            if st.preview_pending.as_ref().is_some_and(|(p, _)| p == path) {
                let (_, sync) = st.preview_pending.take().unwrap_or_default();
                if let (Some(clip), Some(engine)) = (st.pool.ready(path).cloned(), &mut st.audio.engine) {
                    engine.preview(Some(clip), sync);
                }
            }
        }
        let still_loading = !st.pending_imports.is_empty();
        self.as_mut().set_loading(still_loading);

        let at = *self.position_beats();
        for (path, secs, target) in imports {
            let name = file_stem(&path);
            let file = path.display().to_string();
            let mut onto = 0;
            self.as_mut().edit("Import Audio", false, |p| {
                // Onto the audio track it was dropped on, else a new track at the playhead.
                let place = target.filter(|(t, _)| p.track(*t).is_some_and(|t| t.instrument.is_none()));
                let start = p.snap(place.map_or(at, |(_, b)| b));
                onto = match place {
                    Some((t, _)) => t,
                    None => p.add_track(Some(&name)),
                };
                p.add_region(onto, &file, &name, start, secs).is_some()
            });
            self.as_mut().set_selected_track(onto as i32);
        }
        if let Some(path) = failures.first() {
            let reason = match self.as_mut().st().pool.get(path) {
                Some(crate::pool::AudioState::Failed(why)) => why.clone(),
                _ => "unsupported or damaged file".into(),
            };
            self.as_mut().show_problem(&format!("Couldn't import {}: {reason}", file_stem(path)));
        }
        // Regions waiting for audio can now play, and the view shows them ready.
        self.as_mut().push_arrangement();
        self.publish_project();
    }
}

/// Note indices for QML, comma-separated ("" for none).
fn index_text(indices: Option<Vec<usize>>) -> QString {
    qs(&indices.unwrap_or_default().iter().map(usize::to_string).collect::<Vec<_>>().join(","))
}

/// A comma-separated list of indices from QML.
fn index_list(s: &QString) -> Vec<usize> {
    s.to_string().split(',').filter_map(|s| s.trim().parse().ok()).collect()
}

/// EQ settings from QML's { key: value } JSON (missing keys keep their defaults).
fn eq_params(values: &QString) -> [f32; MAX_PARAMS] {
    let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&values.to_string()).unwrap_or_default();
    let mut params = EffectKind::Eq.defaults();
    for (p, def) in params.iter_mut().zip(EffectKind::Eq.params()) {
        if let Some(v) = map.get(def.key).and_then(serde_json::Value::as_f64) {
            *p = v as f32;
        }
    }
    params
}

/// `points` frequencies from 20 Hz to 20 kHz, evenly spaced on a log scale.
fn curve_freqs(points: i32) -> Vec<f32> {
    let n = points.clamp(2, 400);
    (0..n).map(|i| 20.0 * 1000f32.powf(i as f32 / (n - 1) as f32)).collect()
}
