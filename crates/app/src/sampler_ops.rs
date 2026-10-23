//! The controller's sampler side: loading a sound into a sampler track and
//! slicing it.

use std::pin::Pin;

use cxx_qt_lib::{QString, QUrl};
use isai_engine::instruments::sampler::FIRST_SLICE_KEY;
use isai_project::Note;

use crate::controller::qobject::AppController;
use crate::project_ops::{id, url_path};

impl AppController {
    pub fn load_sample(mut self: Pin<&mut Self>, track: i32, url: &QUrl) {
        let Some(path) = url_path(url) else { return };
        let file = path.display().to_string();
        self.as_mut().st().pool.request(&path);
        self.as_mut().edit("Load Sample", true, |p| {
            p.set_instrument_sample(id(track), Some(&file))
        });
        self.update_busy();
    }

    /// The sampler's sound as mono samples, with its sample rate, once loaded.
    fn sample_mono(mut self: Pin<&mut Self>, track: i32) -> Option<(Vec<f32>, u32)> {
        let st = self.as_mut().st();
        let session = st.session.as_ref()?;
        let file = session
            .project
            .track(id(track))?
            .instrument
            .as_ref()?
            .sample
            .as_ref()?;
        let clip = st.pool.ready(&session.resolve(file))?;
        Some((
            clip.left
                .iter()
                .zip(&clip.right)
                .map(|(l, r)| (l + r) * 0.5)
                .collect(),
            clip.sample_rate,
        ))
    }

    pub fn slice_sample(mut self: Pin<&mut Self>, track: i32, how: &QString, count: i32) {
        let Some((mono, rate)) = self.as_mut().sample_mono(track) else {
            self.show_problem("Load a sound into the sampler first (it may still be loading).");
            return;
        };
        let count = count.clamp(1, 64) as usize;
        let starts = if how.to_string() == "beats" {
            crate::slicer::onsets(&mono, rate, count)
        } else {
            crate::slicer::even(mono.len(), count)
        };
        let secs: Vec<f64> = starts.iter().map(|&s| s as f64 / f64::from(rate)).collect();
        self.edit("Slice Sample", true, |p| p.set_slices(id(track), &secs));
    }

    pub fn set_slices(self: Pin<&mut Self>, track: i32, slices: &QString, commit: bool) {
        let secs: Vec<f64> = serde_json::from_str(&slices.to_string()).unwrap_or_default();
        self.edit("Move Slice", commit, |p| p.set_slices(id(track), &secs));
    }

    pub fn slices_to_pattern(mut self: Pin<&mut Self>, track: i32) -> i32 {
        let at = *self.position_beats();
        let Some((mono, rate)) = self.as_mut().sample_mono(track) else {
            self.show_problem("Load and slice a sound first.");
            return -1;
        };
        let total = mono.len() as f64 / f64::from(rate);
        let mut made = None;
        self.as_mut().edit("Slices to Pattern", true, |p| {
            let Some(slices) = p
                .track(id(track))
                .and_then(|t| t.instrument.as_ref())
                .map(|i| i.slices.clone())
            else {
                return false;
            };
            if slices.is_empty() {
                return false;
            }
            let meter = p.meter();
            let bar = p.bar_beats();
            let length = (meter.secs_to_beats(total) / bar).ceil().max(1.0) * bar;
            let start = (at / bar).floor() * bar;
            let Some(region) = p.add_midi_region(id(track), start, length) else {
                return false;
            };
            let notes: Vec<Note> = slices
                .iter()
                .enumerate()
                .map(|(i, &s)| {
                    let end = slices.get(i + 1).copied().unwrap_or(total);
                    Note {
                        start: meter.secs_to_beats(s),
                        length: meter.secs_to_beats(end - s).max(1.0 / 64.0),
                        key: (usize::from(FIRST_SLICE_KEY) + i).min(127) as u8,
                        velocity: 100,
                    }
                })
                .collect();
            if let Some(r) = p
                .tracks
                .iter_mut()
                .flat_map(|t| t.regions.iter_mut())
                .find(|r| r.id == region)
            {
                r.start = start;
                r.name = "Slices".into();
            }
            made = p.add_notes(region, &notes).map(|_| region);
            made.is_some()
        });
        match made {
            Some(r) => {
                self.set_selection(vec![r]);
                r as i32
            }
            None => -1,
        }
    }
}
