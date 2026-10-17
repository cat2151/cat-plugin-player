//! Keep the native shim available even when cargo install deploys only the exe.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;

const DLL: &[u8] = include_bytes!(env!("UAPMD_SHIM_DLL"));

pub fn load_at(
    root: &std::path::Path,
) -> Result<libloading::Library, Box<dyn std::error::Error + Send + Sync>> {
    let mut hash = DefaultHasher::new();
    DLL.hash(&mut hash);
    let directory = root.join("native").join(format!("{:016x}", hash.finish()));
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
