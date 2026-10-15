//! A plain macOS window to hold a plugin's view (CLAP and Audio Units).
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

pub struct HostWindow {
    window: Retained<NSWindow>,
}

impl HostWindow {
    pub fn new(title: &str, (w, h): (f64, f64)) -> Result<Self, String> {
        let mtm = MainThreadMarker::new().ok_or("plugin windows open on the main thread")?;
        let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable;
        let rect = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(w, h));
        // SAFETY: plain AppKit calls on the main thread.
        let window = unsafe {
            let window = NSWindow::initWithContentRect_styleMask_backing_defer(mtm.alloc(), rect, style, NSBackingStoreType::Buffered, false);
            window.setReleasedWhenClosed(false);
            window.setTitle(&NSString::from_str(title));
            window.center();
            window.makeKeyAndOrderFront(None);
            window
        };
        Ok(Self { window })
    }

    /// Its content view, for a plugin to draw into.
    pub fn view(&self) -> *mut std::ffi::c_void {
        self.window.contentView().map_or(std::ptr::null_mut(), |v| Retained::as_ptr(&v) as *mut std::ffi::c_void)
    }

    /// Puts a plugin's own view in the window, sized to it.
    ///
    /// # Safety
    /// `view` must be a valid `NSView`.
    pub unsafe fn set_view(&self, view: &objc2_app_kit::NSView) {
        let size = view.frame().size;
        self.window.setContentSize(size);
        self.window.setContentView(Some(view));
    }

    pub fn is_open(&self) -> bool {
        self.window.isVisible()
    }
}

impl Drop for HostWindow {
    fn drop(&mut self) {
        self.window.close();
    }
}
