//! Writes a take to disk while it is being recorded.
//!
//! The audio thread fills the engine's record queue; this thread empties it
//! into a 32-bit float WAV file, so takes of any length never fill memory.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

/// A finished take.
pub struct Take {
    pub path: PathBuf,
    /// Stereo frames written.
    pub frames: u64,
}

/// What the writer thread hands back: the queue, and the file and frame count.
type WriterResult = (rtrb::Consumer<f32>, anyhow::Result<(PathBuf, u64)>);

pub struct Recorder {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<WriterResult>>,
}

impl Recorder {
    pub fn start(mut queue: rtrb::Consumer<f32>, path: &Path, sample_rate: u32) -> Self {
        // Anything left from an earlier take isn't part of this one.
        let stale = queue.slots();
        if let Ok(chunk) = queue.read_chunk(stale) {
            chunk.commit_all();
        }
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let path = path.to_path_buf();
        let handle = std::thread::spawn(move || {
            let result = write(&mut queue, &path, sample_rate, &flag).map(|frames| (path, frames));
            (queue, result)
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }

    /// Asks the writer to finish once the queue is empty.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Once the writer is done: the engine's record queue (to give back for
    /// the next take) and the take, or why it couldn't be written.
    pub fn finished(&mut self) -> Option<(rtrb::Consumer<f32>, anyhow::Result<Take>)> {
        if !self.handle.as_ref()?.is_finished() {
            return None;
        }
        let (queue, result) = self.handle.take()?.join().ok()?;
        Some((queue, result.map(|(path, frames)| Take { path, frames })))
    }
}

fn write(
    queue: &mut rtrb::Consumer<f32>,
    path: &Path,
    sample_rate: u32,
    stop: &AtomicBool,
) -> anyhow::Result<u64> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut wav = hound::WavWriter::create(path, spec)?;
    let mut samples = 0u64;
    loop {
        let stopping = stop.load(Ordering::Relaxed);
        let available = queue.slots() & !1; // whole stereo frames only
        if available > 0 {
            let chunk = queue.read_chunk(available)?;
            let (a, b) = chunk.as_slices();
            for &s in a.iter().chain(b) {
                wav.write_sample(s)?;
            }
            samples += available as u64;
            chunk.commit_all();
        } else if stopping {
            break;
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    wav.finalize()?;
    Ok(samples / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_what_was_queued() {
        let path = std::env::temp_dir().join(format!("isai-take-{}.wav", std::process::id()));
        let (mut tx, rx) = rtrb::RingBuffer::new(1000);
        let mut rec = Recorder::start(rx, &path, 48_000);
        for i in 0..200 {
            tx.push(i as f32 / 200.0).unwrap();
        }
        rec.stop();
        let take = loop {
            if let Some((_, result)) = rec.finished() {
                break result.unwrap();
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(take.frames, 100);
        let reader = hound::WavReader::open(&take.path).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.len(), 200);
        let _ = std::fs::remove_file(&take.path);
    }
}
