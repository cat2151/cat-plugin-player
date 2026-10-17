use crate::ffi::Host;
impl Host {
    pub fn configure_editor_placement(
        &self,
        context: &eframe::CreationContext<'_>,
        placement: crate::window_config::Placement,
    ) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        match placement {
            crate::window_config::Placement::RightThenBottomRight => {
                if let Ok(handle) = context.window_handle() {
                    if let RawWindowHandle::Win32(handle) = handle.as_raw() {
                        self.set_main_window(handle.hwnd.get());
                    }
                }
            }
        }
    }
}
