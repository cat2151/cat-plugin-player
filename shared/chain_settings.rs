//! Shared SMF settings. Display names never determine restoration.
use crate::{
    host::PluginKey,
    smf::model::{Event, Track},
};
use serde::{Deserialize, Serialize};

pub const PREFIX: &[u8] = b"\x7dcat-plugin-player\0chain\0";
pub const MAX_PAYLOAD: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginState {
    #[serde(default)]
    pub reference: Option<crate::patch_reference::PatchReference>,
    pub plugin: PluginKey,
    pub state: Vec<u8>,
    pub preset_name: Option<String>,
    /// Effects only: the instance stays and passes its input through.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bypass: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub version: u8,
    pub instrument: Option<PluginState>,
    pub effects: Vec<PluginState>,
    pub bypass: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            instrument: None,
            effects: Vec::new(),
            bypass: false,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.effects.len() > 32 {
            return Err("Unsupported chain schema or too many effects".into());
        }
        if self.instrument.is_none() && (!self.effects.is_empty() || self.bypass) {
            return Err("Effects require an instrument".into());
        }
        self.validate_effects()
    }

    pub(crate) fn validate_effects(&self) -> Result<(), String> {
        if self.effects.len() > 32 {
            return Err("Too many effects".into());
        }
        let mut total = 0usize;
        for slot in self.instrument.iter().chain(&self.effects) {
            let key = &slot.plugin;
            if !matches!(key.format.as_str(), "CLAP" | "VST3")
                || key.id.is_empty()
                || [&key.id, &key.name, &key.vendor, &key.bundle_path]
                    .iter()
                    .any(|s| s.contains('\0'))
                || slot.state.is_empty()
                || slot.state.len() > 16 * 1024 * 1024
                || slot
                    .preset_name
                    .as_ref()
                    .is_some_and(|s| s.len() > 4096 || s.contains('\0'))
            {
                return Err("Invalid plugin identity or state".into());
            }
            total += slot.state.len();
        }
        if total > 32 * 1024 * 1024 {
            return Err("Chain state is too large".into());
        }
        Ok(())
    }

    /// State validation for a track's post-sum effect chain without an instrument.
    #[allow(dead_code)] // Used by the editor runtime layer model.
    pub fn validate_effect_states(&self) -> Result<(), String> {
        self.validate_effects()
    }

    /// Whether a chain opened from `self` can take `other` by reloading state alone.
    /// `self` without an instrument was opened with the default instrument. `other` needs an
    /// instrument: there is no stored state that returns a plugin to the default sound.
    #[allow(dead_code)] // Used by the editor and shared native contract tests.
    pub fn same_plugins(&self, other: &Self) -> bool {
        let Some(new) = &other.instrument else {
            return false;
        };
        let default;
        let old = match &self.instrument {
            Some(slot) => &slot.plugin,
            None => match crate::chain::default_instrument() {
                Ok(key) => {
                    default = key;
                    &default
                }
                Err(_) => return false,
            },
        };
        self.effects.len() == other.effects.len()
            && same_plugin(old, &new.plugin)
            && (self.effects.iter().zip(&other.effects))
                .all(|(a, b)| same_plugin(&a.plugin, &b.plugin))
    }

    pub fn label(&self) -> &str {
        self.instrument.as_ref().map_or("Surge XT / Init Saw", |s| {
            s.preset_name.as_deref().unwrap_or(&s.plugin.name)
        })
    }
}

/// Display names are ignored, and bundle paths compare as the same file.
fn same_plugin(a: &PluginKey, b: &PluginKey) -> bool {
    a.format == b.format
        && a.id == b.id
        && cmrt_core::path_identity::same_path(&a.bundle_path, &b.bundle_path)
}

pub fn read(track: &Track) -> Result<Settings, String> {
    let mut found = None;
    for e in &track.events {
        if e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX) {
            if found.is_some() || e.tick != 0 || e.data.len() > MAX_PAYLOAD {
                return Err("Duplicate, misplaced or oversized chain metadata".into());
            }
            let settings: Settings =
                serde_json::from_slice(&e.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
            settings.validate()?;
            found = Some(settings);
        }
    }
    Ok(found.unwrap_or_default())
}

pub fn write(track: &mut Track, settings: &Settings) -> Result<(), String> {
    settings.validate()?;
    let mut payload = PREFIX.to_vec();
    payload.extend(serde_json::to_vec(settings).map_err(|e| e.to_string())?);
    if payload.len() > MAX_PAYLOAD {
        return Err("Encoded chain metadata is too large".into());
    }
    let mut events: Vec<_> = track
        .events
        .iter()
        .filter(|e| !(e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)))
        .cloned()
        .collect();
    events.insert(
        0,
        Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x7f,
            data: payload,
        },
    );
    let mut previous = 0;
    for e in &mut events {
        let delta = e.tick.checked_sub(previous).ok_or("Events out of order")?;
        if delta > 0x0fff_ffff {
            return Err("SMF delta too large".into());
        }
        e.delta = delta as u32;
        previous = e.tick;
    }
    track.events = events;
    Ok(())
}

#[cfg(test)]
#[path = "chain_settings_tests.rs"]
mod tests;
