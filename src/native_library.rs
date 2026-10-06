//! Keep the native shim available even when cargo install deploys only the exe.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;

const DLL: &[u8] = include_bytes!(env!("UAPMD_SHIM_DLL"));

pub fn load() -> Result<libloading::Library, Box<dyn std::error::Error + Send + Sync>> {
    let mut hash = DefaultHasher::new();
    DLL.hash(&mut hash);
    let directory = dirs::cache_dir()
        .ok_or("キャッシュディレクトリを取得できません")?
        .join("cat-plugin-player")
        .join("native")
        .join(format!("{:016x}", hash.finish()));
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("uapmd_shim.dll");
    if !path.is_file() {
        // Publish only complete DLLs; simultaneous app launches may race here.
        let temporary = directory.join(format!("{}.tmp", std::process::id()));
        let result = (|| -> std::io::Result<()> {
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(DLL)?;
            drop(file);
            match std::fs::rename(&temporary, &path) {
                Ok(()) => Ok(()),
                Err(_) if path.is_file() => Ok(()),
                Err(error) => Err(error),
            }
        })();
        let _ = std::fs::remove_file(&temporary);
        result?;
    }
    // SAFETY: these are the exact shim bytes against which this exe was linked.
    // The caller retains the library until all native hosts have been destroyed.
    Ok(unsafe { libloading::Library::new(&path)? })
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
