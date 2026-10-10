//! Exit-only UI: native drawing continues while plugin state blocks the main thread.
use crate::App;
use std::ffi::{c_char, c_void, CString};

extern "C" {
    fn uh_shutdown_overlay_start(window: isize) -> *mut c_void;
    fn uh_shutdown_overlay_update(
        overlay: *mut c_void,
        text: *const c_char,
        done: usize,
        total: usize,
    );
    fn uh_shutdown_overlay_stop(overlay: *mut c_void);
}

#[derive(Default)]
pub(crate) struct Shutdown {
    window: isize,
    overlay: Option<Overlay>,
    saved: bool,
}

struct Overlay(*mut c_void);

impl Shutdown {
    pub(crate) fn set_window(&mut self, context: &eframe::CreationContext<'_>) {
        #[cfg(windows)]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = context.window_handle() {
                if let RawWindowHandle::Win32(handle) = handle.as_raw() {
                    self.window = handle.hwnd.get();
                }
            }
        }
        #[cfg(not(windows))]
        let _ = context;
    }

    fn progress(&self, text: &str, done: usize, total: usize) {
        if let Some(overlay) = &self.overlay {
            let text = CString::new(text.replace('\0', " ")).unwrap();
            unsafe { uh_shutdown_overlay_update(overlay.0, text.as_ptr(), done, total) };
        }
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        unsafe { uh_shutdown_overlay_stop(self.0) };
    }
}

impl App {
    pub(crate) fn begin_shutdown(&mut self) {
        // Headless tests and startup failures have no visible window.
        if self.shutdown.window != 0 && self.shutdown.overlay.is_none() {
            let overlay = unsafe { uh_shutdown_overlay_start(self.shutdown.window) };
            if !overlay.is_null() {
                self.shutdown.overlay = Some(Overlay(overlay));
            } else {
                eprintln!("Could not show shutdown progress overlay");
            }
        }
    }

    pub(crate) fn save_on_shutdown(&mut self) {
        if self.shutdown.saved {
            return;
        }
        self.begin_shutdown();
        self.repaint_heartbeat.take();
        let plugins: Vec<_> = self
            .instances
            .iter()
            .map(|i| (i.id, i.label.clone()))
            .collect();
        let total = plugins.len();
        let mut errors = 0;
        self.shutdown.progress("Stopping audio...", 0, total);
        self.pause_audio();
        // Browser conditions change without any other save trigger.
        self.save_session();
        for (done, (id, label)) in plugins.into_iter().enumerate() {
            self.shutdown
                .progress(&format!("Saving: {label}"), done, total);
            if let Err(error) = self.save_plugin_state(id) {
                errors += 1;
                eprintln!("Could not save plugin state on exit: {error}");
            }
        }
        self.shutdown.saved = true;
        let message = if errors == 0 {
            "Closing plugins...".to_owned()
        } else {
            format!("Closing: {errors} save error(s); see log")
        };
        self.shutdown.progress(&message, total, total);
    }
}
