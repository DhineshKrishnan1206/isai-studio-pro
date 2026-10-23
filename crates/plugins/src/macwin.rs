//! A plain macOS window to hold a plugin's view (CLAP and Audio Units).
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

define_class!(
    // A plain window that ignores keys nobody else took. Without this, a key
    // the plugin's view doesn't use is passed on to macOS, which answers with
    // its error beep. Plugin text fields still get their keys, because the
    // view handles those before the window sees them.
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[name = "IsaiPluginWindow"]
    struct QuietWindow;

    impl QuietWindow {
        #[unsafe(method(keyDown:))]
        fn key_down(&self, _event: &AnyObject) {}
    }
);

pub struct HostWindow {
    window: Retained<QuietWindow>,
}

impl HostWindow {
    pub fn new(title: &str, (w, h): (f64, f64)) -> Result<Self, String> {
        let mtm = MainThreadMarker::new().ok_or("plugin windows open on the main thread")?;
        crate::key_guard::install();
        let style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable;
        let rect = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(w, h));
        // SAFETY: plain AppKit calls on the main thread.
        let window = unsafe {
            let window: Retained<QuietWindow> = msg_send![
                mtm.alloc::<QuietWindow>(),
                initWithContentRect: rect,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ];
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
        self.window.contentView().map_or(std::ptr::null_mut(), |v| {
            Retained::as_ptr(&v) as *mut std::ffi::c_void
        })
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
