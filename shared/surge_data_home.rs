//! Avoid Surge's per-instance browser scans; keep Templates and restore sound from state.
//! Based on clap-mml-play-server/core-lib/src/surge_data.rs (cat-smf-editor ADR 0012).
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

#[cfg(test)]
#[path = "surge_data_home/tests.rs"]
mod tests;
#[path = "surge_data_home/tree.rs"]
mod tree;

const ENV: &str = "SURGE_DATA_HOME";
const MARKER: &str = "cat-smf-source-data-home-v2.txt";
const OMITTED: &[&str] = &["fx_presets", "wavetables", "wavetables_3rdparty"];
static CHILD_ENV: OnceLock<Option<OsString>> = OnceLock::new();

/// # Safety
/// Call before loading plugins, GUI libraries, or starting any threads.
/// Only this process is configured; no user/system environment is changed.
pub unsafe fn before_threads(cache: &Path) -> Result<Option<PathBuf>, String> {
    let original = std::env::var_os(ENV);
    let minimal = match resolve(cache)? {
        None => return Ok(None),
        Some(Home::Inherited(source)) => return Ok(Some(source)),
        Some(Home::Built(minimal)) => minimal,
    };
    CHILD_ENV
        .set(original)
        .map_err(|_| "Surge data home already configured".to_owned())?;
    // SAFETY: delegated to the caller; this is before library initialization/threads.
    unsafe {
        std::env::set_var(ENV, &minimal);
    }
    Ok(Some(minimal))
}

/// The minimal data home for the current `SURGE_DATA_HOME`, built under `cache`
/// if needed. The environment is left unchanged; `None` means use the normal home.
pub fn minimal_path(cache: &Path) -> Result<Option<PathBuf>, String> {
    Ok(resolve(cache)?.map(|home| match home {
        Home::Inherited(path) | Home::Built(path) => path,
    }))
}

enum Home {
    /// The current data home is already a minimal tree.
    Inherited(PathBuf),
    Built(PathBuf),
}

fn resolve(cache: &Path) -> Result<Option<Home>, String> {
    if std::env::var("CMRT_SURGE_MINIMAL_DATA_HOME")
        .ok()
        .is_some_and(|v| matches!(v.trim(), "0" | "false" | "no"))
    {
        return Ok(None);
    }
    let source = std::env::var_os(ENV).map(PathBuf::from).or_else(|| {
        #[cfg(windows)]
        {
            std::env::var_os("ProgramData").map(|p| PathBuf::from(p).join("Surge XT"))
        }
        #[cfg(not(windows))]
        {
            None
        }
    });
    let Some(source) = source else {
        return Ok(None);
    };
    if !source.is_dir() {
        return Ok(None);
    }
    // Respect an inherited minimal tree, without recursively minimizing it.
    for marker in [
        MARKER,
        "cat-smf-source-data-home.txt",
        "cmrt-source-data-home.txt",
    ] {
        if let Ok(recorded) = std::fs::read_to_string(source.join(marker)) {
            let recorded = PathBuf::from(recorded.trim());
            tree::validate(&recorded, &source, marker, marker == MARKER)?;
            return Ok(Some(Home::Inherited(source)));
        }
    }
    prepare(&source, cache).map(|minimal| Some(Home::Built(minimal)))
}

/// The resident browser and headless state/root discovery need the full catalog.
pub fn restore_for_child(command: &mut Command) {
    if let Some(original) = CHILD_ENV.get() {
        child_environment(command, original.as_deref());
    }
}

fn child_environment(command: &mut Command, original: Option<&std::ffi::OsStr>) {
    match original {
        Some(value) => {
            command.env(ENV, value);
        }
        None => {
            command.env_remove(ENV);
        }
    }
}

fn prepare(source: &Path, cache: &Path) -> Result<PathBuf, String> {
    let source = std::fs::canonicalize(source).map_err(|e| e.to_string())?;
    tree::templates(&source)?;
    if !source.join("wavetables").is_dir() {
        return Err("Surge wavetables missing".into());
    }
    let entries = tree::shared_entries(&source)?;
    let mut fingerprint = source.to_string_lossy().as_bytes().to_vec();
    for entry in &entries {
        fingerprint.extend(entry.to_string_lossy().as_bytes());
        let path = source.join(entry);
        if path.is_file() {
            fingerprint.extend(std::fs::read(&path).map_err(|e| e.to_string())?);
        }
    }
    let key = format!("{:x}", Sha256::digest(fingerprint));
    let root = cache.join("surge-data-min-v2");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let minimal = root.join(key);
    if minimal.exists() {
        tree::validate(&source, &minimal, MARKER, true)?;
        return Ok(minimal);
    }
    tree::build(&source, &root, &minimal, &entries)?;
    Ok(minimal)
}
