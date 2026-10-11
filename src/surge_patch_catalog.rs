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
    path_with(fetch, &destination)
}

fn fetch() -> Result<Vec<u8>, String> {
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .build(),
    );
    agent
        .get(URL)
        .call()
        .and_then(|mut response| response.body_mut().read_to_vec())
        .map_err(|error| format!("{URL}: {error}"))
}

fn path_with(
    fetch: impl FnOnce() -> Result<Vec<u8>, String>,
    destination: &Path,
) -> Result<PathBuf, String> {
    let refreshed = fetch().and_then(|bytes| replace_if_valid(&bytes, destination));
    match refreshed {
        Ok(()) => Ok(destination.to_path_buf()),
        Err(_) if destination.is_file() => Ok(destination.to_path_buf()),
        Err(error) => Err(format!("Surge XT patch catalog: {error}")),
    }
}

/// Replaces `destination` only when `bytes` pass the same checks as cmrt's own cache.
fn replace_if_valid(bytes: &[u8], destination: &Path) -> Result<(), String> {
    let candidate = destination.with_extension("download.tmp");
    crate::state_store::atomic_write(&candidate, bytes)?;
    let checked = clap_mml_render_tui::patch_catalog_cache::load_from(&candidate)
        .map(drop)
        .map_err(|error| format!("{error:#}"));
    let _ = std::fs::remove_file(&candidate);
    checked?;
    crate::state_store::atomic_write(destination, bytes)
}

#[cfg(test)]
mod tests;
