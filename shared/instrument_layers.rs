//! Reference-only track metadata. Resolved states belong to the audio session.
use crate::{
    patch_reference::PatchReference,
    patch_roots::{self, Roots},
    smf::model::{Event, Track},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const PREFIX: &[u8] = b"\x7dcat-plugin-player\0instrument-layers\0";
pub const MAX_PAYLOAD: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub id: u64,
    /// None is the existing default instrument, not a failed reference fallback.
    pub patch: Option<PatchReference>,
    pub octave: i32,
    pub volume_db: f32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mute: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub solo: bool,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            id: 1,
            patch: None,
            octave: 0,
            volume_db: 0.0,
            mute: false,
            solo: false,
        }
    }
}

impl Layer {
    pub fn gain(&self) -> Result<f32, String> {
        let gain = 10.0f32.powf(self.volume_db / 20.0);
        if !self.volume_db.is_finite() || !gain.is_finite() {
            return Err("Layer volume must have a finite amplitude".into());
        }
        Ok(gain)
    }

    pub fn pitch(&self, pitch: u8) -> Option<u8> {
        let shifted = i64::from(pitch) + 12 * i64::from(self.octave);
        (0..=127).contains(&shifted).then_some(shifted as u8)
    }
}

/// One post-sum effect in chain order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub patch: PatchReference,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bypass: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub version: u8,
    pub layers: Vec<Layer>,
    pub effects: Vec<Effect>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            layers: vec![Layer::default()],
            effects: vec![],
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with(PatchReference::validate)
    }

    /// The SMF form, whose paths are relative to this machine's roots.
    pub fn validate_portable(&self) -> Result<(), String> {
        self.validate_with(patch_roots::validate_portable)
    }

    fn validate_with(
        &self,
        patch: impl Fn(&PatchReference) -> Result<(), String>,
    ) -> Result<(), String> {
        if self.version != 1 || self.layers.len() > 64 || self.effects.len() > 32 {
            return Err("Unsupported layer schema or too many slots".into());
        }
        let mut ids = HashSet::new();
        for layer in &self.layers {
            if layer.id == 0 || !ids.insert(layer.id) {
                return Err("Layer IDs must be nonzero and unique".into());
            }
            layer.gain()?;
            if let Some(reference) = &layer.patch {
                patch(reference)?;
            }
        }
        for effect in &self.effects {
            patch(&effect.patch)?;
        }
        Ok(())
    }

    fn map_patches(
        mut self,
        mut map: impl FnMut(&PatchReference) -> Result<PatchReference, String>,
    ) -> Result<Self, String> {
        for patch in self.layers.iter_mut().filter_map(|l| l.patch.as_mut()) {
            *patch = map(patch)?;
        }
        for effect in &mut self.effects {
            effect.patch = map(&effect.patch)?;
        }
        Ok(self)
    }
}

pub fn owned(event: &Event) -> bool {
    event.status == 0xff && event.meta_type == 0x7f && event.data.starts_with(PREFIX)
}

/// The SMF form, checked without looking up this machine's roots.
pub fn parse(track: &Track) -> Result<Settings, String> {
    if track.events.iter().any(|e| {
        e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(crate::chain_settings::PREFIX)
    }) {
        return Err(
            "Unsupported state-bearing chain metadata; reference-only layer metadata is required"
                .into(),
        );
    }
    let mut found = None;
    for event in track.events.iter().filter(|e| owned(e)) {
        if found.is_some() || event.tick != 0 || event.data.len() > MAX_PAYLOAD {
            return Err("Duplicate, misplaced or oversized layer metadata".into());
        }
        let settings: Settings =
            serde_json::from_slice(&event.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
        settings.validate_portable()?;
        found = Some(settings);
    }
    Ok(found.unwrap_or_default())
}

pub fn read(track: &Track, roots: &mut dyn Roots) -> Result<Settings, String> {
    let settings = parse(track)?.map_patches(|p| patch_roots::resolve(p, roots))?;
    settings.validate()?;
    Ok(settings)
}

pub fn write(track: &mut Track, settings: &Settings, roots: &mut dyn Roots) -> Result<(), String> {
    // Never silently repair a corrupt or unknown owned event.
    parse(track)?;
    settings.validate()?;
    let portable = settings
        .clone()
        .map_patches(|p| patch_roots::portable(p, roots))?;
    let mut data = PREFIX.to_vec();
    data.extend(serde_json::to_vec(&portable).map_err(|e| e.to_string())?);
    if data.len() > MAX_PAYLOAD {
        return Err("Layer metadata too large".into());
    }
    let mut events: Vec<_> = track.events.iter().filter(|e| !owned(e)).cloned().collect();
    events.insert(
        0,
        Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x7f,
            data,
        },
    );
    let mut previous = 0;
    for event in &mut events {
        let delta = event
            .tick
            .checked_sub(previous)
            .ok_or("Events out of order")?;
        if delta > 0x0fff_ffff {
            return Err("SMF delta too large".into());
        }
        event.delta = delta as u32;
        previous = event.tick;
    }
    track.events = events;
    Ok(())
}

#[cfg(test)]
#[path = "instrument_layers_tests.rs"]
mod tests;
