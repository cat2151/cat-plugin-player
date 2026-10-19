//! Per-app cache policy; the loader and embedded native host are shared.
#[path = "../shared/native_library.rs"]
mod embedded;
#[cfg(test)]
#[path = "native_host_parallel_tests.rs"]
mod host_parallel;
#[cfg(test)]
#[path = "native_test_process.rs"]
pub(crate) mod process;
pub fn load() -> Result<libloading::Library, Box<dyn std::error::Error + Send + Sync>> {
    let root = cache_root()?;
    #[cfg(test)]
    crate::ffi::trace::mark("DLL load begin");
    let result = embedded::load_at(&root);
    #[cfg(test)]
    crate::ffi::trace::mark("DLL load end");
    result
}

pub(crate) fn cache_root() -> Result<std::path::PathBuf, String> {
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
    let root = std::env::var_os(process::ROOT)
        .map(std::path::PathBuf::from)
        .unwrap_or(root);
    Ok(root)
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_shim_loads_and_exports_host_entry_point() {
        if super::process::isolated() {
            return;
        }
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
