//! File-wide reference-only effects; runtime states are resolved by the session.
use crate::{
    instrument_layers,
    patch_roots::Roots,
    smf::model::{Music, Track},
};
use serde::{Deserialize, Serialize};

pub const PREFIX: &[u8] = b"\x7dcat-plugin-player\0master-patch-references\0";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effects {
    pub effects: Vec<instrument_layers::Effect>,
}

/// The owned event as layer metadata, checked without looking up this machine's roots.
fn metadata(music: &Music) -> Result<Track, String> {
    let mut metadata = Track { events: vec![] };
    for (index, track) in music.tracks.iter().enumerate() {
        for event in &track.events {
            if crate::master_effects::owned(event) {
                return Err(
                    "Unsupported state-bearing master metadata; patch references are required"
                        .into(),
                );
            }
            if event.status == 0xff && event.meta_type == 0x7f && event.data.starts_with(PREFIX) {
                if music.format != 1 || index != 0 {
                    return Err("Master effects require format 1 conductor track".into());
                }
                let mut event = event.clone();
                event.data = [instrument_layers::PREFIX, &event.data[PREFIX.len()..]].concat();
                metadata.events.push(event);
            }
        }
    }
    if !metadata.events.is_empty() && !instrument_layers::parse(&metadata)?.layers.is_empty() {
        return Err("Master metadata must not contain instrument layers".into());
    }
    Ok(metadata)
}

pub fn read(music: &Music, roots: &mut dyn Roots) -> Result<Effects, String> {
    Ok(Effects {
        effects: instrument_layers::read(&metadata(music)?, roots)?.effects,
    })
}

pub fn write(music: &mut Music, effects: &Effects, roots: &mut dyn Roots) -> Result<(), String> {
    metadata(music)?;
    if music.format != 1 || music.tracks.is_empty() {
        return Err("Master effects require format 1".into());
    }
    let settings = instrument_layers::Settings {
        version: 1,
        layers: vec![],
        effects: effects.effects.clone(),
    };
    let mut metadata = Track { events: vec![] };
    instrument_layers::write(&mut metadata, &settings, roots)?;
    let mut event = metadata.events.remove(0);
    event.data = [PREFIX, &event.data[instrument_layers::PREFIX.len()..]].concat();
    let mut events = music.tracks[0].events.clone();
    events.retain(|e| !(e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)));
    if !effects.effects.is_empty() {
        events.insert(0, event);
    }
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
    music.tracks[0].events = events;
    Ok(())
}

#[cfg(test)]
#[path = "master_references_tests.rs"]
mod tests;
