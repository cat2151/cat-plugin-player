//! All production exceptions tied to a particular plugin live here.
//! See docs/adr/0002-plugin-specific-favorite-state-comparison.md for the decision and limits.
use crate::config::PluginKey;

mod floe;
#[cfg(test)]
mod instrument_macros_saved_tests;
#[cfg(test)]
mod macro_parameter_lane_native_tests;
#[cfg(test)]
mod macro_value_native_tests;
mod parameter_macro_controls;
mod patch_settling;
mod pitch_tracking;
mod sforzando;
mod six_sines;
pub(crate) mod surge_start_delay;
mod surge_xt;
mod surge_xt_macro;
mod tyrell_n6;
mod tyrell_n6_control;
mod vaporizer2;

pub(crate) use surge_xt::StartupDataHome;

pub(crate) fn apply_measured_patch_categories(candidates: &mut [crate::patch_catalog::Candidate]) {
    pitch_tracking::apply(candidates);
}

/// Surge XT scans its browser trees per instance; the restored one skips them.
pub(crate) fn startup_data_home(
    plugin: &PluginKey,
    cache: &std::path::Path,
) -> Result<Option<StartupDataHome>, String> {
    surge_xt::startup_data_home(plugin, cache)
}

pub(crate) fn automatic_note_start_delay(plugin: &PluginKey) -> std::time::Duration {
    surge_xt::automatic_note_start_delay(plugin).max(floe::automatic_note_start_delay(plugin))
}

pub(crate) fn automatic_note_start_delay_at_rate(
    plugin: &PluginKey,
    sample_rate: u32,
) -> std::time::Duration {
    automatic_note_start_delay(plugin).max(patch_settling::delay(plugin, sample_rate))
}

pub(crate) fn same_favorite_state(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    left == right
        || tyrell_n6::same_sound(plugin, left, right)
        || vaporizer2::same_sound(plugin, left, right)
        || sforzando::same_sound(plugin, left, right)
        || surge_xt::same_sound(plugin, left, right)
        || six_sines::same_sound(plugin, left, right)
        || floe::same_sound(plugin, left, right)
}

/// ADR 0036 / 0038: what an instrument state says about the macros its rows move.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Controls {
    /// Per macro, where a routed macro's values go; `None` leaves that macro row hidden.
    pub targets:
        [Option<crate::sequence_modulation::MacroTarget>; crate::sequence_modulation::MACROS],
    /// Per macro, the plugin's own name for it, such as "Macro 1".
    pub labels: [&'static str; crate::sequence_modulation::MACROS],
    /// Current modulation wheel and macro positions, on the CC scale.
    pub wheel: Option<u8>,
    pub macros: [Option<u8>; crate::sequence_modulation::MACROS],
    /// Per macro, the name given in the patch; `None` when unnamed.
    pub names: [Option<String>; crate::sequence_modulation::MACROS],
}

/// Instruments whose macro rows move their own macro parameters instead of CCs.
pub(crate) fn parameter_macros(plugin: &PluginKey) -> Option<crate::instrument_macros::Instrument> {
    crate::instrument_macros::Instrument::of(&plugin.format, &plugin.id)
        .filter(|i| *i != crate::instrument_macros::Instrument::SurgeXt)
}

#[cfg(test)]
impl Controls {
    /// The CC of each macro row that sends one.
    pub fn ccs(&self) -> [Option<u8>; crate::sequence_modulation::MACROS] {
        self.targets.map(|target| match target {
            Some(crate::sequence_modulation::MacroTarget::Cc(cc)) => Some(cc),
            _ => None,
        })
    }
}

/// ADR 0036 / 0038: only these plugins' states are read for [`controls`].
pub(crate) fn reports_controls(plugin: &PluginKey) -> bool {
    surge_xt_macro::supports(plugin)
        || tyrell_n6_control::supports(plugin)
        || parameter_macros(plugin).is_some()
}

pub(crate) fn controls(plugin: &PluginKey, state: &[u8]) -> Controls {
    if tyrell_n6_control::supports(plugin) {
        tyrell_n6_control::controls(plugin, state)
    } else if parameter_macros(plugin).is_some() {
        parameter_macro_controls::controls(plugin, state)
    } else {
        surge_xt_macro::controls(plugin, state)
    }
}

/// ADR 0027: the swapped-out or swapped-in effect needs a gain ramp around the swap.
pub(crate) fn fades_effect_swap(plugin: &PluginKey) -> bool {
    surge_xt::is_effects(plugin)
}
