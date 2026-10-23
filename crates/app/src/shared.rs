//! Waveform summaries the region painter reads, keyed by audio file path.
//!
//! The audio pool writes an entry when a file finishes loading. Qt calls
//! `paint` while the GUI thread is blocked, so the lock is never contended.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use isai_engine::Peaks;

pub struct WavePeaks {
    pub peaks: Peaks,
    pub sample_rate: u32,
}

static PEAKS: Mutex<Option<HashMap<String, Arc<WavePeaks>>>> = Mutex::new(None);

pub fn set_peaks(path: &Path, peaks: Arc<WavePeaks>) {
    if let Ok(mut map) = PEAKS.lock() {
        map.get_or_insert_with(HashMap::new)
            .insert(path.display().to_string(), peaks);
    }
}

pub fn peaks(key: &str) -> Option<Arc<WavePeaks>> {
    PEAKS.lock().ok()?.as_ref()?.get(key).cloned()
}
