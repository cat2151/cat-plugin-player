//! Floe 2.0.2 / state v30: exclude the macro values and the preset-dirty flag.
use crate::config::PluginKey;
use crate::instrument_macros::floe;

#[cfg(test)]
#[path = "floe_tests.rs"]
mod tests;

// ADR 0018: provisional settling time; DSP continues before the first note.
pub(super) fn automatic_note_start_delay(plugin: &PluginKey) -> std::time::Duration {
    if plugin.format == "CLAP" && plugin.id == floe::ID {
        std::time::Duration::from_millis(100)
    } else {
        std::time::Duration::ZERO
    }
}

/// Byte offsets of the four macro values and of the dirty flag, each value finite 0-1.
// All other bytes, including CC mappings and macro destinations, must match.
fn exclusions(state: &[u8]) -> Option<(Vec<usize>, usize)> {
    let s = floe::parse(state)?;
    let mut macros = Vec::with_capacity(floe::MACROS);
    for &(id, at) in &s.parameters {
        if (floe::FIRST_MACRO..floe::FIRST_MACRO + floe::MACROS as u32).contains(&id) {
            if !(0.0..=1.0).contains(&s.value(state, id)?) {
                return None;
            }
            macros.push(at);
        }
    }
    Some((macros, s.dirty))
}

pub(crate) fn same_sound(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    if plugin.format != "CLAP" || plugin.id != floe::ID || left.len() != right.len() {
        return false;
    }
    let (Some((lm, ld)), Some((rm, rd))) = (exclusions(left), exclusions(right)) else {
        return false;
    };
    if lm != rm || ld != rd {
        return false;
    }
    let ignored = |i: usize| i == ld || lm.iter().any(|&at| (at..at + 4).contains(&i));
    left.iter()
        .zip(right)
        .enumerate()
        .all(|(i, (a, b))| a == b || ignored(i))
}
