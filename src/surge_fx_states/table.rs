//! The prepared Surge XT Effects state table shared by the app and its exporter (ADR 0028).
//!
//! Self-contained so `examples/export_surge_fx_states.rs` can include it by path.
use cmrt_core::audio_effect::PresetLocation;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const PLUGIN_ID: &str = "org.surge-synth-team.surge-xt-fx";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Table {
    pub format_version: u32,
    pub plugin_id: String,
    /// Keyed by the lowercase SHA-256 of the `.srgfx` file.
    pub states: BTreeMap<String, Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    /// Relative to the preset root; for people reading the file.
    pub path: String,
    /// Hex of the plugin state bytes.
    pub state: String,
}

impl Table {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let table: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if table.format_version != FORMAT_VERSION || table.plugin_id != PLUGIN_ID {
            return Err(format!(
                "unsupported table: format_version {}, plugin_id {}",
                table.format_version, table.plugin_id
            ));
        }
        if let Some((hash, _)) = table
            .states
            .iter()
            .find(|(_, entry)| decode_hex(&entry.state).is_none())
        {
            return Err(format!("invalid state hex for {hash}"));
        }
        Ok(table)
    }

    pub fn state(&self, preset_bytes: &[u8]) -> Option<Vec<u8>> {
        decode_hex(&self.states.get(&sha256_hex(preset_bytes))?.state)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    encode_hex(&Sha256::digest(bytes))
}

pub fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(text.get(index..index + 2)?, 16).ok())
        .collect()
}

/// Load `preset` into an owned temporary instance and save its state.
/// The instance is created and destroyed on the calling thread.
pub fn prepare_state(
    bundle: &str,
    plugin_id: &str,
    preset: &PresetLocation,
    sample_rate: f64,
    max_frames: usize,
) -> Result<Vec<u8>, String> {
    let action = || {
        let entry = cmrt_core::load_entry(bundle)?;
        let mut renderer =
            cmrt_core::effect::EffectRenderer::new(&entry, plugin_id, sample_rate, max_frames)?;
        renderer.load_preset(preset)?;
        renderer.save_state()
    };
    action().map_err(|error| format!("{error:#}"))
}
