//! Surge XT patch catalog for machines without cmrt's catalog cache (ADR 0025).
use std::path::{Path, PathBuf};
use std::time::Duration;

const URL: &str =
    "https://raw.githubusercontent.com/cat2151/cat-music-patterns/main/surge-xt-patch-catalog.json";
const TIMEOUT: Duration = Duration::from_secs(10);

/// Refreshes the saved catalog from [`URL`] and returns its path; falls back to the previous file.
pub(crate) fn path() -> Result<PathBuf, String> {
    let destination = crate::config::path()?
        .parent()
        .ok_or("config path has no parent")?
        .join("patch-catalog")
        .join("surge-xt.json");
    path_with(|| fetch(URL), &destination)
}

pub(crate) fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .build(),
    );
    agent
        .get(url)
        .call()
        .and_then(|mut response| response.body_mut().read_to_vec())
        .map_err(|error| format!("{url}: {error}"))
}

fn path_with(
    fetch: impl FnOnce() -> Result<Vec<u8>, String>,
    destination: &Path,
) -> Result<PathBuf, String> {
    refresh(fetch, destination, replace_if_valid)
        .map_err(|error| format!("Surge XT patch catalog: {error}"))
}

/// Saves what `replace` accepts at `destination`; on any failure the previous file stays in use.
pub(crate) fn refresh(
    fetch: impl FnOnce() -> Result<Vec<u8>, String>,
    destination: &Path,
    replace: impl FnOnce(&[u8], &Path) -> Result<(), String>,
) -> Result<PathBuf, String> {
    match fetch().and_then(|bytes| replace(&bytes, destination)) {
        Ok(()) => Ok(destination.to_path_buf()),
        Err(_) if destination.is_file() => Ok(destination.to_path_buf()),
        Err(error) => Err(error),
    }
}

/// Replaces `destination` only when `bytes` pass the same checks as cmrt's own cache.
fn replace_if_valid(bytes: &[u8], destination: &Path) -> Result<(), String> {
    let candidate = destination.with_extension("download.tmp");
    crate::state_store::atomic_write(&candidate, bytes)?;
    let checked = cmrt_tui_core::patch_catalog_cache::load_from(&candidate)
        .map(drop)
        .map_err(|error| format!("{error:#}"));
    let _ = std::fs::remove_file(&candidate);
    checked?;
    crate::state_store::atomic_write(destination, bytes)
}

#[cfg(test)]
mod tests;
