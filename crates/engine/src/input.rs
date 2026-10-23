//! Live audio input from a microphone or audio interface.
//!
//! The input stream converts what it captures to stereo at the output's
//! sample rate and queues it for the output callback, which records and
//! monitors it (see `processor.rs`). Running both through the output keeps
//! recordings aligned with the timeline.

use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};
use cpal::{Device, FromSample, SampleFormat, SizedSample, StreamConfig};

use anyhow::{Context as _, anyhow};

/// An input the user can pick in Settings.
#[derive(Clone, Debug, PartialEq)]
pub struct InputDevice {
    pub id: String,
    pub name: String,
    pub channels: u16,
    pub is_default: bool,
}

pub fn list_input_devices() -> Vec<InputDevice> {
    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|d| d.id().ok())
        .map(|id| id.to_string());
    let Ok(devices) = host.input_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|device| {
            let id = device.id().ok()?.to_string();
            let name = device
                .description()
                .map(|d| d.name().to_owned())
                .unwrap_or_else(|_| id.clone());
            let channels = device
                .default_input_config()
                .map(|c| c.channels())
                .unwrap_or(1);
            let is_default = default_id.as_deref() == Some(id.as_str());
            Some(InputDevice {
                id,
                name,
                channels,
                is_default,
            })
        })
        .collect()
}

/// Which input channels to record. Channels count from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputChannels {
    /// One channel, heard in both ears (a microphone or a guitar).
    Mono(u16),
    /// Two neighbouring channels as left and right.
    Stereo(u16),
}

/// A running input stream. Dropping it stops capture.
pub struct Input {
    _stream: cpal::Stream,
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Opens an input and returns it with the queue to hand to the engine
/// (`Engine::set_input`). `out_rate` is the engine's sample rate.
pub fn start_input(
    device_id: Option<&str>,
    select: InputChannels,
    out_rate: u32,
) -> anyhow::Result<(Input, rtrb::Consumer<f32>)> {
    let host = cpal::default_host();
    let saved = device_id
        .and_then(|id| id.parse().ok())
        .and_then(|id| host.device_by_id(&id));
    let device = saved
        .or_else(|| host.default_input_device())
        .ok_or_else(|| anyhow!("no microphone or audio input found"))?;
    let device_name = device
        .description()
        .map(|d| d.name().to_owned())
        .unwrap_or_else(|_| "Input".into());
    let supported = device
        .default_input_config()
        .context("the input didn't report a usable format")?;
    let config = supported.config();
    // One second of queued stereo audio.
    let (tx, rx) = rtrb::RingBuffer::new(out_rate as usize * 2);
    let state = Capture::new(select, config.channels, config.sample_rate, out_rate, tx);
    let stream = match supported.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, &config, state),
        SampleFormat::I16 => build::<i16>(&device, &config, state),
        SampleFormat::I32 => build::<i32>(&device, &config, state),
        SampleFormat::U16 => build::<u16>(&device, &config, state),
        SampleFormat::F64 => build::<f64>(&device, &config, state),
        other => Err(anyhow!("unsupported input sample format {other}")),
    }?;
    stream.play().context("couldn't start the input")?;
    Ok((
        Input {
            _stream: stream,
            device_name,
            sample_rate: config.sample_rate,
            channels: config.channels,
        },
        rx,
    ))
}

fn build<T: SizedSample + 'static>(
    device: &Device,
    config: &StreamConfig,
    mut state: Capture,
) -> anyhow::Result<cpal::Stream>
where
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    let stream = device.build_input_stream(
        *config,
        move |data: &[T], _| {
            for frame in data.chunks_exact(channels) {
                let get = |i: u16| {
                    frame
                        .get(usize::from(i))
                        .map_or(0.0, |s| <f32 as FromSample<T>>::from_sample_(*s))
                };
                let (l, r) = match state.select {
                    InputChannels::Mono(c) => {
                        let s = get(c);
                        (s, s)
                    }
                    InputChannels::Stereo(c) => (get(c), get(c + 1)),
                };
                state.push(l, r);
            }
        },
        |err| eprintln!("input stream error: {err}"),
        None,
    )?;
    Ok(stream)
}

/// Per-stream state on the input thread: channel choice and a simple
/// streaming resampler (linear), for inputs running at another rate than the
/// output, like Bluetooth headset microphones.
struct Capture {
    select: InputChannels,
    /// Input samples per output sample.
    step: f64,
    /// Position between `prev` and the next input sample, 0..1.
    frac: f64,
    prev: (f32, f32),
    tx: rtrb::Producer<f32>,
}

impl Capture {
    fn new(
        select: InputChannels,
        channels: u16,
        in_rate: u32,
        out_rate: u32,
        tx: rtrb::Producer<f32>,
    ) -> Self {
        // A stereo pair needs two channels; fall back to mono on a mono input.
        let select = match select {
            InputChannels::Stereo(c) if c + 1 >= channels => {
                InputChannels::Mono(c.min(channels.saturating_sub(1)))
            }
            InputChannels::Mono(c) if c >= channels => InputChannels::Mono(0),
            s => s,
        };
        Self {
            select,
            step: f64::from(in_rate) / f64::from(out_rate),
            frac: 0.0,
            prev: (0.0, 0.0),
            tx,
        }
    }

    #[inline]
    fn push(&mut self, l: f32, r: f32) {
        if (self.step - 1.0).abs() < 1e-9 {
            self.emit(l, r);
            return;
        }
        // Emit every output sample that falls between the previous input sample and this one.
        while self.frac < 1.0 {
            let t = self.frac as f32;
            let out = (
                self.prev.0 + (l - self.prev.0) * t,
                self.prev.1 + (r - self.prev.1) * t,
            );
            self.emit(out.0, out.1);
            self.frac += self.step;
        }
        self.frac -= 1.0;
        self.prev = (l, r);
    }

    #[inline]
    fn emit(&mut self, l: f32, r: f32) {
        if self.tx.slots() >= 2 {
            let _ = self.tx.push(l);
            let _ = self.tx.push(r);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture(in_rate: u32, out_rate: u32) -> (Capture, rtrb::Consumer<f32>) {
        let (tx, rx) = rtrb::RingBuffer::new(10_000);
        (
            Capture::new(InputChannels::Mono(0), 1, in_rate, out_rate, tx),
            rx,
        )
    }

    #[test]
    fn same_rate_passes_samples_through() {
        let (mut c, mut rx) = capture(48_000, 48_000);
        c.push(0.5, 0.5);
        assert_eq!((rx.pop().unwrap(), rx.pop().unwrap()), (0.5, 0.5));
    }

    #[test]
    fn lower_input_rates_are_upsampled_to_the_output_rate() {
        // 16 kHz in, 48 kHz out: three output samples per input sample.
        let (mut c, rx) = capture(16_000, 48_000);
        for i in 0..100 {
            c.push(i as f32, i as f32);
        }
        let frames = rx.slots() / 2;
        assert!((299..=301).contains(&frames), "got {frames}");
    }

    #[test]
    fn stereo_on_a_mono_input_falls_back_to_mono() {
        let (tx, _rx) = rtrb::RingBuffer::new(4);
        let c = Capture::new(InputChannels::Stereo(0), 1, 48_000, 48_000, tx);
        assert_eq!(c.select, InputChannels::Mono(0));
    }
}
