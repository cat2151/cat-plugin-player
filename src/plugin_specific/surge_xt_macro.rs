//! Player CC adapter for the shared Surge XT routing detector.
use super::Controls;
use crate::config::PluginKey;
#[cfg(test)]
use crate::sequence_modulation::MACROS;
use crate::surge_macros as shared;
#[cfg(test)]
use shared::parts;
#[cfg(test)]
#[path = "surge_xt_macro_tests.rs"]
mod tests;
pub(super) fn supports(plugin: &PluginKey) -> bool {
    shared::supports(&plugin.format, &plugin.id)
}
pub(super) fn controls(plugin: &PluginKey, state: &[u8]) -> Controls {
    if !supports(plugin) {
        return Controls::default();
    }
    let Some(c) = shared::parse(state, plugin.format == "VST3") else {
        return Controls::default();
    };
    Controls {
        targets: c
            .ccs
            .map(|cc| cc.map(crate::sequence_modulation::MacroTarget::Cc)),
        labels: c.labels,
        wheel: c.wheel,
        macros: c.macros,
        names: c.names,
    }
}
#[cfg(test)]
fn macro_ccs(plugin: &PluginKey, state: &[u8]) -> [Option<u8>; MACROS] {
    controls(plugin, state).ccs()
}
