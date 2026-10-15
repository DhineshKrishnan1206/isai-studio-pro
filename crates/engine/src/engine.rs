use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering::Relaxed;

use anyhow::{Context as _, anyhow};
use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};
use cpal::{
    BufferSize, Device, FromSample, SampleFormat, SizedSample, StreamConfig, SupportedBufferSize,
};

use crate::arrangement::Arrangement;
use crate::instruments::{LiveNote, PlayedNote};
use crate::processor::{Command, Garbage, NO_RECORD_START, Processor, Shared};
use crate::time::Meter;

#[derive(Clone, Debug, Default)]
pub struct EngineConfig {
    /// `None` uses the system's default output.
    pub device_id: Option<String>,
    /// Frames per audio callback. Bigger is safer on slow PCs but adds delay.
    /// `None` lets the system choose.
    pub buffer_size: Option<u32>,
}

/// What the device actually gave us, which can differ from what was asked for.
#[derive(Clone, Debug)]
pub struct StreamInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: Option<u32>,
}

pub struct Engine {
    _stream: cpal::Stream,
    /// Recorded audio (stereo interleaved) for the disk writer. Taken while a
    /// take is being written, then given back.
    record: Option<rtrb::Consumer<f32>>,
    commands: rtrb::Producer<Command>,
    garbage: rtrb::Consumer<Garbage>,
    shared: Arc<Shared>,
    info: StreamInfo,
    /// Live notes in; shared so the MIDI input thread and the UI can both send.
    live: Arc<Mutex<rtrb::Producer<LiveNote>>>,
    /// Live notes with their timeline positions, while capturing.
    played: rtrb::Consumer<PlayedNote>,
}

impl Engine {
    pub fn start(config: &EngineConfig) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let saved = config
            .device_id
            .as_deref()
            .and_then(|id| id.parse().ok())
            .and_then(|id| host.device_by_id(&id));
        // A saved device that is unplugged falls back to the default instead of failing.
        let device = saved
            .or_else(|| host.default_output_device())
            .ok_or_else(|| anyhow!("no audio output device found"))?;
        let device_name = device
            .description()
            .map(|d| d.name().to_owned())
            .unwrap_or_else(|_| "Unknown device".into());

        let supported = device
            .default_output_config()
            .context("the audio device did not report a usable format")?;
        let format = supported.sample_format();
        let mut stream_config = supported.config();
        let wanted = config.buffer_size.map(|n| match supported.buffer_size() {
            SupportedBufferSize::Range { min, max } => n.clamp(*min, *max),
            SupportedBufferSize::Unknown => n,
        });

        // Some drivers refuse a fixed buffer size, so fall back to the default one.
        let (running, buffer_size) = match wanted {
            Some(n) => {
                stream_config.buffer_size = BufferSize::Fixed(n);
                match Running::build(&device, &stream_config, format) {
                    Ok(running) => (running, Some(n)),
                    Err(err) => {
                        eprintln!("fixed buffer of {n} frames refused ({err}), using default");
                        stream_config.buffer_size = BufferSize::Default;
                        (Running::build(&device, &stream_config, format)?, None)
                    }
                }
            }
            None => (Running::build(&device, &stream_config, format)?, None),
        };
        let Running { stream, commands, garbage, shared, record, live, played } = running;
        stream.play().context("could not start the audio stream")?;

        Ok(Self {
            _stream: stream,
            live: Arc::new(Mutex::new(live)),
            played,
            record: Some(record),
            commands,
            garbage,
            shared,
            info: StreamInfo {
                device_name,
                sample_rate: stream_config.sample_rate,
                channels: stream_config.channels,
                buffer_size,
            },
        })
    }

    pub fn info(&self) -> &StreamInfo {
        &self.info
    }

    /// Starts playback, counting in `count_in_bars` bars on the metronome first.
    pub fn play(&mut self, count_in_bars: u32) {
        self.send(Command::Play { count_in_bars });
    }
    pub fn stop(&mut self) {
        self.send(Command::Stop);
    }
    pub fn seek(&mut self, frame: u64) {
        self.send(Command::Seek(frame));
    }
    /// Loops playback between two timeline positions, in samples.
    pub fn set_cycle(&mut self, on: bool, start: u64, end: u64) {
        self.send(Command::SetCycle { on, start, end });
    }
    pub fn set_meter(&mut self, meter: Meter) {
        self.send(Command::SetMeter(meter));
    }
    pub fn set_metronome(&mut self, on: bool, volume: f32) {
        self.send(Command::SetMetronome { on, volume });
    }
    pub fn set_volume(&mut self, gain: f32) {
        self.send(Command::SetVolume(gain));
    }
    /// Replaces what plays. Cheap to call after every edit.
    pub fn set_arrangement(&mut self, arrangement: Arc<Arrangement>) {
        // Free arrangements the audio thread has finished with first, so the
        // return queue never fills even when nothing else is collecting.
        self.collect_garbage();
        self.send(Command::SetArrangement(arrangement));
    }
    /// Connects live input (from `crate::input`), or disconnects it with `None`.
    pub fn set_input(&mut self, input: Option<rtrb::Consumer<f32>>) {
        self.collect_garbage();
        self.send(Command::SetInput(input));
    }
    pub fn set_recording(&mut self, on: bool) {
        self.send(Command::SetRecording(on));
    }
    pub fn set_monitor(&mut self, on: bool) {
        self.send(Command::SetMonitor(on));
    }
    /// Timeline position (samples) of the first recorded sample of this take.
    pub fn record_start(&self) -> Option<u64> {
        let s = self.shared.record_start.load(Relaxed);
        (s != NO_RECORD_START).then_some(s)
    }
    /// Plays a sound by itself (the browser's preview); `None` stops it.
    /// With `sync` while playing, it waits for the next bar.
    pub fn preview(&mut self, clip: Option<Arc<crate::clip::Clip>>, sync: bool) {
        self.send(Command::Preview { clip, sync });
    }
    pub fn previewing(&self) -> bool {
        self.shared.previewing.load(Relaxed)
    }
    /// The most recent live input (mono), oldest first, up to
    /// `processor::INPUT_SCOPE` samples, for the tuner.
    pub fn input_scope(&self, out: &mut Vec<f32>) {
        use crate::processor::INPUT_SCOPE;
        out.clear();
        let end = self.shared.input_scope_pos.load(Relaxed);
        let n = end.min(INPUT_SCOPE);
        out.extend((end - n..end).map(|i| f32::from_bits(self.shared.input_scope[i % INPUT_SCOPE].load(Relaxed))));
    }
    /// Input level since the last call, (left, right).
    pub fn take_input_peaks(&self) -> (f32, f32) {
        let take = |p: &std::sync::atomic::AtomicU32| f32::from_bits(p.swap(0, Relaxed));
        (take(&self.shared.input_peak_l), take(&self.shared.input_peak_r))
    }
    pub fn record_dropped(&self) -> u64 {
        self.shared.record_dropped.load(Relaxed)
    }
    /// Hands the record queue to the disk writer for the length of a take.
    pub fn take_record_queue(&mut self) -> Option<rtrb::Consumer<f32>> {
        self.record.take()
    }
    pub fn return_record_queue(&mut self, queue: rtrb::Consumer<f32>) {
        self.record = Some(queue);
    }

    /// Gives the audio thread the running state for effect `id` (build it
    /// with `Effect::new` here, off the audio thread).
    /// `meters` (optional) receive its live levels for an editor.
    pub fn add_effect(&mut self, id: u64, effect: Box<crate::effects::Effect>, meters: Option<Arc<crate::effects::Meters>>) {
        self.collect_garbage();
        self.send(Command::AddEffect(id, effect, meters));
    }
    pub fn remove_effect(&mut self, id: u64) {
        self.send(Command::RemoveEffect(id));
    }

    /// Gives the audio thread the running state for track instrument `id`.
    pub fn add_instrument(&mut self, id: u64, instrument: Box<crate::instruments::Instrument>) {
        self.collect_garbage();
        self.send(Command::AddInstrument(id, instrument));
    }
    pub fn remove_instrument(&mut self, id: u64) {
        self.send(Command::RemoveInstrument(id));
    }
    /// Which track (arrangement index) live notes play on.
    pub fn set_live_track(&mut self, track: Option<usize>) {
        self.send(Command::SetLiveTrack(track));
    }
    /// Starts or stops reporting live notes with their timeline positions
    /// (while playing), for recording MIDI.
    pub fn set_capture_notes(&mut self, on: bool) {
        self.send(Command::SetCaptureNotes(on));
    }
    /// Live notes captured since the last call.
    pub fn take_played_notes(&mut self, out: &mut Vec<PlayedNote>) {
        while let Ok(n) = self.played.pop() {
            out.push(n);
        }
    }
    /// The selected instrument's keys as bit masks (bit `n` = MIDI note `n`):
    /// held now, and struck since the last call (so quick notes still show).
    /// Also returns how hard each key was last played (1..=127, 0 = never).
    pub fn take_keys(&self) -> (u128, u128, [u8; 128]) {
        let s = &self.shared;
        let held = u128::from(s.keys_held[0].load(Relaxed)) | u128::from(s.keys_held[1].load(Relaxed)) << 64;
        let struck = u128::from(s.keys_struck[0].swap(0, Relaxed)) | u128::from(s.keys_struck[1].swap(0, Relaxed)) << 64;
        let mut velocities = [0u8; 128];
        for (v, a) in velocities.iter_mut().zip(&s.key_velocities) {
            *v = a.load(Relaxed);
        }
        (held, struck, velocities)
    }
    /// Where to send live notes from any thread (MIDI input, the UI).
    pub fn live_notes(&self) -> Arc<Mutex<rtrb::Producer<LiveNote>>> {
        self.live.clone()
    }

    pub fn test_tone(&mut self) {
        self.send(Command::TestTone);
    }

    pub fn position(&self) -> u64 {
        self.shared.position.load(Relaxed)
    }
    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Relaxed)
    }
    pub fn counting_in(&self) -> bool {
        self.shared.counting_in.load(Relaxed)
    }
    /// Loudest (left, right) sample since the last call.
    pub fn take_peaks(&self) -> (f32, f32) {
        let take = |p: &std::sync::atomic::AtomicU32| f32::from_bits(p.swap(0, Relaxed));
        (take(&self.shared.peak_l), take(&self.shared.peak_r))
    }
    /// Post-fader (left, right) levels since the last call: the first `count`
    /// entries of tracks then buses, in arrangement order.
    pub fn take_track_peaks(&self, tracks: usize, out: &mut Vec<(f32, f32)>) {
        let take = |p: &std::sync::atomic::AtomicU32| f32::from_bits(p.swap(0, Relaxed));
        out.clear();
        out.extend(self.shared.track_peaks.chunks_exact(2).take(tracks).map(|p| (take(&p[0]), take(&p[1]))));
    }
    /// Share of the real-time budget the audio thread is using, where 1.0 means 100%.
    pub fn load(&self) -> f32 {
        f32::from_bits(self.shared.load.load(Relaxed))
    }
    /// How many times audio has been late since the engine started.
    pub fn overruns(&self) -> u64 {
        self.shared.overruns.load(Relaxed)
    }

    /// Frees arrangements the audio thread has finished with. Call this regularly from the UI thread.
    pub fn collect_garbage(&mut self) {
        while self.garbage.pop().is_ok() {}
    }

    fn send(&mut self, cmd: Command) {
        if self.commands.push(cmd).is_err() {
            eprintln!("engine command queue full; command dropped");
        }
    }
}

/// A started stream plus the UI-side ends of its queues.
struct Running {
    stream: cpal::Stream,
    record: rtrb::Consumer<f32>,
    commands: rtrb::Producer<Command>,
    garbage: rtrb::Consumer<Garbage>,
    shared: Arc<Shared>,
    live: rtrb::Producer<LiveNote>,
    played: rtrb::Consumer<PlayedNote>,
}

impl Running {
    fn build(device: &Device, config: &StreamConfig, format: SampleFormat) -> anyhow::Result<Self> {
        match format {
            SampleFormat::F32 => Self::build_typed::<f32>(device, config),
            SampleFormat::F64 => Self::build_typed::<f64>(device, config),
            SampleFormat::I16 => Self::build_typed::<i16>(device, config),
            SampleFormat::I32 => Self::build_typed::<i32>(device, config),
            SampleFormat::U16 => Self::build_typed::<u16>(device, config),
            SampleFormat::I8 => Self::build_typed::<i8>(device, config),
            SampleFormat::U8 => Self::build_typed::<u8>(device, config),
            other => Err(anyhow!("unsupported sample format {other}")),
        }
    }

    fn build_typed<T: SizedSample + FromSample<f32> + 'static>(
        device: &Device,
        config: &StreamConfig,
    ) -> anyhow::Result<Self> {
        let shared = Arc::new(Shared::default());
        let (commands, command_rx) = rtrb::RingBuffer::new(1024);
        let (garbage_tx, garbage) = rtrb::RingBuffer::new(1024);
        // Twenty seconds of stereo audio: plenty of slack for the disk writer.
        let (record_tx, record) = rtrb::RingBuffer::new(config.sample_rate as usize * 2 * 20);
        let (live_tx, live_rx) = rtrb::RingBuffer::new(512);
        let (played_tx, played) = rtrb::RingBuffer::new(4096);
        let mut processor = Processor::new(
            command_rx,
            garbage_tx,
            shared.clone(),
            config.sample_rate,
            Arc::new(Arrangement::default()),
            record_tx,
            live_rx,
            played_tx,
        );
        let channels = usize::from(config.channels);
        let err_shared = shared.clone();

        let stream = device.build_output_stream(
            *config,
            move |data: &mut [T], _| processor.process(data, channels),
            move |err| {
                if err.kind() == cpal::ErrorKind::Xrun {
                    err_shared.overruns.fetch_add(1, Relaxed);
                } else {
                    eprintln!("audio stream error: {err}");
                }
            },
            None,
        )?;
        Ok(Self { stream, record, commands, garbage, shared, live: live_tx, played })
    }
}
