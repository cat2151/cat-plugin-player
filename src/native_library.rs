//! Per-app cache policy; the loader and embedded native host are shared.
#[path = "../shared/native_library.rs"]
mod embedded;
pub fn load() -> Result<libloading::Library, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "recording-mode")]
    let root = crate::recording::config_path()?
        .parent()
        .ok_or("recording config has no parent")?
        .to_path_buf();
    #[cfg(not(feature = "recording-mode"))]
    let root = dirs::cache_dir()
        .ok_or("cache directory unavailable")?
        .join("cat-plugin-player");
    #[cfg(test)]
    crate::ffi::trace::mark("DLL load begin");
    let result = embedded::load_at(&root);
    #[cfg(test)]
    crate::ffi::trace::mark("DLL load end");
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_shim_loads_and_exports_host_entry_point() {
        let library = super::load().expect("embedded shim should load");
        extern "C" {
            fn uh_plugin_count(host: *mut crate::ffi::UhHost) -> i32;
        }
        // Resolve without creating a host, scanning plugins, or opening audio.
        unsafe {
            library
                .get::<unsafe extern "C" fn()>(b"uh_create\0")
                .expect("shim should export uh_create");
            // The shim explicitly accepts null here. Exercise the delay import
            // as well as LoadLibrary, without initializing the plugin host.
            assert_eq!(uh_plugin_count(std::ptr::null_mut()), 0);
        }
    }
}
