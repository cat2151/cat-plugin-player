//! File-wide effects, stored separately from instrument/track chains.
use crate::{
    chain_settings::{PluginState, Settings as TrackSettings, MAX_PAYLOAD},
    smf::model::{Event, Music},
};
use serde::{Deserialize, Serialize};

pub const PREFIX: &[u8] = b"\x7dcat-plugin-player\0master-effects\0";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub version: u8,
    pub effects: Vec<PluginState>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            effects: vec![],
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported master effects schema".into());
        }
        // Reuse identity/state limits without inventing a master instrument.
        TrackSettings {
            effects: self.effects.clone(),
            instrument: None,
            ..Default::default()
        }
        .validate_effects()
    }
    pub fn label(&self) -> String {
        if self.effects.is_empty() {
            return "No master effects".into();
        }
        self.effects
            .iter()
            .map(|s| s.preset_name.as_deref().unwrap_or(&s.plugin.name))
            .collect::<Vec<_>>()
            .join(" → ")
    }
}
pub fn owned(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)
}

pub fn read(music: &Music) -> Result<Settings, String> {
    let mut found = None;
    for (track, t) in music.tracks.iter().enumerate() {
        for e in t.events.iter().filter(|e| owned(e)) {
            if music.format != 1
                || track != 0
                || e.tick != 0
                || found.is_some()
                || e.data.len() > MAX_PAYLOAD
            {
                return Err(
                    "Master effects require one tick-zero metadata event in format 1 track 0"
                        .into(),
                );
            }
            let settings: Settings =
                serde_json::from_slice(&e.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
            settings.validate()?;
            found = Some(settings);
        }
    }
    Ok(found.unwrap_or_default())
}
pub fn write(music: &mut Music, settings: &Settings) -> Result<(), String> {
    if music.format != 1 || music.tracks.is_empty() {
        return Err("Master effects editing requires SMF format 1".into());
    }
    read(music)?;
    settings.validate()?;
    let mut data = PREFIX.to_vec();
    data.extend(serde_json::to_vec(settings).map_err(|e| e.to_string())?);
    if data.len() > MAX_PAYLOAD {
        return Err("Encoded master effects metadata is too large".into());
    }
    let mut events = music.tracks[0].events.clone();
    events.retain(|e| !owned(e));
    if !settings.effects.is_empty() {
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
    }
    let mut previous = 0;
    for e in &mut events {
        let delta = e.tick.checked_sub(previous).ok_or("Events out of order")?;
        if delta > 0x0fff_ffff {
            return Err("SMF delta too large".into());
        }
        e.delta = delta as u32;
        previous = e.tick;
    }
    music.tracks[0].events = events;
    Ok(())
}

#[cfg(test)]
#[path = "master_effects_tests.rs"]
mod tests;
