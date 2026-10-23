//! Loading audio files into memory, ready for playback.
//!
//! Decoding and resampling happen here, off the audio thread. The audio thread
//! only ever reads a finished [`Clip`].

use std::fs::File;
use std::path::Path;

use anyhow::{Context as _, anyhow};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler as _};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// A stereo audio clip, already converted to the engine's sample rate.
pub struct Clip {
    pub name: String,
    pub sample_rate: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

impl Clip {
    pub fn frames(&self) -> usize {
        self.left.len()
    }

    pub fn duration_secs(&self) -> f64 {
        self.frames() as f64 / f64::from(self.sample_rate)
    }

    /// Decodes any supported file (WAV, FLAC, MP3, OGG, AAC, ...) and resamples
    /// it to `target_rate` so it plays at the right speed and pitch.
    pub fn load(path: &Path, target_rate: u32) -> anyhow::Result<Self> {
        let (interleaved, channels, source_rate) = decode(path)?;
        let frames = interleaved.len() / channels;
        if frames == 0 {
            return Err(anyhow!("the file contains no audio"));
        }

        let (interleaved, frames) = if source_rate == target_rate {
            (interleaved, frames)
        } else {
            resample(&interleaved, channels, frames, source_rate, target_rate)?
        };

        // Mono files play on both sides; extra channels beyond stereo are dropped for now.
        let mut left = Vec::with_capacity(frames);
        let mut right = Vec::with_capacity(frames);
        for frame in interleaved.chunks_exact(channels) {
            left.push(frame[0]);
            right.push(if channels > 1 { frame[1] } else { frame[0] });
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Self {
            name,
            sample_rate: target_rate,
            left,
            right,
        })
    }
}

fn decode(path: &Path) -> anyhow::Result<(Vec<f32>, usize, u32)> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let source = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .context("unsupported or damaged audio file")?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| anyhow!("the file has no audio track"))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| anyhow!("the file has no audio track"))?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .context("unsupported audio codec")?;

    let mut samples = Vec::new();
    let mut packet_samples = Vec::new();
    let mut spec = None;
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            // Some valid files (e.g. from some FLAC encoders) report the end of
            // the stream this way; once audio has been decoded, it just means "done".
            Err(DecodeError::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof && !samples.is_empty() =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        };
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => {
                spec.get_or_insert_with(|| (buf.spec().channels().count(), buf.spec().rate()));
                packet_samples.resize(buf.samples_interleaved(), 0.0f32);
                buf.copy_to_slice_interleaved(&mut packet_samples);
                samples.extend_from_slice(&packet_samples);
            }
            // A corrupt packet is skipped rather than failing the whole file.
            Err(DecodeError::DecodeError(_)) => continue,
            Err(err) => return Err(err.into()),
        }
    }

    let (channels, rate) = spec.ok_or_else(|| anyhow!("the file contains no audio"))?;
    if channels == 0 {
        return Err(anyhow!("the file has no audio channels"));
    }
    Ok((samples, channels, rate))
}

fn resample(
    interleaved: &[f32],
    channels: usize,
    frames: usize,
    from: u32,
    to: u32,
) -> anyhow::Result<(Vec<f32>, usize)> {
    let mut resampler =
        Fft::<f32>::new(from as usize, to as usize, 1024, channels, FixedSync::Both)?;
    let input = InterleavedSlice::new(interleaved, channels, frames)?;
    let output = resampler.process_all(&input, frames, None)?;
    let data = output.take_data();
    let out_frames = data.len() / channels;
    Ok((data, out_frames))
}

/// A min/max summary of a clip's waveform, computed once so drawing it is cheap.
pub struct Peaks {
    pub frames_per_peak: usize,
    /// (min, max) of both channels mixed, one entry per `frames_per_peak` frames.
    pub values: Vec<(f32, f32)>,
}

impl Peaks {
    pub const FRAMES_PER_PEAK: usize = 256;

    pub fn compute(clip: &Clip) -> Self {
        let values = clip
            .left
            .chunks(Self::FRAMES_PER_PEAK)
            .zip(clip.right.chunks(Self::FRAMES_PER_PEAK))
            .map(|(l, r)| {
                l.iter().zip(r).fold((0.0f32, 0.0f32), |(lo, hi), (a, b)| {
                    let s = (a + b) * 0.5;
                    (lo.min(s), hi.max(s))
                })
            })
            .collect();
        Self {
            frames_per_peak: Self::FRAMES_PER_PEAK,
            values,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampling_keeps_duration() {
        let frames = 44_100;
        let input: Vec<f32> = (0..frames * 2)
            .map(|i| ((i / 2) as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin())
            .collect();
        let (out, out_frames) = resample(&input, 2, frames, 44_100, 48_000).unwrap();
        assert_eq!(out.len(), out_frames * 2);
        assert!(
            (out_frames as i64 - 48_000).abs() <= 1,
            "got {out_frames} frames"
        );
    }

    #[test]
    fn peaks_summarise_min_and_max() {
        let clip = Clip {
            name: String::new(),
            sample_rate: 48_000,
            left: [1.0, -1.0].repeat(Peaks::FRAMES_PER_PEAK),
            right: [1.0, -1.0].repeat(Peaks::FRAMES_PER_PEAK),
        };
        let peaks = Peaks::compute(&clip);
        assert_eq!(peaks.values, vec![(-1.0, 1.0), (-1.0, 1.0)]);
    }
}
