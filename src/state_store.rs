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
    save_named(config, key, state, None)
}

#[derive(serde::Deserialize, serde::Serialize)]
struct PresetName {
    name: Option<String>,
    state_hash: String,
}

pub(crate) fn save_named(
    config: &Path,
    key: &PluginKey,
    state: &[u8],
    name: Option<&str>,
) -> Result<(), String> {
    let destination = path(config, key)?;
    atomic_write(&destination, state)?;
    let metadata_path = destination.with_extension("preset.json");
    if name.is_none() {
        return match std::fs::remove_file(metadata_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    // Bind the label to these exact bytes. A failed second write must never
    // associate metadata from the previous state with a different snapshot.
    let metadata = PresetName {
        name: name.map(str::to_owned),
        state_hash: format!("{:x}", Sha256::digest(state)),
    };
    let data = serde_json::to_vec(&metadata).map_err(|e| e.to_string())?;
    atomic_write(&metadata_path, &data)
}

pub(crate) fn load_preset_name(config: &Path, key: &PluginKey) -> Option<String> {
    let destination = path(config, key).ok()?;
    let data = std::fs::read(destination.with_extension("preset.json")).ok()?;
    let metadata: PresetName = serde_json::from_slice(&data).ok()?;
    let state = std::fs::read(destination).ok()?;
    (metadata.state_hash == format!("{:x}", Sha256::digest(&state)))
        .then_some(metadata.name)
        .flatten()
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

    #[test]
    fn preset_names_require_matching_state_and_unknown_saves_clear_them() {
        let directory =
            std::env::temp_dir().join(format!("cat-preset-name-{}", std::process::id()));
        let config = directory.join("config.toml");
        let key = PluginKey {
            format: "CLAP".into(),
            id: "synth".into(),
            name: "Synth".into(),
            vendor: String::new(),
            bundle_path: String::new(),
        };
        assert_eq!(load_preset_name(&config, &key), None);
        save_named(&config, &key, &[1], Some("日本語 Pad")).unwrap();
        assert_eq!(
            load_preset_name(&config, &key).as_deref(),
            Some("日本語 Pad")
        );
        // Simulate a state replacement without updating its metadata.
        atomic_write(&path(&config, &key).unwrap(), &[2]).unwrap();
        assert_eq!(load_preset_name(&config, &key), None);
        save_named(&config, &key, &[1], Some("Other")).unwrap();
        save(&config, &key, &[1]).unwrap();
        assert_eq!(load_preset_name(&config, &key), None);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
