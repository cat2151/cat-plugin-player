//! New-container defaults use the browser's candidates, metadata and Space load gate.
use crate::{
    patch_catalog::Candidate,
    patch_reference::{PatchReference, Source},
};
use std::io::Write;

const DEFAULT: &str = "Free Sounds/Programs/DSF/Mind Game.sfz";
const PLUGIN: &str = "com.Plogue Art et Technologie, Inc.sforzando";

fn choose(candidates: &[Candidate], exists: impl Fn(&Candidate) -> bool) -> Option<&Candidate> {
    let usable = |c: &&Candidate| {
        c.measurement.drum_kit
            && crate::drum_browser::notes(c).is_some_and(|n| !n.is_empty())
            && exists(c)
    };
    candidates
        .iter()
        .filter(usable)
        .find(|c| c.plugin_id == PLUGIN && c.display == DEFAULT)
        .or_else(|| {
            candidates
                .iter()
                .filter(usable)
                .find(|c| !c.measurement.is_heavy_offline_load())
        })
}

pub(crate) fn run() -> Result<(), String> {
    let candidates = crate::patch_catalog::load()?;
    let candidate = choose(&candidates, |c| c.path.is_file() && c.bundle_path.exists())
        .ok_or("No default drum kit is available")?;
    let kit = crate::drum_browser::kit_metadata(candidate, 1)?;
    let reference = PatchReference {
        plugin: crate::host::PluginKey {
            format: candidate.format.clone(),
            id: candidate.plugin_id.clone(),
            name: candidate.plugin_name.clone(),
            vendor: String::new(),
            bundle_path: candidate.bundle_path.to_string_lossy().into(),
        },
        source: Source::Catalog {
            path: candidate.path.clone(),
            file: candidate.path.clone(),
        },
        display: candidate.display.clone(),
    };
    let bytes = serde_json::to_vec(&(reference, kit)).map_err(|e| e.to_string())?;
    std::io::stdout()
        .write_all(&bytes)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
