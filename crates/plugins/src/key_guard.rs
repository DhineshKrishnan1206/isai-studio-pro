//! Stops plugin windows from beeping. A key press that no one takes in a
//! plugin's window travels up to macOS, which answers it with the error beep.
//! This sees every key press before a window does, and drops unhandled keys
//! aimed at a plugin window. Text fields and Command shortcuts still receive
//! their keys; mapped computer-keyboard notes are routed to the DAW first.
//! Isai's own windows are Qt windows and are never touched.
use std::ffi::CStr;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, Once, OnceLock};

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags};

static INSTALL: Once = Once::new();
type KeyHandler = Arc<dyn Fn(u16, bool) + Send + Sync>;
static HANDLER: OnceLock<Mutex<Option<KeyHandler>>> = OnceLock::new();

pub fn set_handler(handler: impl Fn(u16, bool) + Send + Sync + 'static) {
    if let Ok(mut current) = HANDLER.get_or_init(|| Mutex::new(None)).lock() {
        *current = Some(Arc::new(handler));
    }
}

/// Installs the filter once, for the whole app. Call it on the main thread
/// before a plugin window opens.
pub fn install() {
    INSTALL.call_once(|| {
        let key_down = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY: AppKit passes a valid event for the duration of the call.
            let drop_it = MainThreadMarker::new()
                .is_some_and(|mtm| drops(unsafe { event.as_ref() }, mtm, true));
            if drop_it {
                std::ptr::null_mut()
            } else {
                event.as_ptr()
            }
        });
        let key_up = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY: AppKit passes a valid event for the duration of the call.
            let drop_it = MainThreadMarker::new()
                .is_some_and(|mtm| drops(unsafe { event.as_ref() }, mtm, false));
            if drop_it {
                std::ptr::null_mut()
            } else {
                event.as_ptr()
            }
        });
        // SAFETY: the monitor is set up on the main thread. AppKit keeps its own
        // copy of the block, and it lives as long as the app does.
        let down_monitor = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &key_down)
        };
        let up_monitor = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyUp, &key_up)
        };
        // Keep the monitor for the whole run of the app.
        std::mem::forget(down_monitor);
        std::mem::forget(up_monitor);
    });
}

/// Whether a key press should be dropped rather than passed on.
fn drops(event: &NSEvent, mtm: MainThreadMarker, down: bool) -> bool {
    let Some(window) = event.window(mtm) else {
        return false;
    };
    if !is_plugin_window(&window) {
        return false;
    }
    let command = event
        .modifierFlags()
        .contains(NSEventModifierFlags::Command);
    if command
        || window
            .firstResponder()
            .is_some_and(|responder| is_text_input(&responder))
    {
        return false;
    }
    call_handler(event, down);
    true
}

fn is_text_input(responder: &AnyObject) -> bool {
    let name = class_name(responder);
    name == "NSTextField"
        || name.ends_with("TextField")
        || name == "NSTextView"
        || name.ends_with("TextView")
}

fn call_handler(event: &NSEvent, down: bool) {
    if let Some(handler) = HANDLER
        .get()
        .and_then(|h| h.lock().ok())
        .and_then(|h| h.clone())
    {
        handler(event.keyCode(), down);
    }
}

/// The plain windows plugins open: vst3-host's, and our own (`macwin`).
fn is_plugin_window(window: &AnyObject) -> bool {
    matches!(class_name(window).as_str(), "NSWindow" | "IsaiPluginWindow")
}

/// The class name, without the `NSKVONotifying_` prefix AppKit adds to an
/// object that someone observes (a plugin window's does, so the exact name
/// would never match).
fn class_name(object: &AnyObject) -> String {
    let name: &CStr = object.class().name();
    let name = name.to_string_lossy();
    name.strip_prefix("NSKVONotifying_")
        .unwrap_or(&name)
        .to_owned()
}
