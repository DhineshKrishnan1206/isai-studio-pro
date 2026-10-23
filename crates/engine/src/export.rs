//! Exporting (bouncing) an arrangement to an audio file, faster than real time.
//!
//! Mixing uses the same rules as playback (track gain and pan law, mute/solo,
//! fades and crossfades), so an export sounds exactly like what you hear,
//! minus the metronome.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

use anyhow::{Context as _, bail};

use crate::arrangement::Arrangement;
use crate::effects::EffectBank;
use crate::instruments::InstrumentBank;
use crate::mixer::{BLOCK, Block, Mixer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// 16-bit WAV, dithered.
    Wav16,
    Wav24,
    /// 32-bit float WAV: no quality loss, and no clipping in the file.
    Wav32Float,
    /// 16-bit FLAC (lossless, about half the size of WAV), dithered.
    Flac16,
    Flac24,
}

impl Format {
    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "wav16" => Format::Wav16,
            "wav24" => Format::Wav24,
            "wav32f" => Format::Wav32Float,
            "flac16" => Format::Flac16,
            "flac24" => Format::Flac24,
            _ => return None,
        })
    }

    pub fn extension(self) -> &'static str {
        match self {
            Format::Flac16 | Format::Flac24 => "flac",
            _ => "wav",
        }
    }

    fn integer_bits(self) -> Option<u32> {
        match self {
            Format::Wav16 | Format::Flac16 => Some(16),
            Format::Wav24 | Format::Flac24 => Some(24),
            Format::Wav32Float => None,
        }
    }
}

pub struct ExportOptions {
    pub format: Format,
    pub sample_rate: u32,
    /// Timeline range in samples, start inclusive, end exclusive.
    pub start: u64,
    pub end: u64,
    /// Master volume.
    pub gain: f32,
    /// Scale so the loudest peak sits at -1 dBFS.
    pub normalize: bool,
}

#[derive(Debug)]
pub struct ExportReport {
    pub path: PathBuf,
    /// Loudest sample before normalizing, in dBFS.
    pub peak_db: f32,
    /// True when the file had to be clipped (peaks above 0 dBFS in an integer format).
    pub clipped: bool,
}

/// Progress (0..1, as f32 bits) and a cancel flag, shared with the UI.
#[derive(Default)]
pub struct ExportControl {
    progress: AtomicU32,
    cancel: AtomicBool,
}

impl ExportControl {
    pub fn progress(&self) -> f32 {
        f32::from_bits(self.progress.load(Relaxed))
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Relaxed);
    }
    fn set(&self, p: f32) {
        self.progress.store(p.to_bits(), Relaxed);
    }
    /// True once `cancel` has been called.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled()
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Relaxed)
    }
}

/// Mixes `arrangement` over `[start, end)` into stereo interleaved samples,
/// through the same mixer playback uses.
pub fn render(
    arrangement: &Arrangement,
    sample_rate: u32,
    start: u64,
    end: u64,
    gain: f32,
    control: &ExportControl,
) -> anyhow::Result<Vec<f32>> {
    let frames = end.saturating_sub(start) as usize;
    let mut out = Vec::with_capacity(frames * 2);
    let mut mixer = Mixer::new();
    mixer.snap_gains(arrangement);
    // Fresh effects, so an export never shares state with playback.
    let mut bank = EffectBank::for_inserts(arrangement.inserts(), sample_rate);
    let mut instruments = InstrumentBank::for_instruments(arrangement.instruments());
    let (mut positions, mut l, mut r) = (
        Vec::with_capacity(BLOCK),
        vec![0.0f32; BLOCK],
        vec![0.0f32; BLOCK],
    );
    let mut pos = start;
    while pos < end {
        if control.cancelled() {
            bail!("cancelled");
        }
        control.set(0.9 * (pos - start) as f32 / frames.max(1) as f32);
        let n = ((end - pos) as usize).min(BLOCK);
        positions.clear();
        positions.extend(pos..pos + n as u64);
        let block = Block {
            positions: &positions,
            fades: &[],
            live: &[],
            live_track: None,
        };
        mixer.mix(
            arrangement,
            &mut bank,
            &mut instruments,
            sample_rate as f32,
            &block,
            &mut l[..n],
            &mut r[..n],
        );
        for k in 0..n {
            out.push(l[k] * gain);
            out.push(r[k] * gain);
        }
        pos += n as u64;
    }
    Ok(out)
}

/// Renders and writes `arrangement` to `path` in the chosen format.
pub fn export(
    arrangement: &Arrangement,
    opts: &ExportOptions,
    path: &Path,
    control: &ExportControl,
) -> anyhow::Result<ExportReport> {
    if opts.end <= opts.start {
        bail!("there's nothing to export in that range");
    }
    let mut samples = render(
        arrangement,
        opts.sample_rate,
        opts.start,
        opts.end,
        opts.gain,
        control,
    )?;
    let peak = samples.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    let peak_db = 20.0 * peak.max(1e-9).log10();
    if opts.normalize && peak > 0.0 {
        let target = 10f32.powf(-1.0 / 20.0);
        let scale = target / peak;
        samples.iter_mut().for_each(|s| *s *= scale);
    }
    let clipped = opts.format.integer_bits().is_some() && samples.iter().any(|s| s.abs() > 1.0);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    match opts.format {
        Format::Wav16 | Format::Wav24 | Format::Wav32Float => write_wav(&samples, opts, path)?,
        Format::Flac16 | Format::Flac24 => write_flac(&samples, opts, path)?,
    }
    control.set(1.0);
    Ok(ExportReport {
        path: path.to_path_buf(),
        peak_db,
        clipped,
    })
}

/// Float to integer, with TPDF dither for 16-bit so quiet fades stay smooth.
fn to_int(samples: &[f32], bits: u32) -> Vec<i32> {
    let max = ((1i64 << (bits - 1)) - 1) as f32;
    let mut rng = 0x2545_F491_4F6C_DD1Du64;
    let mut noise = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        (rng >> 40) as f32 / (1u64 << 24) as f32
    };
    samples
        .iter()
        .map(|&s| {
            let dither = if bits <= 16 { noise() - noise() } else { 0.0 };
            (s * max + dither).round().clamp(-max - 1.0, max) as i32
        })
        .collect()
}

fn write_wav(samples: &[f32], opts: &ExportOptions, path: &Path) -> anyhow::Result<()> {
    let (bits, format) = match opts.format.integer_bits() {
        Some(b) => (b as u16, hound::SampleFormat::Int),
        None => (32, hound::SampleFormat::Float),
    };
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: opts.sample_rate,
        bits_per_sample: bits,
        sample_format: format,
    };
    let mut wav = hound::WavWriter::create(path, spec)
        .with_context(|| format!("couldn't create {}", path.display()))?;
    match opts.format.integer_bits() {
        Some(b) => {
            for s in to_int(samples, b) {
                wav.write_sample(s)?;
            }
        }
        None => {
            for &s in samples {
                wav.write_sample(s)?;
            }
        }
    }
    wav.finalize()?;
    Ok(())
}

fn write_flac(samples: &[f32], opts: &ExportOptions, path: &Path) -> anyhow::Result<()> {
    use flacenc::component::BitRepr as _;
    use flacenc::error::Verify as _;

    let bits = opts.format.integer_bits().unwrap_or(24);
    let ints = to_int(samples, bits);
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|e| anyhow::anyhow!("FLAC settings: {e:?}"))?;
    let source = flacenc::source::MemSource::from_samples(
        &ints,
        2,
        bits as usize,
        opts.sample_rate as usize,
    );
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| anyhow::anyhow!("FLAC encoding failed: {e:?}"))?;
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| anyhow::anyhow!("FLAC encoding failed: {e:?}"))?;
    let mut bytes = sink.as_slice().to_vec();
    fix_min_block_size(&mut bytes);
    std::fs::write(path, &bytes).with_context(|| format!("couldn't write {}", path.display()))?;
    Ok(())
}

/// The encoder records the short final block's length as the stream's minimum
/// block size. The FLAC spec says the last block doesn't count, and strict
/// readers then mistake the stream for a variable-block-size one and can't
/// open it. Every block but the last has the maximum size, so set min = max.
fn fix_min_block_size(bytes: &mut [u8]) {
    // "fLaC", then the STREAMINFO block header (4 bytes), then min (2) and max (2) block size.
    if bytes.len() >= 12 && &bytes[..4] == b"fLaC" && bytes[4] & 0x7F == 0 {
        let (min, max) = (8..10, 10..12);
        let max_bytes = [bytes[max.start], bytes[max.start + 1]];
        bytes[min].copy_from_slice(&max_bytes);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::arrangement::{PlayRegion, PlayTrack};
    use crate::clip::Clip;

    fn arrangement(level: f32) -> Arrangement {
        let audio = Arc::new(Clip {
            name: String::new(),
            sample_rate: 1000,
            left: vec![level; 1000],
            right: vec![level; 1000],
        });
        Arrangement {
            tracks: vec![PlayTrack {
                sends: vec![],
                inserts: vec![],
                automation: vec![],
                instrument: None,
                notes: vec![],
                gain: 1.0,
                pan: 0.0,
                audible: true,
                regions: vec![PlayRegion {
                    audio,
                    start: 100,
                    offset: 0,
                    len: 1000,
                    fade_in: 0,
                    fade_out: 0,
                    gain: 1.0,
                    reverse: false,
                }],
            }],
            ..Default::default()
        }
    }

    fn opts(format: Format) -> ExportOptions {
        ExportOptions {
            format,
            sample_rate: 1000,
            start: 0,
            end: 1200,
            gain: 1.0,
            normalize: false,
        }
    }

    #[test]
    fn renders_the_range_like_playback() {
        let a = arrangement(0.5);
        let out = render(&a, 1000, 0, 1200, 1.0, &ExportControl::default()).unwrap();
        assert_eq!(out.len(), 2400);
        assert_eq!(out[2 * 50], 0.0, "silence before the region");
        assert!((out[2 * 500] - 0.5).abs() < 1e-6);
        // A range starting mid-region begins with the region's audio.
        let out = render(&a, 1000, 600, 700, 1.0, &ExportControl::default()).unwrap();
        assert!((out[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn writes_wav_and_flac_that_read_back() {
        let dir = std::env::temp_dir().join(format!("isai-export-{}", std::process::id()));
        for format in [
            Format::Wav16,
            Format::Wav24,
            Format::Wav32Float,
            Format::Flac16,
            Format::Flac24,
        ] {
            let path = dir.join(format!("{format:?}.{}", format.extension()));
            let report = export(
                &arrangement(0.5),
                &opts(format),
                &path,
                &ExportControl::default(),
            )
            .unwrap();
            assert!(!report.clipped);
            assert!((report.peak_db - (-6.02)).abs() < 0.1);
            let clip = Clip::load(&path, 1000).unwrap_or_else(|e| panic!("{format:?}: {e:#}"));
            assert_eq!(clip.frames(), 1200, "{format:?}");
            assert!(
                (clip.left[500] - 0.5).abs() < 0.001,
                "{format:?}: {}",
                clip.left[500]
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn normalizing_raises_the_peak_to_minus_one_db() {
        let path = std::env::temp_dir().join(format!("isai-norm-{}.wav", std::process::id()));
        let mut o = opts(Format::Wav32Float);
        o.normalize = true;
        export(&arrangement(0.25), &o, &path, &ExportControl::default()).unwrap();
        let clip = Clip::load(&path, 1000).unwrap();
        assert!((clip.left[500] - 0.891).abs() < 0.001);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn loud_mixes_report_clipping_in_integer_formats() {
        let path = std::env::temp_dir().join(format!("isai-clip-{}.wav", std::process::id()));
        let report = export(
            &arrangement(1.5),
            &opts(Format::Wav16),
            &path,
            &ExportControl::default(),
        )
        .unwrap();
        assert!(report.clipped);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn can_be_cancelled() {
        let control = ExportControl::default();
        control.cancel();
        assert!(render(&arrangement(0.5), 1000, 0, 1200, 1.0, &control).is_err());
    }

    #[test]
    fn exports_run_the_master_effects() {
        use crate::effects::{EffectKind, PlayInsert};
        let mut a = arrangement(1.5);
        a.master = vec![PlayInsert {
            plugin: None,
            id: 1,
            kind: EffectKind::Limiter,
            enabled: true,
            params: EffectKind::Limiter.defaults(),
        }];
        let out = render(&a, 48_000, 0, 1200, 1.0, &ExportControl::default()).unwrap();
        let peak = out.iter().fold(0.0f32, |p, s| p.max(s.abs()));
        assert!(
            peak <= 10f32.powf(-1.0 / 20.0) + 1e-5 && peak > 0.5,
            "{peak}"
        );
    }

    #[test]
    fn exports_play_instrument_tracks() {
        use crate::instruments::{InstrumentKind, PlayInstrument, PlayNote};
        let mut a = arrangement(0.0);
        a.tracks[0].regions.clear();
        a.tracks[0].instrument = Some(PlayInstrument::new(
            9,
            InstrumentKind::Synth,
            InstrumentKind::Synth.defaults(),
        ));
        a.tracks[0].notes = vec![PlayNote {
            start: 1000,
            end: 20_000,
            key: 60,
            velocity: 100,
        }];
        let out = render(&a, 48_000, 0, 24_000, 1.0, &ExportControl::default()).unwrap();
        let level = |from: usize, to: usize| {
            out[from * 2..to * 2]
                .iter()
                .fold(0.0f32, |p, s| p.max(s.abs()))
        };
        assert_eq!(level(0, 1000), 0.0);
        assert!(level(4000, 18_000) > 0.01);
    }
}
