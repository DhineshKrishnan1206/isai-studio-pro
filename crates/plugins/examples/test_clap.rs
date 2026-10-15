//! A tiny CLAP instrument for testing Isai's CLAP hosting: a sine wave at the
//! pitch of the note held, and a state that says "isai-test-state".
//! Built with `cargo build -p isai-plugins --example test_clap`.

use std::sync::Mutex;

use clack_extensions::state::{PluginState, PluginStateImpl};
use clack_plugin::events::event_types::{NoteOffEvent, NoteOnEvent};
use clack_plugin::prelude::*;
use clack_plugin::stream::{InputStream, OutputStream};
use std::io::{Read, Write};

pub struct TestTone;

impl Plugin for TestTone {
    type AudioProcessor<'a> = Tone;
    type Shared<'a> = ();
    type MainThread<'a> = Main;

    fn declare_extensions(builder: &mut PluginExtensions<Self>, _shared: Option<&()>) {
        builder.register::<PluginState>();
    }
}

impl DefaultPluginFactory for TestTone {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("org.isai.test-tone", "Isai Test Tone").with_features([clack_plugin::plugin::features::INSTRUMENT])
    }
    fn new_shared(_host: HostSharedHandle<'_>) -> Result<(), PluginError> {
        Ok(())
    }
    fn new_main_thread<'a>(_host: HostMainThreadHandle<'a>, _shared: &'a ()) -> Result<Main, PluginError> {
        Ok(Main { state: Mutex::new(b"isai-test-state".to_vec()) })
    }
}

pub struct Main {
    state: Mutex<Vec<u8>>,
}

impl<'a> PluginMainThread<'a, ()> for Main {}

impl PluginStateImpl for Main {
    fn save(&self, output: &mut OutputStream) -> Result<(), PluginError> {
        output.write_all(&self.state.lock().unwrap())?;
        Ok(())
    }
    fn load(&self, input: &mut InputStream) -> Result<(), PluginError> {
        let mut data = Vec::new();
        input.read_to_end(&mut data)?;
        *self.state.lock().unwrap() = data;
        Ok(())
    }
}

pub struct Tone {
    phase: f32,
    freq: f32,
    on: bool,
    rate: f32,
}

impl<'a> PluginAudioProcessor<'a, (), Main> for Tone {
    fn activate(_host: HostAudioProcessorHandle<'a>, _main: &Main, _shared: &'a (), config: PluginAudioConfiguration) -> Result<Self, PluginError> {
        Ok(Self { phase: 0.0, freq: 440.0, on: false, rate: config.sample_rate as f32 })
    }

    fn process(&mut self, _process: Process, mut audio: Audio, events: Events) -> Result<ProcessStatus, PluginError> {
        for e in events.input {
            if let Some(n) = e.as_event::<NoteOnEvent>() {
                let key = n.key().into_specific().unwrap_or(60) as f32;
                self.freq = 440.0 * 2f32.powf((key - 69.0) / 12.0);
                self.on = true;
            } else if e.as_event::<NoteOffEvent>().is_some() {
                self.on = false;
            }
        }
        for mut port in &mut audio {
            let Some(channels) = port.channels()?.into_f32() else { continue };
            let (start, freq, on, rate) = (self.phase, self.freq, self.on, self.rate);
            let mut end = start;
            for pair in channels {
                let buf = match pair {
                    ChannelPair::OutputOnly(b) => b,
                    ChannelPair::InPlace(b) => b,
                    ChannelPair::InputOutput(_, b) => b,
                    ChannelPair::InputOnly(_) => continue,
                };
                let mut ph = start;
                for s in buf.iter_mut() {
                    *s = if on { (ph * std::f32::consts::TAU).sin() * 0.3 } else { 0.0 };
                    ph = (ph + freq / rate).fract();
                }
                end = ph;
            }
            self.phase = end;
        }
        Ok(ProcessStatus::Continue)
    }
}

clack_export_entry!(SinglePluginEntry<TestTone>);
