//! Complete catalog metadata; loading never opens plugin libraries.
use crate::{config::PluginKey, ffi::PluginInfo, plugin_list::PluginKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct Entry {
    pub plugin: PluginKey,
    pub kind: String,
}

fn path(config: &Path) -> PathBuf {
    config.with_file_name("plugin-list-cache.json")
}

pub(crate) fn entries(plugins: &[PluginInfo]) -> Vec<Entry> {
    let mut entries: Vec<_> = plugins
        .iter()
        .map(|plugin| Entry {
            plugin: PluginKey::from_plugin(plugin),
            kind: match plugin.kind {
                PluginKind::Instrument => "instrument",
                PluginKind::Effect => "effect",
                PluginKind::Unknown => "unknown",
            }
            .into(),
        })
        .collect();
    entries.sort_by(|a, b| (&a.plugin.format, &a.plugin.id).cmp(&(&b.plugin.format, &b.plugin.id)));
    entries
}

pub(crate) fn load(config: &Path) -> Result<Option<Vec<Entry>>, String> {
    let bytes = match std::fs::read(path(config)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub(crate) fn save(config: &Path, plugins: &[PluginInfo]) -> Result<bool, String> {
    let entries = entries(plugins);
    if load(config).ok().flatten().as_ref() == Some(&entries) {
        return Ok(false);
    }
    let bytes = serde_json::to_vec_pretty(&entries).map_err(|error| error.to_string())?;
    crate::state_store::atomic_write(&path(config), &bytes)?;
    Ok(true)
}

#[cfg(test)]
mod tests;
