//! Decoded audio for every file the project uses, loaded in the background.
//!
//! Files are decoded at the engine's sample rate. Each finished file also
//! publishes its waveform summary for the region painter. Regions that follow
//! the song's tempo get a time-stretched copy, made in the background too
//! (one stretch per file: the one the current tempo needs).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

use isai_engine::{Clip, Peaks};

use crate::shared::{self, WavePeaks};

pub enum AudioState {
    Loading,
    Ready(Arc<Clip>),
    Failed(String),
}

type Loaded = (PathBuf, u32, anyhow::Result<Clip>);
/// A stretched copy: the file, the stretch (in ten-thousandths) and the sample rate.
type Stretched = (PathBuf, u32, u32, Clip);

/// A stretch amount as a map key, to four decimal places.
fn stretch_key(ratio: f64) -> u32 {
    (ratio * 10_000.0).round() as u32
}

pub struct AudioPool {
    rate: u32,
    entries: HashMap<PathBuf, AudioState>,
    tx: mpsc::Sender<Loaded>,
    rx: mpsc::Receiver<Loaded>,
    /// Stretched copies by file: (stretch key, the copy, or `None` while it's made).
    stretched: HashMap<PathBuf, (u32, Option<Arc<Clip>>)>,
    stretch_tx: mpsc::Sender<Stretched>,
    stretch_rx: mpsc::Receiver<Stretched>,
}

impl Default for AudioPool {
    fn default() -> Self {
        Self::new(48_000)
    }
}

impl AudioPool {
    pub fn new(rate: u32) -> Self {
        let (tx, rx) = mpsc::channel();
        let (stretch_tx, stretch_rx) = mpsc::channel();
        Self {
            rate,
            entries: HashMap::new(),
            tx,
            rx,
            stretched: HashMap::new(),
            stretch_tx,
            stretch_rx,
        }
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Starts loading `path` unless it is already loaded or loading.
    pub fn request(&mut self, path: &Path) {
        if self.entries.contains_key(path) {
            return;
        }
        self.entries.insert(path.to_path_buf(), AudioState::Loading);
        let (tx, path, rate) = (self.tx.clone(), path.to_path_buf(), self.rate);
        std::thread::spawn(move || {
            let result = Clip::load(&path, rate);
            let _ = tx.send((path, rate, result));
        });
    }

    /// Starts making a copy of `path` stretched by `ratio` (once the file is
    /// loaded), replacing any other stretch of it.
    pub fn request_stretch(&mut self, path: &Path, ratio: f64) {
        let key = stretch_key(ratio);
        if key == 10_000 || self.stretched.get(path).is_some_and(|(k, _)| *k == key) {
            return;
        }
        let Some(clip) = self.ready(path).cloned() else {
            return;
        };
        self.stretched.insert(path.to_path_buf(), (key, None));
        let (tx, path, rate) = (self.stretch_tx.clone(), path.to_path_buf(), self.rate);
        std::thread::spawn(move || {
            let out = isai_engine::stretch::stretch(&clip, f64::from(key) / 10_000.0);
            let _ = tx.send((path, key, rate, out));
        });
    }

    /// The copy of `path` stretched by `ratio`, once it's ready.
    pub fn stretched(&self, path: &Path, ratio: f64) -> Option<&Arc<Clip>> {
        match self.stretched.get(path) {
            Some((key, Some(clip))) if *key == stretch_key(ratio) => Some(clip),
            _ => None,
        }
    }

    /// Takes finished loads. Returns the files that just became ready or failed.
    pub fn poll(&mut self) -> Vec<PathBuf> {
        let mut done = Vec::new();
        while let Ok((path, key, rate, clip)) = self.stretch_rx.try_recv() {
            // Only the stretch still wanted, at the current sample rate.
            if rate == self.rate && self.stretched.get(&path).is_some_and(|(k, _)| *k == key) {
                self.stretched
                    .insert(path.clone(), (key, Some(Arc::new(clip))));
                done.push(path);
            }
        }
        while let Ok((path, rate, result)) = self.rx.try_recv() {
            // A load started before a sample-rate change is out of date.
            if rate != self.rate {
                continue;
            }
            let state = match result {
                Ok(clip) => {
                    let peaks = Peaks::compute(&clip);
                    shared::set_peaks(
                        &path,
                        Arc::new(WavePeaks {
                            peaks,
                            sample_rate: clip.sample_rate,
                        }),
                    );
                    AudioState::Ready(Arc::new(clip))
                }
                Err(err) => AudioState::Failed(format!("{err:#}")),
            };
            self.entries.insert(path.clone(), state);
            done.push(path);
        }
        done
    }

    pub fn get(&self, path: &Path) -> Option<&AudioState> {
        self.entries.get(path)
    }

    pub fn ready(&self, path: &Path) -> Option<&Arc<Clip>> {
        match self.entries.get(path) {
            Some(AudioState::Ready(clip)) => Some(clip),
            _ => None,
        }
    }

    pub fn is_loading(&self) -> bool {
        self.entries
            .values()
            .any(|s| matches!(s, AudioState::Loading))
            || self.stretched.values().any(|(_, c)| c.is_none())
    }

    /// The device's sample rate changed: everything must be decoded again.
    pub fn set_rate(&mut self, rate: u32) {
        if rate == self.rate {
            return;
        }
        self.rate = rate;
        let paths: Vec<PathBuf> = self.entries.keys().cloned().collect();
        self.entries.clear();
        self.stretched.clear();
        for path in paths {
            self.request(&path);
        }
    }

    /// Forgets files no longer used, to free their memory.
    pub fn retain(&mut self, used: &[PathBuf]) {
        self.entries.retain(|path, _| used.contains(path));
        self.stretched.retain(|path, _| used.contains(path));
    }
}
