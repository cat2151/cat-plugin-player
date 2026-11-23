//! JC303: ADR 0044. Host sync starts its step sequencer, which then ignores incoming notes.
use super::Controls;
use crate::{
    config::PluginKey,
    ffi::parameters::Parameter,
    sequence_modulation::{MacroTarget, MACROS},
};

#[cfg(test)]
#[path = "jc303_tests.rs"]
mod tests;

const ID: &str = "co.midilab.JC303";
const HOST: &[u8] = b"1.0";
const INTERNAL: &[u8] = b"0.0";
pub(crate) const PARAMETER_ROWS: [(&str, &str); MACROS] = [
    ("Cutoff", "Cutoff"),
    ("Resonance", "Resonance"),
    ("EnvMod", "Filt. EnvMod"),
    ("Decay", "Filt. Decay"),
    ("Accent", "Accent"),
    ("Waveform", "Waveform"),
    ("Drive", "OD Drive"),
    ("Dry/Wet", "OD Dry/Wet"),
];

pub(crate) fn supports(plugin: &PluginKey) -> bool {
    plugin.format == "CLAP" && plugin.id == ID
}

pub(crate) fn controls(parameters: &[Parameter]) -> Controls {
    let mut controls = Controls::default();
    for (index, (name, label)) in PARAMETER_ROWS.into_iter().enumerate() {
        if let Some(p) = parameters.iter().find(|p| p.eligible && p.name == name) {
            controls.targets[index] = Some(MacroTarget::Parameter);
            controls.labels[index] = label;
            controls.macros[index] = p
                .checked_current()
                .and_then(|value| p.domain()?.position(value))
                .ok()
                .map(|position| (position.get() * 127.0).round() as u8);
        }
    }
    controls
}

/// The state with `seqSyncMode` Host replaced by Internal; `None` when it is not Host.
/// Same-length values keep the size fields of JUCE's binary wrapper valid.
pub(crate) fn internal_sync(plugin: &PluginKey, state: &[u8]) -> Option<Vec<u8>> {
    if !supports(plugin) {
        return None;
    }
    let id = find(state, br#"id="seqSyncMode""#)?;
    let start = state[..id].iter().rposition(|&b| b == b'<')?;
    let end = id + state[id..].iter().position(|&b| b == b'>')?;
    let value = start + find(&state[start..end], br#"value=""#)? + br#"value=""#.len();
    if state.get(value..value + HOST.len()) != Some(HOST)
        || state.get(value + HOST.len()) != Some(&b'"')
    {
        return None;
    }
    let mut fixed = state.to_vec();
    fixed[value..value + HOST.len()].copy_from_slice(INTERNAL);
    Some(fixed)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
