//! Raw declarations for shim/uapmd_shim.h, plus a thin safe wrapper.
//!
//! Everything here must be used from the main (UI) thread only; `Host` is
//! neither `Send` nor `Sync` because it holds a raw pointer.

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;

#[path = "plugin_state.rs"]
mod plugin_state;

#[repr(C)]
pub struct UhHost {
    _private: [u8; 0],
}

#[repr(C)]
pub struct UhProcessor {
    _private: [u8; 0],
}

pub type UhWakeFn = unsafe extern "C" fn(user: *mut c_void);
pub type UhScanDoneFn = unsafe extern "C" fn(user: *mut c_void, error: *const c_char);
pub type UhInstanceFn =
    unsafe extern "C" fn(user: *mut c_void, instance_id: i32, error: *const c_char);

pub const UH_PLUGIN_NAME: i32 = 0;
pub const UH_PLUGIN_VENDOR: i32 = 1;
pub const UH_PLUGIN_FORMAT: i32 = 2;
pub const UH_PLUGIN_ID: i32 = 3;
pub const UH_PLUGIN_PATH: i32 = 4;
pub const UH_PLUGIN_KIND: i32 = 5;

pub const UH_OK: i32 = 0;

extern "C" {
    fn uh_create(wake: Option<UhWakeFn>, wake_user: *mut c_void) -> *mut UhHost;
    fn uh_destroy(host: *mut UhHost);
    fn uh_pump(host: *mut UhHost);
    fn uh_pump_startup(host: *mut UhHost);
    fn uh_restore_plugin(
        host: *mut UhHost,
        format: *const c_char,
        id: *const c_char,
        name: *const c_char,
        vendor: *const c_char,
        path: *const c_char,
    ) -> i32;
    fn uh_scan_async(
        host: *mut UhHost,
        rescan: i32,
        done: Option<UhScanDoneFn>,
        user: *mut c_void,
    ) -> i32;
    fn uh_plugin_count(host: *mut UhHost) -> i32;
    fn uh_plugin_info(
        host: *mut UhHost,
        index: i32,
        field: i32,
        buf: *mut c_char,
        buf_len: i32,
    ) -> i32;
    fn uh_instance_create(
        host: *mut UhHost,
        index: i32,
        sample_rate: u32,
        buffer_size: u32,
        done: Option<UhInstanceFn>,
        user: *mut c_void,
    );
    fn uh_instance_destroy(host: *mut UhHost, instance_id: i32);
    fn uh_processor_destroy(processor: *mut UhProcessor);
    fn uh_chain_create(
        host: *mut UhHost,
        instrument_id: i32,
        effect_ids: *const i32,
        effect_count: i32,
        bypass: i32,
        sample_rate: u32,
        max_frames: u32,
    ) -> *mut UhProcessor;
    fn uh_processor_process(
        processor: *mut UhProcessor,
        ump_words: *const u32,
        ump_word_count: i32,
        out_interleaved: *mut f32,
        out_channels: i32,
        frames: i32,
    ) -> i32;
    fn uh_ui_set_main_window(host: *mut UhHost, hwnd: *mut c_void);
    fn uh_ui_show(host: *mut UhHost, instance_id: i32) -> i32;
    fn uh_ui_hide(host: *mut UhHost, instance_id: i32);
    fn uh_ui_is_visible(host: *mut UhHost, instance_id: i32) -> i32;
    fn uh_ui_thumbnail(
        host: *mut UhHost,
        instance_id: i32,
        rgba: *mut u8,
        len: i32,
        width: *mut i32,
        height: *mut i32,
    ) -> i32;
}

/// Turns the `error` argument of a callback into an owned string.
///
/// # Safety
/// `error` must be null or a valid NUL-terminated string.
pub unsafe fn error_string(error: *const c_char) -> Option<String> {
    if error.is_null() {
        None
    } else {
        Some(CStr::from_ptr(error).to_string_lossy().into_owned())
    }
}

#[derive(Clone, Debug)]
pub struct PluginInfo {
    pub index: i32,
    pub name: String,
    pub vendor: String,
    pub format: String,
    pub id: String,
    pub bundle_path: String,
    pub kind: crate::plugin_list::PluginKind,
}

pub struct Host {
    raw: *mut UhHost,
}

impl Host {
    /// `wake` may be called from any thread, for as long as the host exists;
    /// whatever `wake_user` points to must stay valid that long.
    pub fn new(wake: UhWakeFn, wake_user: *mut c_void) -> Option<Self> {
        let raw = unsafe { uh_create(Some(wake), wake_user) };
        if raw.is_null() {
            None
        } else {
            Some(Self { raw })
        }
    }

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
                        unsafe {
                            uh_ui_set_main_window(self.raw, handle.hwnd.get() as *mut c_void)
                        };
                    }
                }
            }
        }
    }

    /// Call once per UI frame. Callbacks are delivered from inside this call.
    pub fn pump(&self) {
        unsafe { uh_pump(self.raw) }
    }

    /// Before eframe starts, also dispatch native messages for plugin windows.
    pub fn pump_startup(&self) {
        unsafe { uh_pump_startup(self.raw) }
    }

    pub fn restore_plugin(&self, key: &crate::config::PluginKey) -> Option<PluginInfo> {
        let format = CString::new(key.format.as_str()).ok()?;
        let id = CString::new(key.id.as_str()).ok()?;
        let name = CString::new(key.name.as_str()).ok()?;
        let vendor = CString::new(key.vendor.as_str()).ok()?;
        let path = CString::new(key.bundle_path.as_str()).ok()?;
        let index = unsafe {
            uh_restore_plugin(
                self.raw,
                format.as_ptr(),
                id.as_ptr(),
                name.as_ptr(),
                vendor.as_ptr(),
                path.as_ptr(),
            )
        };
        (index >= 0).then(|| PluginInfo {
            index,
            format: key.format.clone(),
            id: key.id.clone(),
            name: key.name.clone(),
            vendor: key.vendor.clone(),
            bundle_path: key.bundle_path.clone(),
            kind: crate::plugin_list::PluginKind::Unknown,
        })
    }

    /// Returns false if a scan is already running.
    pub fn scan_async(&self, rescan: bool, done: UhScanDoneFn, user: *mut c_void) -> bool {
        unsafe { uh_scan_async(self.raw, rescan as i32, Some(done), user) == 0 }
    }

    pub fn plugins(&self) -> Vec<PluginInfo> {
        let count = unsafe { uh_plugin_count(self.raw) };
        (0..count)
            .map(|index| PluginInfo {
                index,
                name: self.plugin_field(index, UH_PLUGIN_NAME),
                vendor: self.plugin_field(index, UH_PLUGIN_VENDOR),
                format: self.plugin_field(index, UH_PLUGIN_FORMAT),
                id: self.plugin_field(index, UH_PLUGIN_ID),
                bundle_path: self.plugin_field(index, UH_PLUGIN_PATH),
                kind: crate::plugin_list::PluginKind::from_metadata(
                    &self.plugin_field(index, UH_PLUGIN_KIND),
                ),
            })
            .collect()
    }

    fn plugin_field(&self, index: i32, field: i32) -> String {
        let len = unsafe { uh_plugin_info(self.raw, index, field, ptr::null_mut(), 0) };
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u8; len as usize + 1];
        unsafe {
            uh_plugin_info(
                self.raw,
                index,
                field,
                buf.as_mut_ptr() as *mut c_char,
                buf.len() as i32,
            );
        }
        buf.truncate(len as usize);
        String::from_utf8_lossy(&buf).into_owned()
    }

    pub fn create_instance(
        &self,
        plugin_index: i32,
        sample_rate: u32,
        buffer_size: u32,
        done: UhInstanceFn,
        user: *mut c_void,
    ) {
        unsafe {
            uh_instance_create(
                self.raw,
                plugin_index,
                sample_rate,
                buffer_size,
                Some(done),
                user,
            )
        }
    }

    pub fn destroy_instance(&self, instance_id: i32) {
        unsafe { uh_instance_destroy(self.raw, instance_id) }
    }

    /// Owns all processors for a serial chain. Stop its audio stream and drop
    /// this handle before saving state or destroying any of the instances.
    pub fn create_chain(
        &self,
        instrument_id: i32,
        effects: &[i32],
        bypass: bool,
        sample_rate: u32,
        max_frames: u32,
    ) -> Option<Processor> {
        let raw = unsafe {
            uh_chain_create(
                self.raw,
                instrument_id,
                effects.as_ptr(),
                effects.len() as i32,
                bypass as i32,
                sample_rate,
                max_frames,
            )
        };
        (!raw.is_null()).then_some(Processor { raw })
    }

    /// `Err` carries the UH_ERR_* code.
    pub fn show_ui(&self, instance_id: i32) -> Result<(), i32> {
        match unsafe { uh_ui_show(self.raw, instance_id) } {
            UH_OK => Ok(()),
            code => Err(code),
        }
    }

    pub fn hide_ui(&self, instance_id: i32) {
        unsafe { uh_ui_hide(self.raw, instance_id) }
    }

    pub fn ui_is_visible(&self, instance_id: i32) -> bool {
        unsafe { uh_ui_is_visible(self.raw, instance_id) != 0 }
    }

    pub fn ui_thumbnail(&self, instance_id: i32) -> Result<image::RgbaImage, String> {
        let mut rgba = vec![0; 192 * 128 * 4];
        let (mut width, mut height) = (0, 0);
        let code = unsafe {
            uh_ui_thumbnail(
                self.raw,
                instance_id,
                rgba.as_mut_ptr(),
                rgba.len() as i32,
                &mut width,
                &mut height,
            )
        };
        if code != UH_OK || !(1..=192).contains(&width) || !(1..=128).contains(&height) {
            return Err(format!("Could not capture plugin UI (code {code})"));
        }
        rgba.truncate(width as usize * height as usize * 4);
        image::RgbaImage::from_raw(width as u32, height as u32, rgba)
            .ok_or_else(|| "Invalid plugin thumbnail dimensions".into())
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        unsafe { uh_destroy(self.raw) }
    }
}

/// Owns a `UhProcessor`. Lives on the main thread; the audio thread works
/// through a `ProcessorRef`.
pub struct Processor {
    raw: *mut UhProcessor,
}

impl Processor {
    /// A handle for the audio callback.
    ///
    /// # Safety
    /// The audio stream using the returned handle must be stopped and dropped
    /// before this `Processor` is dropped, and only one thread may render at a time.
    pub unsafe fn audio_ref(&self) -> ProcessorRef {
        ProcessorRef { raw: self.raw }
    }
}

impl Drop for Processor {
    fn drop(&mut self) {
        unsafe { uh_processor_destroy(self.raw) }
    }
}

/// What the audio thread holds. See `Processor::audio_ref` for the rules.
pub struct ProcessorRef {
    raw: *mut UhProcessor,
}

// The shim allows process() on the audio thread while the main thread keeps
// using the instance, which is uapmd's own threading model.
unsafe impl Send for ProcessorRef {}

impl ProcessorRef {
    /// Renders `out.len() / channels` frames of interleaved audio, delivering
    /// `ump` at the start of the block. Returns false if rendering failed
    /// (the output is silence then).
    pub fn process(&mut self, ump: &[u32], out: &mut [f32], channels: usize) -> bool {
        if channels == 0 {
            return false;
        }
        let frames = out.len() / channels;
        unsafe {
            uh_processor_process(
                self.raw,
                ump.as_ptr(),
                ump.len() as i32,
                out.as_mut_ptr(),
                channels as i32,
                frames as i32,
            ) == 0
        }
    }
}
