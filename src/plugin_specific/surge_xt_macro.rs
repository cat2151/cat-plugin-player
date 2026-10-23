//! Player CC adapter for the shared Surge XT routing detector.
use super::Controls;
use crate::config::PluginKey;
#[cfg(test)]
use crate::sequence_modulation::MACROS;
#[cfg(test)]
use shared::parts;
#[path = "../../shared/surge_macros.rs"]
#[allow(dead_code)]
mod shared;
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
        ccs: c.ccs,
        labels: c.labels,
        wheel: c.wheel,
        macros: c.macros,
        names: c.names,
    }
}
#[cfg(test)]
fn macro_ccs(plugin: &PluginKey, state: &[u8]) -> [Option<u8>; MACROS] {
    controls(plugin, state).ccs
}
