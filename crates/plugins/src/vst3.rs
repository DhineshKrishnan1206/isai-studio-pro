//! VST3 plugins, through `vst3-host`.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex, TryLockError};

use isai_engine::mixer::BLOCK;
use isai_engine::plugin::{ExternalPlugin, NoteEvent, PluginRef, Transport};
use vst3_host::audio::BusAudioBuffers;
use vst3_host::midi::{MidiChannel, MidiEvent};
use vst3_host::{Plugin, PluginWindow, Vst3Host};

use crate::{Format, PluginInfo};

pub(crate) fn scan() -> Vec<PluginInfo> {
    let Ok(found) = vst3_host::simple::discover_plugins() else { return Vec::new() };
    found
        .into_iter()
        .map(|p| {
            let category = p.category.clone();
            let lower = category.to_lowercase();
            PluginInfo {
                format: Format::Vst3,
                path: p.path,
                uid: p.uid,
                name: p.name,
                vendor: p.vendor,
                instrument: lower.contains("instrument") || lower.contains("synth") || lower.contains("sampler"),
                category,
                has_gui: p.has_gui,
            }
        })
        .collect()
}

/// A loaded VST3 plugin, processing at the engine's sample rate. Lives on the
/// UI thread (it owns the plugin's window); the engine gets `handle()`.
pub struct Vst3Plugin {
    core: Arc<Core>,
    window: Option<PluginWindow>,
}

/// The part the audio thread uses.
struct Core {
    plugin: Arc<Mutex<Plugin>>,
    /// Only the audio thread (or an export) uses these, so the lock is never contended.
    buffers: Mutex<BusAudioBuffers>,
    /// The tempo last told to the plugin (as f64 bits) and whether it was playing.
    bpm: AtomicU64,
    playing: AtomicU64,
}

impl Vst3Plugin {
    /// Loads `uid` from the bundle at `path` and starts it processing.
    pub fn load(path: &Path, uid: &str, sample_rate: f64) -> Result<Self, String> {
        let mut host = Vst3Host::builder()
            .sample_rate(sample_rate)
            .block_size(BLOCK)
            .input_channels(2)
            .output_channels(2)
            .build()
            .map_err(|e| e.to_string())?;
        let mut plugin = if uid.is_empty() { host.load_plugin(path) } else { host.load_plugin_class(path, uid) }.map_err(|e| e.to_string())?;
        plugin.start_processing().map_err(|e| e.to_string())?;
        let buffers = plugin.create_bus_audio_buffers(BLOCK).map_err(|e| e.to_string())?;
        let core = Core { plugin: Arc::new(Mutex::new(plugin)), buffers: Mutex::new(buffers), bpm: AtomicU64::new(0), playing: AtomicU64::new(2) };
        Ok(Self { core: Arc::new(core), window: None })
    }

    /// The handle the engine processes through.
    pub fn handle(&self) -> PluginRef {
        PluginRef(self.core.clone())
    }

    /// The plugin's whole state (its sound, its settings), to save with the song.
    pub fn save_state(&self) -> Option<Vec<u8>> {
        self.core.plugin.lock().ok()?.save_state().ok()
    }

    pub fn load_state(&self, data: &[u8]) -> Result<(), String> {
        let mut p = self.core.plugin.lock().map_err(|_| "the plugin is unavailable".to_string())?;
        p.load_state(data).map_err(|e| e.to_string())
    }

    pub fn has_editor(&self) -> bool {
        self.core.plugin.lock().is_ok_and(|p| p.has_editor())
    }

    /// Opens the plugin's own window (or brings it back if it's open).
    pub fn open_window(&mut self) -> Result<(), String> {
        if self.window.as_ref().is_some_and(|w| w.is_open() && !w.closed_by_user()) {
            return Ok(());
        }
        self.window = None;
        let mut window = PluginWindow::new(self.core.plugin.clone());
        window.open().map_err(|e| e.to_string())?;
        self.window = Some(window);
        Ok(())
    }

    pub fn close_window(&mut self) {
        if let Some(mut w) = self.window.take() {
            w.close();
        }
    }

    /// Keeps the window responsive and notices when the user closed it.
    /// Returns whether it's open.
    pub fn service_window(&mut self) -> bool {
        let Some(w) = &mut self.window else { return false };
        let _ = w.service_platform_events();
        if w.closed_by_user() || !w.is_open() {
            self.close_window();
            return false;
        }
        true
    }

    pub fn window_open(&self) -> bool {
        self.window.as_ref().is_some_and(|w| w.is_open())
    }
}

impl Drop for Vst3Plugin {
    fn drop(&mut self) {
        self.close_window();
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        if let Ok(mut p) = self.plugin.lock() {
            let _ = p.stop_processing();
        }
    }
}

impl ExternalPlugin for Core {
    fn process(&self, l: &mut [f32], r: &mut [f32], events: &[NoteEvent], transport: &Transport, wait: bool) -> bool {
        let n = l.len().min(r.len()).min(BLOCK);
        let mut plugin = match if wait { self.plugin.lock().map_err(|_| ()) } else { self.plugin.try_lock().map_err(|e| matches!(e, TryLockError::WouldBlock)).map_err(|_| ()) } {
            Ok(p) => p,
            Err(()) => return false,
        };
        let Ok(mut buffers) = self.buffers.try_lock() else { return false };

        // Tempo and play state, only when they change.
        let bpm_bits = transport.bpm.to_bits();
        if self.bpm.swap(bpm_bits, Relaxed) != bpm_bits {
            let _ = plugin.set_tempo(transport.bpm);
        }
        let playing = u64::from(transport.playing);
        if self.playing.swap(playing, Relaxed) != playing {
            let _ = plugin.set_playing(transport.playing);
        }

        for e in events {
            let event = if e.velocity > 0 {
                MidiEvent::NoteOn { channel: MidiChannel::Ch1, note: e.key, velocity: e.velocity }
            } else {
                MidiEvent::NoteOff { channel: MidiChannel::Ch1, note: e.key, velocity: 0 }
            };
            let _ = plugin.send_midi_event_at(event, e.offset as i32);
        }

        // Size every channel to this block (within the room it already has: no allocation).
        let buffers = &mut *buffers;
        for bus in buffers.inputs.iter_mut().chain(buffers.outputs.iter_mut()) {
            for ch in &mut bus.channels {
                ch.resize(n, 0.0);
            }
        }
        if let Some(main) = buffers.inputs.first_mut() {
            for (i, ch) in main.channels.iter_mut().enumerate() {
                ch[..n].copy_from_slice(if i % 2 == 0 { &l[..n] } else { &r[..n] });
            }
        }
        if plugin.process_bus_audio(buffers).is_err() {
            return false;
        }
        match buffers.outputs.first().map(|b| b.channels.as_slice()) {
            Some([left, right, ..]) => {
                l[..n].copy_from_slice(&left[..n]);
                r[..n].copy_from_slice(&right[..n]);
            }
            Some([mono]) => {
                l[..n].copy_from_slice(&mono[..n]);
                r[..n].copy_from_slice(&mono[..n]);
            }
            _ => return false,
        }
        true
    }

    fn all_notes_off(&self) {
        if let Ok(mut p) = self.plugin.try_lock() {
            for note in 0..128u8 {
                let _ = p.send_midi_event_at(MidiEvent::NoteOff { channel: MidiChannel::Ch1, note, velocity: 0 }, 0);
            }
        }
    }
}
