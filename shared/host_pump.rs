//! Main-thread event-loop bridge; no plugin discovery or rendering.
use super::{Host, UhHost};

unsafe extern "C" {
    fn uh_pump(host: *mut UhHost);
    fn uh_pump_startup(host: *mut UhHost);
    fn uh_main_thread_work_pending() -> i32;
}

impl Host {
    /// Call once per UI frame. Callbacks are delivered from inside this call.
    pub fn pump(&self) {
        unsafe { uh_pump(self.raw) }
    }

    /// Before eframe starts, also dispatch native messages for plugin windows.
    pub fn pump_startup(&self) {
        unsafe { uh_pump_startup(self.raw) }
    }

    /// UI-thread snapshot of the shared callback queue, excluding window messages.
    /// If false, defer all hosts' pumps until the next poll so newly posted work
    /// cannot invalidate audio bindings outside the caller's render boundary.
    #[allow(dead_code)] // Used by the editor's app-owned chains without plugin windows.
    pub fn main_thread_work_pending() -> bool {
        unsafe { uh_main_thread_work_pending() != 0 }
    }
}
