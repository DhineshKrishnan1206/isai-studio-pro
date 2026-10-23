//! CLAP plugins, through `clack-host`.
//!
//! A CLAP plugin has a main-thread instance (its window, its state) and an
//! audio processor that moves to the audio thread. The processor lives in
//! `Core`, which the engine shares; `ClapPlugin`, on the UI thread, owns the
//! instance and takes the processor back to shut it down properly.

use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
use clack_extensions::gui::Window as ClapWindow;
use clack_extensions::gui::{GuiApiType, GuiConfiguration, PluginGui};
use clack_extensions::state::PluginState;
use clack_host::events::Match;
use clack_host::events::event_types::{NoteOffEvent, NoteOnEvent};
use clack_host::prelude::*;
use isai_engine::mixer::BLOCK;
use isai_engine::plugin::{ExternalPlugin, NoteEvent, PluginRef, Transport};

use crate::{Format, PluginInfo};

/// The standard places CLAP plugins are installed, plus `CLAP_PATH`.
fn folders() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::env::var_os("CLAP_PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        if let Some(h) = &home {
            out.push(h.join("Library/Audio/Plug-Ins/CLAP"));
        }
        out.push("/Library/Audio/Plug-Ins/CLAP".into());
    } else if cfg!(target_os = "windows") {
        if let Some(d) = std::env::var_os("COMMONPROGRAMFILES") {
            out.push(PathBuf::from(d).join("CLAP"));
        }
        if let Some(d) = std::env::var_os("LOCALAPPDATA") {
            out.push(PathBuf::from(d).join("Programs/Common/CLAP"));
        }
    } else {
        if let Some(h) = &home {
            out.push(h.join(".clap"));
        }
        out.push("/usr/lib/clap".into());
        out.push("/usr/local/lib/clap".into());
    }
    out
}

/// Every `.clap` file or bundle in a folder (and one level of subfolders).
fn find(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("clap"))
        {
            out.push(path);
        } else if depth > 0 && path.is_dir() {
            find(&path, depth - 1, out);
        }
    }
}

/// The plugins inside one `.clap` file or bundle.
pub(crate) fn describe(path: &Path) -> Vec<PluginInfo> {
    // SAFETY: loading a plugin runs its code; the app scans in a separate process.
    let Ok(entry) = (unsafe { PluginEntry::load(path) }) else {
        return Vec::new();
    };
    let Some(factory) = entry.get_plugin_factory() else {
        return Vec::new();
    };
    factory
        .plugin_descriptors()
        .filter_map(|d| {
            let id = d.id()?.to_str().ok()?.to_owned();
            let features: Vec<String> = d
                .features()
                .filter_map(|f| f.to_str().ok().map(str::to_owned))
                .collect();
            Some(PluginInfo {
                format: Format::Clap,
                path: path.to_path_buf(),
                uid: id,
                name: d
                    .name()
                    .and_then(|n| n.to_str().ok())
                    .unwrap_or("CLAP plugin")
                    .to_owned(),
                vendor: d
                    .vendor()
                    .and_then(|n| n.to_str().ok())
                    .unwrap_or_default()
                    .to_owned(),
                instrument: features.iter().any(|f| f == "instrument"),
                category: features.join("|"),
                has_gui: true,
            })
        })
        .collect()
}

pub(crate) fn scan() -> Vec<PluginInfo> {
    let mut files = Vec::new();
    for dir in folders() {
        find(&dir, 1, &mut files);
    }
    files.iter().flat_map(|f| describe(f)).collect()
}

// ------------------------------------------------------------------- host

struct IsaiHost;

struct Shared;

impl<'a> SharedHandler<'a> for Shared {
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {}
}

impl HostHandlers for IsaiHost {
    type Shared<'a> = Shared;
    type MainThread<'a> = ();
    type AudioProcessor<'a> = ();
}

fn host_info() -> Result<HostInfo, String> {
    HostInfo::new(
        "Isai Studio Pro",
        "Isai",
        "https://github.com/DhineshKrishnan1206/isai-studio-pro",
        env!("CARGO_PKG_VERSION"),
    )
    .map_err(|e| e.to_string())
}

/// What the audio thread needs: the processor and its preallocated buffers.
struct Audio {
    processor: StartedPluginAudioProcessor<IsaiHost>,
    inputs: AudioPorts,
    outputs: AudioPorts,
    in_bufs: [Vec<f32>; 2],
    out_bufs: [Vec<f32>; 2],
    events: EventBuffer,
    out_events: EventBuffer,
}

struct Core {
    /// `None` once the UI side has taken it back to shut the plugin down.
    audio: Mutex<Option<Audio>>,
}

/// A loaded CLAP plugin. Lives on the UI thread; the engine gets `handle()`.
pub struct ClapPlugin {
    instance: PluginInstance<IsaiHost>,
    core: Arc<Core>,
    gui: Option<Gui>,
}

struct Gui {
    ext: PluginGui,
    #[cfg(target_os = "macos")]
    window: Option<crate::macwin::HostWindow>,
}

impl ClapPlugin {
    pub fn load(path: &Path, id: &str, sample_rate: f64) -> Result<Self, String> {
        // SAFETY: loading a plugin runs its code; it was found by a scan in a separate process.
        let entry = unsafe { PluginEntry::load(path) }.map_err(|e| e.to_string())?;
        let id = CString::new(id).map_err(|e| e.to_string())?;
        let mut instance =
            PluginInstance::<IsaiHost>::new(|_| Shared, |_| (), &entry, &id, &host_info()?)
                .map_err(|e| e.to_string())?;
        let config = PluginAudioConfiguration {
            sample_rate,
            min_frames_count: 1,
            max_frames_count: BLOCK as u32,
        };
        let stopped = instance
            .activate(|_, _| (), config)
            .map_err(|e| e.to_string())?;
        let processor = stopped.start_processing().map_err(|e| e.to_string())?;
        let audio = Audio {
            processor,
            inputs: AudioPorts::with_capacity(2, 1),
            outputs: AudioPorts::with_capacity(2, 1),
            in_bufs: [vec![0.0; BLOCK], vec![0.0; BLOCK]],
            out_bufs: [vec![0.0; BLOCK], vec![0.0; BLOCK]],
            events: EventBuffer::with_capacity(1024),
            out_events: EventBuffer::with_capacity(1024),
        };
        Ok(Self {
            instance,
            core: Arc::new(Core {
                audio: Mutex::new(Some(audio)),
            }),
            gui: None,
        })
    }

    pub fn handle(&self) -> PluginRef {
        PluginRef(self.core.clone())
    }

    pub fn save_state(&mut self) -> Option<Vec<u8>> {
        let state: PluginState = self.instance.plugin_shared_handle().get_extension()?;
        let mut out = Vec::new();
        state.save(&self.instance.plugin_handle(), &mut out).ok()?;
        Some(out)
    }

    pub fn load_state(&mut self, data: &[u8]) -> Result<(), String> {
        let state: PluginState = self
            .instance
            .plugin_shared_handle()
            .get_extension()
            .ok_or("this plugin can't restore its sound")?;
        state
            .load(&self.instance.plugin_handle(), &mut &data[..])
            .map_err(|e| e.to_string())
    }

    /// Opens the plugin's window: inside one of ours on macOS, otherwise the
    /// plugin's own floating window.
    pub fn open_window(&mut self) -> Result<(), String> {
        if self.window_open() {
            return Ok(());
        }
        self.close_window();
        let ext: PluginGui = self
            .instance
            .plugin_shared_handle()
            .get_extension()
            .ok_or("this plugin has no window")?;
        let handle = self.instance.plugin_handle();
        #[cfg(target_os = "macos")]
        {
            let embedded = GuiConfiguration {
                api_type: GuiApiType::COCOA,
                is_floating: false,
            };
            if ext.is_api_supported(&handle, embedded) {
                ext.create(&handle, embedded).map_err(|e| e.to_string())?;
                let size = ext.get_size(&handle).map_or((800.0, 500.0), |s| {
                    (f64::from(s.width), f64::from(s.height))
                });
                let window = crate::macwin::HostWindow::new("Plugin", size)?;
                // SAFETY: the view stays alive until `close_window` destroys the plugin's GUI first.
                unsafe {
                    ext.set_parent(
                        &handle,
                        ClapWindow::from_generic_ptr(GuiApiType::COCOA, window.view()),
                    )
                }
                .map_err(|e| e.to_string())?;
                let _ = ext.show(&handle);
                self.gui = Some(Gui {
                    ext,
                    window: Some(window),
                });
                return Ok(());
            }
        }
        let api = if cfg!(target_os = "macos") {
            GuiApiType::COCOA
        } else if cfg!(target_os = "windows") {
            GuiApiType::WIN32
        } else {
            GuiApiType::X11
        };
        let floating = GuiConfiguration {
            api_type: api,
            is_floating: true,
        };
        if !ext.is_api_supported(&handle, floating) {
            return Err("Isai can't show this plugin's window on this system yet".into());
        }
        ext.create(&handle, floating).map_err(|e| e.to_string())?;
        ext.show(&handle).map_err(|e| e.to_string())?;
        self.gui = Some(Gui {
            ext,
            #[cfg(target_os = "macos")]
            window: None,
        });
        Ok(())
    }

    pub fn close_window(&mut self) {
        if let Some(gui) = self.gui.take() {
            let handle = self.instance.plugin_handle();
            let _ = gui.ext.hide(&handle);
            gui.ext.destroy(&handle);
            #[cfg(target_os = "macos")]
            drop(gui.window);
        }
    }

    pub fn window_open(&self) -> bool {
        match &self.gui {
            #[cfg(target_os = "macos")]
            Some(Gui {
                window: Some(w), ..
            }) => w.is_open(),
            Some(_) => true,
            None => false,
        }
    }

    /// Notices when the user closed our window. Returns whether it's open.
    pub fn service_window(&mut self) -> bool {
        self.instance.call_on_main_thread_callback();
        if self.gui.is_some() && !self.window_open() {
            self.close_window();
        }
        self.window_open()
    }
}

impl Drop for ClapPlugin {
    fn drop(&mut self) {
        self.close_window();
        // Take the processor back from the engine's side and shut down on this thread.
        let audio = self.core.audio.lock().ok().and_then(|mut a| a.take());
        if let Some(a) = audio {
            self.instance.deactivate(a.processor.stop_processing());
        }
    }
}

impl ExternalPlugin for Core {
    fn process(
        &self,
        l: &mut [f32],
        r: &mut [f32],
        events: &[NoteEvent],
        _transport: &Transport,
        wait: bool,
    ) -> bool {
        let n = l.len().min(r.len()).min(BLOCK);
        let mut guard = match if wait {
            self.audio.lock().ok()
        } else {
            self.audio.try_lock().ok()
        } {
            Some(g) => g,
            None => return false,
        };
        let Some(a) = guard.as_mut() else {
            return false;
        };

        a.events.clear();
        for e in events {
            let pckn = Pckn::new(0u16, 0u16, u16::from(e.key), Match::<u32>::All);
            if e.velocity > 0 {
                a.events.push(&NoteOnEvent::new(
                    e.offset,
                    pckn,
                    f64::from(e.velocity) / 127.0,
                ));
            } else {
                a.events.push(&NoteOffEvent::new(e.offset, pckn, 0.0));
            }
        }
        a.out_events.clear();
        a.in_bufs[0][..n].copy_from_slice(&l[..n]);
        a.in_bufs[1][..n].copy_from_slice(&r[..n]);

        let Audio {
            processor,
            inputs,
            outputs,
            in_bufs,
            out_bufs,
            events,
            out_events,
        } = a;
        let input_audio = inputs.with_input_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_input_only(
                in_bufs
                    .iter_mut()
                    .map(|b| InputChannel::variable(&mut b[..n])),
            ),
        }]);
        let mut output_audio = outputs.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only(
                out_bufs.iter_mut().map(|b| &mut b[..n]),
            ),
        }]);
        let input_events = InputEvents::from_buffer(events);
        let mut output_events = OutputEvents::from_buffer(out_events);
        if processor
            .process(
                &input_audio,
                &mut output_audio,
                &input_events,
                &mut output_events,
                None,
                None,
            )
            .is_err()
        {
            return false;
        }
        l[..n].copy_from_slice(&a.out_bufs[0][..n]);
        r[..n].copy_from_slice(&a.out_bufs[1][..n]);
        true
    }
}
