//! Wake the GUI even when a native editor keeps the Windows message queue busy.
use std::{
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

pub(crate) struct RepaintHeartbeat {
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl RepaintHeartbeat {
    pub(crate) fn start(context: &eframe::CreationContext<'_>) -> Self {
        let ctx = context.egui_ctx.clone();
        #[cfg(windows)]
        let window = {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            context
                .window_handle()
                .ok()
                .and_then(|handle| match handle.as_raw() {
                    RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
                    _ => None,
                })
        };
        let (stop, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("gui-heartbeat".into())
            .spawn(move || {
                while matches!(
                    receiver.recv_timeout(Duration::from_millis(50)),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    // Native editors can keep system-generated paint messages busy.
                    // Send a bounded repaint request so it can run without waiting
                    // for an empty queue, without accumulating posted WM_PAINTs.
                    #[cfg(windows)]
                    if let Some(window) = window {
                        unsafe {
                            SendMessageTimeoutW(
                                window as *mut std::ffi::c_void,
                                WM_PAINT,
                                0,
                                0,
                                SMTO_BLOCK | SMTO_ABORTIFHUNG,
                                50,
                                std::ptr::null_mut(),
                            );
                        }
                        continue;
                    }
                    ctx.request_repaint();
                }
            })
            .expect("could not start GUI heartbeat");
        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for RepaintHeartbeat {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(windows)]
const WM_PAINT: u32 = 0x000F;
#[cfg(windows)]
const SMTO_BLOCK: u32 = 0x0001;
#[cfg(windows)]
const SMTO_ABORTIFHUNG: u32 = 0x0002;

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn SendMessageTimeoutW(
        window: *mut std::ffi::c_void,
        message: u32,
        wparam: usize,
        lparam: isize,
        flags: u32,
        timeout: u32,
        result: *mut usize,
    ) -> isize;
}
