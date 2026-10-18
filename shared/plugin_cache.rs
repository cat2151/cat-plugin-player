//! Shared metadata reader for fast startup, without loading plugin libraries.
use super::ffi::PluginKey;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Entry {
    pub plugin: PluginKey,
    pub kind: String,
}

pub fn path(config: &Path) -> PathBuf {
    config.with_file_name("plugin-list-cache.json")
}

pub fn load(config: &Path) -> Result<Option<Vec<Entry>>, String> {
    let bytes = match std::fs::read(path(config)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| error.to_string())
}
