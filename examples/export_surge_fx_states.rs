//! Write the prepared Surge XT Effects state table for cat-music-patterns (ADR 0028).
//!
//! Usage: cargo run --release --example export_surge_fx_states -- <output.json>
// The reading half of the table is used only by the app.
#[allow(dead_code)]
#[path = "../src/surge_fx_states/table.rs"]
mod table;

use cmrt_core::{audio_effect::PresetLocation, AudioEffectCatalog};

/// The state does not depend on it; any rate the plugin accepts will do.
const SAMPLE_RATE: f64 = 48_000.0;
const MAX_FRAMES: usize = 1024;

fn main() -> Result<(), String> {
    let output = std::env::args()
        .nth(1)
        .ok_or("usage: export_surge_fx_states <output.json>")?;
    let catalog = AudioEffectCatalog::scan(
        cmrt_core::audio_effect::builtin_effect_plugins()
            .into_iter()
            .filter(|plugin| plugin.plugin_id == table::PLUGIN_ID)
            .collect(),
    );
    let mut out = table::Table {
        format_version: table::FORMAT_VERSION,
        plugin_id: table::PLUGIN_ID.into(),
        ..Default::default()
    };
    let mut failed = 0;
    for preset in catalog.presets() {
        let plugin = catalog
            .plugin(&preset.plugin)
            .map_err(|error| error.to_string())?;
        let bytes = std::fs::read(&preset.path)
            .map_err(|error| format!("{}: {error}", preset.path.display()))?;
        let location = PresetLocation {
            path: preset.path.clone(),
            value: preset.value.clone(),
            display: preset.display.clone(),
        };
        match table::prepare_state(
            &plugin.plugin_path,
            &plugin.plugin_id,
            &location,
            SAMPLE_RATE,
            MAX_FRAMES,
        ) {
            Ok(state) => {
                out.states.insert(
                    table::sha256_hex(&bytes),
                    table::Entry {
                        path: preset.value.clone(),
                        state: table::encode_hex(&state),
                    },
                );
            }
            Err(error) => {
                failed += 1;
                eprintln!("skipped {}: {error}", preset.value);
            }
        }
    }
    if out.states.is_empty() {
        return Err("no Surge XT Effects preset was prepared".into());
    }
    let json = serde_json::to_string_pretty(&out).map_err(|error| error.to_string())?;
    std::fs::write(&output, json).map_err(|error| format!("{output}: {error}"))?;
    eprintln!(
        "wrote {} states to {output} ({failed} skipped)",
        out.states.len()
    );
    Ok(())
}
