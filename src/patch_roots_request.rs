//! Headless root discovery for the editor, whose SMF references are root-relative.
use crate::patch_roots::{Plugin, Table};
use std::io::Write;

/// `\\?\C:\x` as `C:\x`, so resolved paths match the ones the browser offers.
fn simplified(path: &str) -> String {
    match path.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => rest.into(),
        _ => path.into(),
    }
}

/// Built from the same caches the browsers read, so offered paths always fall under a root.
pub(crate) fn table() -> Result<Table, String> {
    let entries = crate::plugin_cache::load(&crate::config::path()?)?
        .ok_or("Plugin list has not been scanned yet")?;
    let instruments = crate::patch_catalog::roots()?;
    let effects = cmrt_core::audio_effect::builtin_effect_plugins();
    Ok(Table {
        plugins: entries
            .into_iter()
            .map(|entry| {
                let key = entry.plugin;
                let instrument = instruments
                    .iter()
                    .filter(|(id, _)| *id == key.id)
                    .flat_map(|(_, roots)| roots.iter().cloned());
                let effect = effects
                    .iter()
                    .filter(|effect| effect.plugin_id == key.id)
                    .map(|effect| effect.preset_root.to_string_lossy().into_owned());
                Plugin {
                    roots: instrument.chain(effect).map(|r| simplified(&r)).collect(),
                    format: key.format,
                    id: key.id,
                    bundle_path: key.bundle_path,
                }
            })
            .collect(),
    })
}

pub(crate) fn run() -> Result<(), String> {
    let bytes = serde_json::to_vec(&table()?).map_err(|e| e.to_string())?;
    std::io::stdout()
        .write_all(&bytes)
        .map_err(|e| e.to_string())
}
