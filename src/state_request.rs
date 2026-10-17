//! Headless state generation for editor cache misses; no GUI or catalog scan.
use crate::patch_reference::{PatchReference, Source};
use std::io::{Read, Write};

pub(crate) fn catalog(
    plugin: &str,
    bundle: &std::path::Path,
    path: &std::path::Path,
) -> Result<Vec<u8>, String> {
    cmrt_patches::prepare_catalog_clap_patch_state(plugin, bundle, path).map_err(|e| e.to_string())
}

pub(crate) fn generate(reference: &PatchReference) -> Result<Vec<u8>, String> {
    reference.validate()?;
    if reference.plugin.format != "CLAP" {
        return Err("Catalog restoration currently requires CLAP".into());
    }
    match &reference.source {
        Source::Catalog { path, .. } => catalog(
            &reference.plugin.id,
            std::path::Path::new(&reference.plugin.bundle_path),
            path,
        ),
        Source::Effect { path, value } => crate::effect_preparation::restore(
            &reference.plugin.bundle_path,
            &reference.plugin.id,
            &cmrt_core::audio_effect::PresetLocation {
                path: path.clone(),
                value: value.clone(),
                display: reference.display.clone(),
            },
        ),
    }
}

pub(crate) fn run() -> Result<(), String> {
    let mut input = Vec::new();
    std::io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut input)
        .map_err(|e| e.to_string())?;
    if input.len() > 1024 * 1024 {
        return Err("State request too large".into());
    }
    let reference: PatchReference = serde_json::from_slice(&input).map_err(|e| e.to_string())?;
    let state = generate(&reference)?;
    if state.is_empty() || state.len() > 16 * 1024 * 1024 {
        return Err("Invalid generated state size".into());
    }
    std::io::stdout()
        .write_all(&state)
        .map_err(|e| e.to_string())
}
