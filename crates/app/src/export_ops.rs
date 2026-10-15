//! The controller's export side: bouncing the mix or stems in the background.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::thread::JoinHandle;

use cxx_qt_lib::{QString, QUrl};
use isai_engine::Arrangement;
use isai_engine::export::{ExportControl, ExportOptions, ExportReport, Format};

use isai_project::{Chain, EffectKind};
use serde_json::json;

use crate::controller::{qobject::AppController, qs};

/// A running export job.
pub struct ExportJob {
    pub control: Arc<ExportControl>,
    pub handle: JoinHandle<anyhow::Result<Vec<ExportReport>>>,
    /// Where to point "Show in Finder" afterwards.
    pub folder: PathBuf,
}

/// A running freeze: one track rendered to a file in the background.
pub struct FreezeJob {
    pub track: u64,
    pub path: PathBuf,
    pub control: Arc<ExportControl>,
    pub handle: JoinHandle<anyhow::Result<ExportReport>>,
}

/// The mastering assistant measuring the mix in the background.
pub struct MasterJob {
    pub target: f32,
    pub control: Arc<ExportControl>,
    pub handle: JoinHandle<Option<crate::mastering::Plan>>,
}

/// Where the sound in an arrangement ends, in samples: the last audio, or the
/// last note plus time for instruments to ring out.
fn content_end(a: &Arrangement, rate: u32) -> u64 {
    let audio = a.tracks.iter().flat_map(|t| &t.regions).map(|r| r.start + r.len).max().unwrap_or(0);
    let notes = a.tracks.iter().flat_map(|t| &t.notes).map(|n| n.end).max().unwrap_or(0);
    // A short tail after audio; two seconds after notes for releases and reverbs.
    let audio = if audio > 0 { audio + u64::from(rate) / 10 } else { 0 };
    let notes = if notes > 0 { notes + u64::from(rate) * 2 } else { 0 };
    audio.max(notes)
}

/// File names can't contain path separators and a few other characters.
fn safe_name(name: &str) -> String {
    name.chars().map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c }).collect()
}

impl AppController {
    /// Exports the mix (to the file `url`) or stems (into the folder `url`).
    /// `format`: wav16, wav24, wav32f, flac16, flac24.
    pub fn export_audio(mut self: Pin<&mut Self>, url: &QUrl, stems: bool, cycle_only: bool, format: &QString, normalize: bool) {
        let Some(target) = url.to_local_file().map(|p| PathBuf::from(p.to_string())) else { return };
        let Some(format) = Format::from_key(&format.to_string()) else { return };
        let volume = *self.volume() as f32;
        let st = self.as_mut().st();
        if st.export.is_some() {
            return;
        }
        let Some(session) = &st.session else { return };
        let Some(engine) = &st.audio.engine else {
            self.show_problem("Exporting needs the audio engine running (check Settings → Audio)");
            return;
        };
        if st.pool.is_loading() {
            self.show_problem("Still loading audio; try again in a moment");
            return;
        }
        let rate = engine.info().sample_rate;
        let project = &session.project;
        let meter = project.meter();
        // Exports always use full quality, whatever Eco mode is set to.
        let arrangement = session.arrangement(&st.pool, false);

        // The range: the cycle region, or the whole song up to the end of the last region.
        let (start, end) = if cycle_only {
            (meter.beats_to_samples(project.cycle_start, rate), meter.beats_to_samples(project.cycle_end, rate))
        } else {
            (0, content_end(&arrangement, rate))
        };
        if end <= start {
            self.show_problem("Nothing to export: add some audio first");
            return;
        }

        // One job per output file: the mix, or each unmuted track that has audio.
        let mut jobs: Vec<(Arrangement, PathBuf)> = Vec::new();
        let folder;
        if stems {
            folder = target.clone();
            let project_name = safe_name(&project.name);
            // Each stem keeps its sends, so its share of a shared reverb comes with it.
            // Master effects (a limiter, say) are left off: stems are for mixing elsewhere.
            let buses = arrangement.buses;
            let bpm = arrangement.bpm;
            for (track, play) in project.tracks.iter().zip(arrangement.tracks) {
                if track.mute || (play.regions.is_empty() && play.notes.is_empty()) {
                    continue;
                }
                let mut solo = play;
                solo.audible = true; // stems ignore solo
                let name = format!("{project_name} - {}.{}", safe_name(&track.name), format.extension());
                jobs.push((Arrangement { tracks: vec![solo], buses: buses.clone(), master: Vec::new(), bpm }, target.join(name)));
            }
            if jobs.is_empty() {
                self.show_problem("Nothing to export: every track is muted or empty");
                return;
            }
        } else {
            let path = target.with_extension(format.extension());
            folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
            jobs.push((arrangement, path));
        }

        let control = Arc::new(ExportControl::default());
        let worker = control.clone();
        let opts = ExportOptions { format, sample_rate: rate, start, end, gain: volume, normalize };
        let handle = std::thread::spawn(move || {
            jobs.iter().map(|(a, path)| isai_engine::export::export(a, &opts, path, &worker)).collect()
        });
        st.export = Some(ExportJob { control, handle, folder });
        self.as_mut().set_exporting(true);
        self.as_mut().set_export_progress(0.0);
        self.update_busy();
    }

    pub fn freeze_track(mut self: Pin<&mut Self>, track: i32) {
        let track = crate::project_ops::id(track);
        let st = self.as_mut().st();
        if st.freeze.is_some() {
            return;
        }
        let Some(session) = &st.session else { return };
        let Some(engine) = &st.audio.engine else { return };
        if st.pool.is_loading() {
            self.show_problem("Still loading audio; try again in a moment");
            return;
        }
        let rate = engine.info().sample_rate;
        // Render what the track plays without any earlier freeze.
        let mut project = session.project.clone();
        let Some(index) = project.tracks.iter().position(|t| t.id == track) else { return };
        project.tracks[index].frozen = None;
        let name = safe_name(&project.tracks[index].name);
        let paths = session.paths.clone();
        let thawed = crate::session::Session::new(project, paths.clone());
        let mut all = thawed.arrangement(&st.pool, false);
        let bpm = all.bpm;
        let mut solo = all.tracks.swap_remove(index);
        // Up to the fader: its effects and their automation, but not the fader, pan or sends.
        solo.gain = 1.0;
        solo.pan = 0.0;
        solo.audible = true;
        solo.sends.clear();
        solo.automation.retain(|l| matches!(l.target, isai_engine::AutoTarget::Param { .. }));
        let single = Arrangement { tracks: vec![solo], buses: Vec::new(), master: Vec::new(), bpm };
        let end = content_end(&single, rate);
        if end == 0 {
            self.show_problem("Nothing to freeze: this track has no audio or notes yet");
            return;
        }
        let dir = match &paths {
            Some(p) => p.audio_dir(),
            None => match crate::project_ops::app_dir() {
                Some(d) => d.join("Frozen"),
                None => return,
            },
        };
        if let Err(err) = std::fs::create_dir_all(&dir) {
            self.show_problem(&format!("Couldn't freeze: {err}"));
            return;
        }
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let path = dir.join(format!("{name} (frozen {stamp}).wav"));
        let control = Arc::new(ExportControl::default());
        let worker = control.clone();
        let opts = ExportOptions { format: Format::Wav32Float, sample_rate: rate, start: 0, end, gain: 1.0, normalize: false };
        let out = path.clone();
        let handle = std::thread::spawn(move || isai_engine::export::export(&single, &opts, &out, &worker));
        st.freeze = Some(FreezeJob { track, path, control, handle });
        self.as_mut().set_freezing_track(track as i32);
        self.update_busy();
    }

    pub fn unfreeze_track(self: Pin<&mut Self>, track: i32) {
        self.edit("Unfreeze Track", true, |p| p.track_mut(crate::project_ops::id(track)).and_then(|t| t.frozen.take()).is_some());
    }

    /// When a freeze finishes, the track switches to its rendered audio.
    pub(crate) fn poll_freeze(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(job) = &st.freeze else { return };
        if !job.handle.is_finished() {
            let progress = f64::from(job.control.progress());
            self.set_freeze_progress(progress);
            return;
        }
        let job = st.freeze.take().expect("checked above");
        self.as_mut().set_freezing_track(-1);
        match job.handle.join() {
            Ok(Ok(_)) => {
                let file = job.path.display().to_string();
                self.as_mut().st().pool.request(&job.path);
                self.as_mut().edit("Freeze Track", true, |p| p.track_mut(job.track).map(|t| t.frozen = Some(file)).is_some());
                self.as_mut().show_info("Frozen: the track now plays its rendered audio (unfreeze to edit it)");
            }
            Ok(Err(err)) => self.as_mut().show_problem(&format!("Couldn't freeze: {err:#}")),
            Err(_) => self.as_mut().show_problem("Couldn't freeze the track"),
        }
        self.update_busy();
    }

    pub fn analyse_master(mut self: Pin<&mut Self>, target: f64) {
        let st = self.as_mut().st();
        if st.master_job.is_some() {
            return;
        }
        let (Some(session), Some(engine)) = (&st.session, &st.audio.engine) else { return };
        if st.pool.is_loading() {
            self.show_problem("Still loading audio; try again in a moment");
            return;
        }
        let rate = engine.info().sample_rate;
        // The mix as it is, without master effects: the assistant builds those.
        let mut arrangement = session.arrangement(&st.pool, false);
        arrangement.master.clear();
        let end = content_end(&arrangement, rate);
        if end == 0 {
            self.show_problem("Nothing to master yet: add some audio or notes first");
            return;
        }
        let target = target.clamp(-24.0, -6.0) as f32;
        let control = Arc::new(ExportControl::default());
        let worker = control.clone();
        let handle = std::thread::spawn(move || {
            let audio = isai_engine::export::render(&arrangement, rate, 0, end, 1.0, &worker).ok()?;
            crate::mastering::plan(&audio, rate, target, &|| worker.is_cancelled())
        });
        st.master_job = Some(MasterJob { target, control, handle });
        st.master_plan = None;
        self.as_mut().set_mastering(true);
        self.as_mut().set_master_report(QString::default());
        self.update_busy();
    }

    pub fn cancel_master(mut self: Pin<&mut Self>) {
        if let Some(job) = &self.as_mut().st().master_job {
            job.control.cancel();
        }
    }

    pub fn apply_master_plan(mut self: Pin<&mut Self>) {
        let Some(plan) = self.as_mut().st().master_plan.clone() else { return };
        self.edit("Mastering Assistant", true, |p| {
            p.master_inserts.clear();
            for (kind, values) in [(EffectKind::Eq, &plan.eq), (EffectKind::Compressor, &plan.compressor), (EffectKind::Limiter, &plan.limiter)] {
                let Some(id) = p.add_insert(Chain::Master, kind) else { return false };
                for (key, v) in values {
                    p.set_insert_param(id, key, *v);
                }
            }
            true
        });
    }

    /// When the assistant has measured the mix: its report for the dialog.
    pub(crate) fn poll_master(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(job) = &st.master_job else { return };
        if !job.handle.is_finished() {
            return;
        }
        let job = st.master_job.take().expect("checked above");
        self.as_mut().set_mastering(false);
        let Ok(Some(plan)) = job.handle.join() else {
            self.as_mut().show_problem("The mastering assistant couldn't measure the mix");
            return;
        };
        let get = |list: &[(&str, f32)], key: &str| list.iter().find(|(k, _)| *k == key).map_or(0.0, |(_, v)| *v);
        let mut changes = vec!["Low cut at 25 Hz (rumble you can't hear)".to_owned()];
        for (key, freq, what) in [("ls_gain", "120 Hz", "Bass"), ("hs_gain", "9 kHz", "Air")] {
            let g = get(&plan.eq, key);
            if g != 0.0 {
                changes.push(format!("{what} {} {:.1} dB at {freq}", if g > 0.0 { "up" } else { "down" }, g.abs()));
            }
        }
        changes.push(format!("Compressor 2:1 from {:.0} dB, slow attack (holds the mix together)", get(&plan.compressor, "threshold")));
        changes.push(format!("Limiter +{:.1} dB, ceiling −1 dB (loudness without clipping)", get(&plan.limiter, "gain")));
        let a = |x: &crate::mastering::Analysis| json!({ "lufs": x.lufs, "peak": x.peak_db, "low": x.low_db, "high": x.high_db });
        let report = json!({ "target": job.target, "before": a(&plan.before), "after": a(&plan.after), "changes": changes });
        self.as_mut().set_master_report(qs(&report.to_string()));
        self.as_mut().st().master_plan = Some(plan);
        self.update_busy();
    }

    pub fn cancel_export(mut self: Pin<&mut Self>) {
        if let Some(job) = &self.as_mut().st().export {
            job.control.cancel();
        }
    }

    /// Progress, and the result once the export finishes.
    pub(crate) fn poll_export(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(job) = &st.export else { return };
        let progress = f64::from(job.control.progress());
        if !job.handle.is_finished() {
            self.set_export_progress(progress);
            return;
        }
        let job = st.export.take().expect("checked above");
        let folder = job.folder.display().to_string();
        self.as_mut().set_exporting(false);
        self.as_mut().set_export_progress(1.0);
        let (ok, message) = match job.handle.join() {
            Ok(Ok(reports)) => {
                self.as_mut().set_export_folder(qs(&folder));
                let clipped = reports.iter().any(|r| r.clipped);
                let peak = reports.iter().map(|r| r.peak_db).fold(f32::MIN, f32::max);
                let what = if reports.len() == 1 { "Exported the mix".to_owned() } else { format!("Exported {} stems", reports.len()) };
                if clipped {
                    (false, format!("{what}, but it clipped (peak {peak:+.1} dB). Turn the volume down or use Normalize."))
                } else {
                    (true, format!("{what} (peak {peak:.1} dB)"))
                }
            }
            Ok(Err(err)) if err.to_string() == "cancelled" => (true, "Export cancelled".into()),
            Ok(Err(err)) => (false, format!("Export failed: {err:#}")),
            Err(_) => (false, "Export failed unexpectedly".into()),
        };
        if ok {
            self.as_mut().show_info(&message);
        } else {
            self.as_mut().show_problem(&message);
        }
        self.update_busy();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn file_names_are_made_safe() {
        assert_eq!(super::safe_name("Drums/Perc: take 2?"), "Drums_Perc_ take 2_");
    }
}
