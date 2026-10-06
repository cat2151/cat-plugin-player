//! Per-plugin opaque state in the local config directory.
use crate::config::PluginKey;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn path(config: &Path, key: &PluginKey) -> Result<PathBuf, String> {
    let parent = config.parent().ok_or("config path has no parent")?;
    let mut hash = Sha256::new();
    hash.update((key.format.len() as u64).to_le_bytes());
    hash.update(key.format.as_bytes());
    hash.update(key.id.as_bytes());
    Ok(parent
        .join("states")
        .join(format!("{:x}.bin", hash.finalize())))
}

pub fn load(config: &Path, key: &PluginKey) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path(config, key)?) {
        Ok(state) => Ok(Some(state)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub fn save(config: &Path, key: &PluginKey, state: &[u8]) -> Result<(), String> {
    let destination = path(config, key)?;
    atomic_write(&destination, state)
}

pub(crate) fn atomic_write(destination: &Path, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = destination.parent().ok_or("state path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = destination.with_extension(format!(
        "{}-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        replace(&temporary, destination)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|error: std::io::Error| error.to_string())
}

#[cfg(windows)]
fn replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32;
    }
    let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<_> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // Replace only after the complete new file is flushed. Never delete the old
    // state first: a failed write must leave the last usable snapshot intact.
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0x1 | 0x8) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_states_survive_replacement_and_are_isolated_by_identity() {
        let directory = std::env::temp_dir().join(format!(
            "cat-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config = directory.join("config.toml");
        let mut key = PluginKey {
            format: "CLAP".into(),
            id: "X:/音源/../synth".into(),
            name: String::new(),
            vendor: String::new(),
            bundle_path: String::new(),
        };
        assert!(load(&config, &key).unwrap().is_none());
        save(&config, &key, &[0, 255, 128, 0]).unwrap();
        key.name = "renamed".into();
        assert_eq!(load(&config, &key).unwrap(), Some(vec![0, 255, 128, 0]));
        key.format = "VST3".into();
        assert!(load(&config, &key).unwrap().is_none());
        save(&config, &key, &[42]).unwrap();
        key.format = "CLAP".into();
        save(&config, &key, &[]).unwrap();
        assert_eq!(load(&config, &key).unwrap(), Some(vec![]));
        key.format = "VST3".into();
        assert_eq!(load(&config, &key).unwrap(), Some(vec![42]));
        assert_eq!(
            std::fs::read_dir(directory.join("states")).unwrap().count(),
            2
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
