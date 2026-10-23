//! Splitting an audio region into vocals, drums, bass and other.
//!
//! The model (Deezer's Spleeter, MIT) isn't shipped with Isai. The first split
//! downloads Deezer's release with the system's `curl` (in macOS, Windows 10+
//! and Linux; no TLS library is bundled), checks it against Deezer's checksum,
//! converts it to a 79 MB weights file and deletes the download. Everything
//! runs on a background thread; the UI polls progress.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering::Relaxed};
use std::thread::JoinHandle;
use std::time::Duration;

use cxx_qt_lib::QString;
use isai_engine::clip::Clip;
use isai_stems::{SAMPLE_RATE, STEMS, Separator, Weights, model};

use crate::controller::qobject;
use crate::project_ops::app_dir;

#[derive(Default)]
pub struct StemControl {
    progress: AtomicU32,
    cancel: AtomicBool,
    /// 0 downloading, 1 preparing, 2 splitting.
    stage: AtomicU8,
}

impl StemControl {
    fn set_progress(&self, p: f32) {
        self.progress.store(p.clamp(0.0, 1.0).to_bits(), Relaxed);
    }
    fn progress(&self) -> f32 {
        f32::from_bits(self.progress.load(Relaxed))
    }
    fn cancelled(&self) -> bool {
        self.cancel.load(Relaxed)
    }
}

/// Each stem's name, file and length in seconds.
type Stems = Vec<(String, PathBuf, f64)>;

pub struct StemJob {
    pub region: u64,
    control: Arc<StemControl>,
    handle: JoinHandle<Result<Stems, String>>,
}

/// Where the converted model is kept.
pub fn model_path() -> Option<PathBuf> {
    app_dir().map(|d| d.join("models").join("spleeter-4stems.isaistems"))
}

pub fn model_ready() -> bool {
    model_path().is_some_and(|p| p.exists())
}

/// Downloads and converts the model (stage 0), reporting progress.
fn fetch_model(dest: &Path, control: &StemControl) -> Result<(), String> {
    let dir = dest.parent().ok_or("no folder for the model")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let part = dir.join("4stems.tar.gz.part");
    let mut child = Command::new("curl")
        .args([
            "--location",
            "--fail",
            "--silent",
            "--show-error",
            "--output",
        ])
        .arg(&part)
        .arg(model::MODEL_URL)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| {
            "Couldn't start the download: curl isn't available on this computer".to_string()
        })?;
    loop {
        if control.cancelled() {
            let _ = child.kill();
            let _ = std::fs::remove_file(&part);
            return Err("cancelled".into());
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            if !status.success() {
                let _ = std::fs::remove_file(&part);
                return Err("Couldn't download the stem model. Check your internet connection and try again.".into());
            }
            break;
        }
        let got = std::fs::metadata(&part).map_or(0, |m| m.len());
        control.set_progress(got as f32 / model::MODEL_BYTES as f32);
        std::thread::sleep(Duration::from_millis(150));
    }
    control.stage.store(1, Relaxed);
    let bytes = std::fs::read(&part).map_err(|e| e.to_string())?;
    let converted = model::convert_release(&bytes);
    let _ = std::fs::remove_file(&part);
    let file = converted?;
    let tmp = dest.with_extension("tmp");
    std::fs::write(&tmp, file).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

/// The whole job, on a background thread: the model if needed, then the split.
fn run(
    source: PathBuf,
    offset: f64,
    length: f64,
    name: String,
    out_dir: PathBuf,
    control: Arc<StemControl>,
) -> Result<Stems, String> {
    let model_file = model_path().ok_or("no settings folder for the stem model")?;
    if !model_file.exists() {
        fetch_model(&model_file, &control)?;
    }
    control.stage.store(1, Relaxed);
    control.set_progress(0.0);
    let weights = Weights::load(&std::fs::read(&model_file).map_err(|e| e.to_string())?)?;
    let separator = Separator::new(&weights)?;
    drop(weights);
    let clip =
        Clip::load(&source, SAMPLE_RATE).map_err(|e| format!("Couldn't read the audio: {e:#}"))?;
    let rate = f64::from(SAMPLE_RATE);
    let from = ((offset * rate) as usize).min(clip.left.len());
    let to = (((offset + length) * rate) as usize).clamp(from, clip.left.len());
    let (left, right) = (&clip.left[from..to], &clip.right[from..to]);

    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let paths: Vec<PathBuf> = STEMS
        .iter()
        .map(|s| out_dir.join(format!("{name} - {s} {stamp}.wav")))
        .collect();
    let mut writers = Vec::new();
    for p in &paths {
        writers.push(hound::WavWriter::create(p, spec).map_err(|e| e.to_string())?);
    }
    control.stage.store(2, Relaxed);
    // Leave a core for the audio, so playback keeps going smoothly.
    let threads =
        std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).max(1));
    let mut failed = None;
    let done = separator.separate(
        left,
        right,
        threads,
        |stem, l, r| {
            for (a, b) in l.iter().zip(r) {
                if let Err(e) = writers[stem]
                    .write_sample(*a)
                    .and_then(|_| writers[stem].write_sample(*b))
                {
                    failed = Some(e.to_string());
                }
            }
        },
        |p| control.set_progress(p),
        || control.cancelled(),
    );
    let finish: Result<(), String> = writers
        .into_iter()
        .try_for_each(|w| w.finalize().map_err(|e| e.to_string()));
    if !done || failed.is_some() || finish.is_err() {
        for p in &paths {
            let _ = std::fs::remove_file(p);
        }
        return Err(failed
            .or(finish.err())
            .unwrap_or_else(|| "cancelled".into()));
    }
    let secs = left.len() as f64 / rate;
    Ok(STEMS
        .iter()
        .zip(paths)
        .map(|(s, p)| ((*s).to_owned(), p, secs))
        .collect())
}

fn safe_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim().to_owned();
    if s.is_empty() { "Stems".into() } else { s }
}

impl qobject::AppController {
    pub fn stem_model_ready(&self) -> bool {
        model_ready()
    }

    /// Splits an audio region into four new tracks (downloading the model first if needed).
    pub fn split_stems(mut self: Pin<&mut Self>, region: i32) {
        let st = self.as_mut().st();
        if st.stems.is_some() {
            return;
        }
        let Some(session) = &st.session else { return };
        let Some((_, r)) = session.project.find_region(region as u64) else {
            return;
        };
        if r.midi.is_some() {
            self.show_problem("Stems come from audio: pick an audio region");
            return;
        }
        let source = session.resolve(&r.file);
        let (offset, length, name) = (r.offset, r.length, safe_name(&r.name));
        let out_dir = match &session.paths {
            Some(p) => p.audio_dir(),
            None => match app_dir() {
                Some(d) => d.join("Stems"),
                None => return,
            },
        };
        let control = Arc::new(StemControl::default());
        let worker = control.clone();
        let handle = std::thread::spawn(move || run(source, offset, length, name, out_dir, worker));
        st.stems = Some(StemJob {
            region: region as u64,
            control,
            handle,
        });
        self.as_mut().set_stem_region(region);
        self.as_mut().set_stem_progress(0.0);
        self.as_mut()
            .set_stem_stage(QString::from(if model_ready() {
                "Preparing…"
            } else {
                "Downloading the stem model…"
            }));
        self.update_busy();
    }

    pub fn cancel_stems(mut self: Pin<&mut Self>) {
        if let Some(job) = &self.as_mut().st().stems {
            job.control.cancel.store(true, Relaxed);
        }
    }

    pub(crate) fn poll_stems(mut self: Pin<&mut Self>) {
        let st = self.as_mut().st();
        let Some(job) = &st.stems else { return };
        if !job.handle.is_finished() {
            let (p, stage) = (
                f64::from(job.control.progress()),
                job.control.stage.load(Relaxed),
            );
            let label = match stage {
                0 => "Downloading the stem model…",
                1 => "Preparing…",
                _ => "Splitting into stems…",
            };
            self.as_mut().set_stem_progress(p);
            if self.stem_stage().to_string() != label {
                self.as_mut().set_stem_stage(QString::from(label));
            }
            return;
        }
        let job = st.stems.take().expect("checked above");
        self.as_mut().set_stem_region(-1);
        self.as_mut().set_stem_progress(-1.0);
        self.as_mut().set_stem_stage(QString::default());
        match job.handle.join() {
            Ok(Ok(files)) => {
                for (_, path, _) in &files {
                    self.as_mut().st().pool.request(path);
                }
                let stems: Vec<(String, String, f64)> = files
                    .iter()
                    .map(|(n, p, s)| (n.clone(), p.display().to_string(), *s))
                    .collect();
                self.as_mut().edit("Split into Stems", true, |p| {
                    p.add_stem_tracks(job.region, &stems)
                });
                self.as_mut().show_info(
                    "Split into vocals, drums, bass and other (the original track is muted)",
                );
            }
            Ok(Err(e)) if e == "cancelled" => {}
            Ok(Err(e)) => self.as_mut().show_problem(&e),
            Err(_) => self.as_mut().show_problem("Couldn't split the stems"),
        }
        self.update_busy();
    }
}
