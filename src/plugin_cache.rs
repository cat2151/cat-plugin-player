//! Complete catalog metadata; loading never opens plugin libraries.
use crate::ffi;
use crate::{config::PluginKey, ffi::PluginInfo, plugin_list::PluginKind};
use std::path::Path;
#[path = "../shared/plugin_cache.rs"]
mod metadata;
use metadata::path;
pub(crate) use metadata::{load, Entry};

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
