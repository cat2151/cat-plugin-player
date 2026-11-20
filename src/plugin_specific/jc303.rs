//! JC303: ADR 0044. Host sync starts its step sequencer, which then ignores incoming notes.
use crate::config::PluginKey;

#[cfg(test)]
#[path = "jc303_tests.rs"]
mod tests;

const ID: &str = "co.midilab.JC303";
const HOST: &[u8] = b"1.0";
const INTERNAL: &[u8] = b"0.0";

pub(crate) fn supports(plugin: &PluginKey) -> bool {
    plugin.format == "CLAP" && plugin.id == ID
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
