//! Audio Units (macOS), through Apple's Audio Unit C API.
//!
//! Instruments ('aumu') and effects ('aufx', 'aumf'), including Apple's own
//! (its sampler, reverbs, delays, EQ). An effect pulls its input from us
//! through a render callback while `AudioUnitRender` runs, so the input
//! buffers only need to live for that call.

use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use isai_engine::mixer::BLOCK;
use isai_engine::plugin::{ExternalPlugin, NoteEvent, PluginRef, Transport};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::msg_send;
use objc2_app_kit::NSView;
use objc2_audio_toolbox::{
    AURenderCallbackStruct, AudioComponent, AudioComponentCopyName, AudioComponentDescription, AudioComponentFindNext,
    AudioComponentGetDescription, AudioComponentInstanceDispose, AudioComponentInstanceNew, AudioUnit, AudioUnitGetProperty,
    AudioUnitGetPropertyInfo, AudioUnitInitialize, AudioUnitRender, AudioUnitRenderActionFlags, AudioUnitSetProperty,
    AudioUnitUninitialize, MusicDeviceMIDIEvent,
};
use objc2_core_audio_types::{AudioBuffer, AudioBufferList, AudioStreamBasicDescription, AudioTimeStamp, AudioTimeStampFlags};
use objc2_core_foundation::{CFData, CFPropertyList, CFPropertyListCreateData, CFPropertyListCreateWithData, CFPropertyListFormat, CFRetained, CFString, CFType};
use objc2_foundation::{NSBundle, NSSize, NSString, NSURL};

use crate::macwin::HostWindow;
use crate::{Format, PluginInfo};

const fn four(c: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*c)
}

const MUSIC_DEVICE: u32 = four(b"aumu");
const EFFECT: u32 = four(b"aufx");
const MUSIC_EFFECT: u32 = four(b"aumf");

// Property ids and scopes (AudioUnitProperties.h).
const PROP_CLASS_INFO: u32 = 0;
const PROP_STREAM_FORMAT: u32 = 8;
const PROP_MAX_FRAMES: u32 = 14;
const PROP_SET_RENDER_CALLBACK: u32 = 23;
const PROP_COCOA_UI: u32 = 31;
const SCOPE_GLOBAL: u32 = 0;
const SCOPE_INPUT: u32 = 1;
const SCOPE_OUTPUT: u32 = 2;

fn fourcc(v: u32) -> String {
    v.to_be_bytes().iter().map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '?' }).collect()
}

/// "aumu:subt:manu", the id saved in songs.
fn uid(d: &AudioComponentDescription) -> String {
    format!("{}:{}:{}", fourcc(d.componentType), fourcc(d.componentSubType), fourcc(d.componentManufacturer))
}

fn parse_uid(uid: &str) -> Option<AudioComponentDescription> {
    let parts: Vec<&str> = uid.split(':').collect();
    let code = |s: &str| -> Option<u32> { <[u8; 4]>::try_from(s.as_bytes()).ok().map(|b| four(&b)) };
    let [t, s, m] = parts.as_slice() else { return None };
    Some(AudioComponentDescription { componentType: code(t)?, componentSubType: code(s)?, componentManufacturer: code(m)?, componentFlags: 0, componentFlagsMask: 0 })
}

pub(crate) fn scan() -> Vec<PluginInfo> {
    let mut out = Vec::new();
    for kind in [MUSIC_DEVICE, EFFECT, MUSIC_EFFECT] {
        let mut want = AudioComponentDescription { componentType: kind, componentSubType: 0, componentManufacturer: 0, componentFlags: 0, componentFlagsMask: 0 };
        let mut comp: AudioComponent = std::ptr::null_mut();
        loop {
            // SAFETY: plain Audio Component lookups; nothing is loaded.
            comp = unsafe { AudioComponentFindNext(comp, NonNull::from(&mut want)) };
            if comp.is_null() {
                break;
            }
            let mut desc = want;
            let mut name: *const CFString = std::ptr::null();
            // SAFETY: `comp` came from FindNext; the out-pointers are valid.
            unsafe {
                AudioComponentGetDescription(comp, NonNull::from(&mut desc));
                AudioComponentCopyName(comp, NonNull::from(&mut name));
            }
            // "Vendor: Plugin"
            let full = if name.is_null() {
                String::new()
            } else {
                // SAFETY: CopyName returns a +1 CFString, released here.
                let s = unsafe { CFRetained::from_raw(NonNull::new_unchecked(name as *mut CFString)) };
                s.to_string()
            };
            let (vendor, plugin) = full.split_once(": ").map_or((String::new(), full.clone()), |(v, p)| (v.to_owned(), p.to_owned()));
            out.push(PluginInfo {
                format: Format::Au,
                path: PathBuf::from(format!("AudioUnit/{}", uid(&desc))),
                uid: uid(&desc),
                name: plugin,
                vendor,
                category: if kind == MUSIC_DEVICE { "Instrument".into() } else { "Fx".into() },
                instrument: kind == MUSIC_DEVICE,
                has_gui: true,
            });
        }
    }
    out
}

pub(crate) fn exists(uid: &str) -> bool {
    let Some(mut desc) = parse_uid(uid) else { return false };
    // SAFETY: a lookup only.
    !unsafe { AudioComponentFindNext(std::ptr::null_mut(), NonNull::from(&mut desc)) }.is_null()
}

/// Two non-interleaved channels: what we hand `AudioUnitRender`.
#[repr(C)]
struct StereoBufferList {
    count: u32,
    buffers: [AudioBuffer; 2],
}

struct Unit(AudioUnit);
// SAFETY: the unit is only used behind `Core`'s mutex, and created and
// disposed on the main thread.
unsafe impl Send for Unit {}

/// An effect's input for the block being rendered. The render callback reads
/// it through a raw pointer while `AudioUnitRender` runs, hence the cell.
struct Inputs(UnsafeCell<[Vec<f32>; 2]>);

struct Audio {
    unit: Unit,
    effect: bool,
    inputs: Box<Inputs>,
    out_bufs: [Vec<f32>; 2],
    frames: u32,
    sample_time: f64,
}

struct Core {
    audio: Mutex<Audio>,
}

/// A loaded Audio Unit. Lives on the UI thread; the engine gets `handle()`.
pub struct AuPlugin {
    core: Arc<Core>,
    unit: AudioUnit,
    window: Option<HostWindow>,
}

/// The render callback for effects: copies our input into the unit.
unsafe extern "C-unwind" fn input_proc(
    refcon: NonNull<c_void>,
    _flags: NonNull<AudioUnitRenderActionFlags>,
    _ts: NonNull<AudioTimeStamp>,
    _bus: u32,
    frames: u32,
    data: *mut AudioBufferList,
) -> i32 {
    // SAFETY: `refcon` is the `Inputs` of a unit being rendered (its `Core`
    // is locked for the whole `AudioUnitRender`); `data` is the unit's own list.
    unsafe {
        let inputs = &*(*(refcon.as_ptr() as *const Inputs)).0.get();
        let list = &mut *(data as *mut StereoBufferList);
        let n = (frames as usize).min(inputs[0].len());
        for (i, buf) in list.buffers.iter_mut().take(list.count.min(2) as usize).enumerate() {
            if !buf.mData.is_null() {
                std::ptr::copy_nonoverlapping(inputs[i].as_ptr(), buf.mData as *mut f32, n);
            }
        }
    }
    0
}

fn check(status: i32, what: &str) -> Result<(), String> {
    if status == 0 { Ok(()) } else { Err(format!("{what} failed ({status})")) }
}

impl AuPlugin {
    pub fn load(uid: &str, sample_rate: f64) -> Result<Self, String> {
        let mut desc = parse_uid(uid).ok_or("not an Audio Unit id")?;
        // SAFETY: Audio Unit C API calls on the main thread, with valid pointers.
        unsafe {
            let comp = AudioComponentFindNext(std::ptr::null_mut(), NonNull::from(&mut desc));
            if comp.is_null() {
                return Err("this Audio Unit isn't installed".into());
            }
            let mut unit: AudioUnit = std::ptr::null_mut();
            check(AudioComponentInstanceNew(comp, NonNull::from(&mut unit)), "opening the Audio Unit")?;
            let effect = desc.componentType != MUSIC_DEVICE;
            let format = AudioStreamBasicDescription {
                mSampleRate: sample_rate,
                mFormatID: four(b"lpcm"),
                // Float, packed, non-interleaved.
                mFormatFlags: 1 | 8 | 32,
                mBytesPerPacket: 4,
                mFramesPerPacket: 1,
                mBytesPerFrame: 4,
                mChannelsPerFrame: 2,
                mBitsPerChannel: 32,
                mReserved: 0,
            };
            let size = std::mem::size_of::<AudioStreamBasicDescription>() as u32;
            let set = |prop: u32, scope: u32, data: *const c_void, len: u32| AudioUnitSetProperty(unit, prop, scope, 0, data, len);
            check(set(PROP_STREAM_FORMAT, SCOPE_OUTPUT, &format as *const _ as *const c_void, size), "setting the output format")?;
            if effect {
                check(set(PROP_STREAM_FORMAT, SCOPE_INPUT, &format as *const _ as *const c_void, size), "setting the input format")?;
            }
            let max = BLOCK as u32;
            let _ = set(PROP_MAX_FRAMES, SCOPE_GLOBAL, &max as *const _ as *const c_void, 4);

            let inputs = Box::new(Inputs(UnsafeCell::new([vec![0.0; BLOCK], vec![0.0; BLOCK]])));
            // The callback reads the boxed inputs, which never move.
            let refcon = &*inputs as *const Inputs as *mut c_void;
            let core = Arc::new(Core {
                audio: Mutex::new(Audio { unit: Unit(unit), effect, inputs, out_bufs: [vec![0.0; BLOCK], vec![0.0; BLOCK]], frames: 0, sample_time: 0.0 }),
            });
            if effect {
                let callback = AURenderCallbackStruct { inputProc: Some(input_proc), inputProcRefCon: refcon };
                check(
                    set(PROP_SET_RENDER_CALLBACK, SCOPE_INPUT, &callback as *const _ as *const c_void, std::mem::size_of::<AURenderCallbackStruct>() as u32),
                    "connecting the effect's input",
                )?;
            }
            check(AudioUnitInitialize(unit), "starting the Audio Unit")?;
            Ok(Self { core, unit, window: None })
        }
    }

    pub fn handle(&self) -> PluginRef {
        PluginRef(self.core.clone())
    }

    /// The unit's state ("class info"), as a binary property list.
    pub fn save_state(&mut self) -> Option<Vec<u8>> {
        let _guard = self.core.audio.lock().ok()?;
        let mut plist: *const CFType = std::ptr::null();
        let mut size = std::mem::size_of::<*const CFType>() as u32;
        // SAFETY: ClassInfo returns a +1 property list.
        unsafe {
            if AudioUnitGetProperty(self.unit, PROP_CLASS_INFO, SCOPE_GLOBAL, 0, NonNull::from(&mut plist).cast(), NonNull::from(&mut size)) != 0 || plist.is_null() {
                return None;
            }
            let plist: CFRetained<CFPropertyList> = CFRetained::from_raw(NonNull::new_unchecked(plist as *mut CFPropertyList));
            let data = CFPropertyListCreateData(None, Some(&plist), CFPropertyListFormat::BinaryFormat_v1_0, 0, std::ptr::null_mut())?;
            Some(data.to_vec())
        }
    }

    pub fn load_state(&mut self, data: &[u8]) -> Result<(), String> {
        let _guard = self.core.audio.lock().map_err(|_| "plugin lock")?;
        // SAFETY: the property list is valid for the call; the unit copies it.
        unsafe {
            let bytes = CFData::from_bytes(data);
            let plist = CFPropertyListCreateWithData(None, Some(&bytes), 0, std::ptr::null_mut(), std::ptr::null_mut()).ok_or("not a saved Audio Unit state")?;
            let raw: *const CFType = CFRetained::as_ptr(&plist).as_ptr();
            check(
                AudioUnitSetProperty(self.unit, PROP_CLASS_INFO, SCOPE_GLOBAL, 0, &raw as *const _ as *const c_void, std::mem::size_of::<*const CFType>() as u32),
                "restoring the Audio Unit's sound",
            )
        }
    }

    /// The unit's own Cocoa view, or Apple's generic controls for it.
    fn make_view(&self) -> Option<Retained<NSView>> {
        // SAFETY: AppKit and Audio Unit calls on the main thread.
        unsafe {
            let mut size = 0u32;
            let mut writable = 0u8;
            if AudioUnitGetPropertyInfo(self.unit, PROP_COCOA_UI, SCOPE_GLOBAL, 0, &mut size, &mut writable) == 0 && size > 0 {
                // AudioUnitCocoaViewInfo: a bundle URL, then one or more class names.
                let mut info = vec![0u8; size as usize];
                let mut len = size;
                if AudioUnitGetProperty(self.unit, PROP_COCOA_UI, SCOPE_GLOBAL, 0, NonNull::new_unchecked(info.as_mut_ptr()).cast(), NonNull::from(&mut len)) == 0 {
                    let url = *(info.as_ptr() as *const *const NSURL);
                    let class_name = *(info.as_ptr().add(std::mem::size_of::<*const c_void>()) as *const *const NSString);
                    if !url.is_null() && !class_name.is_null() {
                        let bundle = NSBundle::bundleWithURL(&*url);
                        if let Some(bundle) = bundle {
                            let class: Option<&AnyClass> = bundle.classNamed(&*class_name);
                            if let Some(class) = class {
                                let factory: Retained<AnyObject> = msg_send![msg_send![class, alloc], init];
                                let view: Option<Retained<NSView>> = msg_send![&factory, uiViewForAudioUnit: self.unit, withSize: NSSize::new(800.0, 500.0)];
                                if view.is_some() {
                                    return view;
                                }
                            }
                        }
                    }
                }
            }
            // Apple's generic view, from CoreAudioKit.
            let class = AnyClass::get(c"AUGenericView")?;
            let view: Option<Retained<NSView>> = msg_send![msg_send![class, alloc], initWithAudioUnit: self.unit];
            view
        }
    }

    pub fn open_window(&mut self) -> Result<(), String> {
        if self.window_open() {
            return Ok(());
        }
        let view = self.make_view().ok_or("this Audio Unit has no window Isai can show")?;
        let window = HostWindow::new("Audio Unit", (800.0, 500.0))?;
        // SAFETY: `view` is a valid NSView.
        unsafe { window.set_view(&view) };
        self.window = Some(window);
        Ok(())
    }

    pub fn window_open(&self) -> bool {
        self.window.as_ref().is_some_and(HostWindow::is_open)
    }

    pub fn service_window(&mut self) -> bool {
        if self.window.is_some() && !self.window_open() {
            self.window = None;
        }
        self.window_open()
    }
}

impl Drop for AuPlugin {
    fn drop(&mut self) {
        self.window = None;
        if let Ok(mut audio) = self.core.audio.lock() {
            // SAFETY: torn down once, on the main thread; the engine skips a null unit.
            unsafe {
                AudioUnitUninitialize(self.unit);
                AudioComponentInstanceDispose(self.unit);
            }
            audio.unit = Unit(std::ptr::null_mut());
        }
    }
}

impl ExternalPlugin for Core {
    fn process(&self, l: &mut [f32], r: &mut [f32], events: &[NoteEvent], _transport: &Transport, wait: bool) -> bool {
        let n = l.len().min(r.len()).min(BLOCK);
        let mut guard = match if wait { self.audio.lock().ok() } else { self.audio.try_lock().ok() } {
            Some(g) => g,
            None => return false,
        };
        let a = &mut *guard;
        if a.unit.0.is_null() {
            return false;
        }
        // SAFETY: the unit is valid while `a.unit` isn't null; buffers outlive the render.
        unsafe {
            for e in events {
                let (status, vel) = if e.velocity > 0 { (0x90, u32::from(e.velocity)) } else { (0x80, 0) };
                MusicDeviceMIDIEvent(a.unit.0, status, u32::from(e.key), vel, e.offset);
            }
            if a.effect {
                let inputs = &mut *a.inputs.0.get();
                inputs[0][..n].copy_from_slice(&l[..n]);
                inputs[1][..n].copy_from_slice(&r[..n]);
            }
            a.frames = n as u32;
            let bytes = (n * 4) as u32;
            let mut list = StereoBufferList {
                count: 2,
                buffers: [
                    AudioBuffer { mNumberChannels: 1, mDataByteSize: bytes, mData: a.out_bufs[0].as_mut_ptr() as *mut c_void },
                    AudioBuffer { mNumberChannels: 1, mDataByteSize: bytes, mData: a.out_bufs[1].as_mut_ptr() as *mut c_void },
                ],
            };
            let mut ts: AudioTimeStamp = std::mem::zeroed();
            ts.mSampleTime = a.sample_time;
            ts.mFlags = AudioTimeStampFlags::SampleTimeValid;
            let mut flags = AudioUnitRenderActionFlags(0);
            let status = AudioUnitRender(a.unit.0, &mut flags, NonNull::from(&ts), 0, n as u32, NonNull::from(&mut list).cast());
            a.sample_time += n as f64;
            if status != 0 {
                return false;
            }
        }
        l[..n].copy_from_slice(&a.out_bufs[0][..n]);
        r[..n].copy_from_slice(&a.out_bufs[1][..n]);
        true
    }
}

// AUGenericView lives in CoreAudioKit.
#[link(name = "CoreAudioKit", kind = "framework")]
unsafe extern "C" {}
