//! Macro rows of the instruments whose macros move by their own parameters, not CCs.
use super::Controls;
use crate::config::PluginKey;
use crate::instrument_macros::Instrument;
use crate::sequence_modulation::{MacroTarget, MACROS};

#[cfg(test)]
#[path = "parameter_macro_controls_tests.rs"]
mod tests;

const CUSTOM_MODULATORS: [&str; MACROS] = [
    "Custom modulator 1",
    "Custom modulator 2",
    "Custom modulator 3",
    "Custom modulator 4",
    "",
    "",
    "",
    "",
];
const MACRO_LABELS: [&str; MACROS] = [
    "Macro 1", "Macro 2", "Macro 3", "Macro 4", "Macro 5", "Macro 6", "Macro 7", "Macro 8",
];

pub(super) fn controls(plugin: &PluginKey, state: &[u8]) -> Controls {
    let Some(instrument) = super::parameter_macros(plugin) else {
        return Controls::default();
    };
    let Some(macros) = instrument.movable(&plugin.format, state) else {
        return Controls::default();
    };
    let mut controls = Controls {
        labels: match instrument {
            Instrument::Vaporizer2 => CUSTOM_MODULATORS,
            _ => MACRO_LABELS,
        },
        ..Controls::default()
    };
    for m in macros {
        let Some(i) = m.number.checked_sub(1).filter(|i| *i < MACROS) else {
            continue;
        };
        controls.targets[i] = Some(MacroTarget::Parameter);
        // The rows map 0-127 linearly onto the parameter's range, so does the shown value.
        controls.macros[i] = m.position.map(|p| (p * 127.0).round() as u8);
        controls.names[i] = m.name;
    }
    controls
}
