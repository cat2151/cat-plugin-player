//! All production exceptions tied to a particular plugin live here.
//! See docs/adr/0002-plugin-specific-favorite-state-comparison.md for the decision and limits.
use crate::config::PluginKey;

mod floe;
mod patch_settling;
mod sforzando;
pub(crate) mod surge_start_delay;
mod surge_xt;
mod surge_xt_macro;
mod tyrell_n6;
mod tyrell_n6_control;
mod vaporizer2;

pub(crate) use surge_xt::StartupDataHome;

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
}

/// ADR 0036 / 0038: what an instrument state says about the controls CCs move.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Controls {
    /// Per macro, the CC of a routed macro; `None` leaves that macro row hidden.
    pub ccs: [Option<u8>; crate::sequence_modulation::MACROS],
    /// Per macro, the plugin's own name for it, such as "Macro 1".
    pub labels: [&'static str; crate::sequence_modulation::MACROS],
    /// Current modulation wheel and macro positions, on the CC scale.
    pub wheel: Option<u8>,
    pub macros: [Option<u8>; crate::sequence_modulation::MACROS],
    /// Per macro, the name given in the patch; `None` when unnamed.
    pub names: [Option<String>; crate::sequence_modulation::MACROS],
}

/// ADR 0036 / 0038: only these plugins' states are read for [`controls`].
pub(crate) fn reports_controls(plugin: &PluginKey) -> bool {
    surge_xt_macro::supports(plugin) || tyrell_n6_control::supports(plugin)
}

pub(crate) fn controls(plugin: &PluginKey, state: &[u8]) -> Controls {
    if tyrell_n6_control::supports(plugin) {
        tyrell_n6_control::controls(plugin, state)
    } else {
        surge_xt_macro::controls(plugin, state)
    }
}

/// ADR 0027: the swapped-out or swapped-in effect needs a gain ramp around the swap.
pub(crate) fn fades_effect_swap(plugin: &PluginKey) -> bool {
    surge_xt::is_effects(plugin)
}

pub(crate) fn same_favorite_state_with_sweep(
    plugin: &PluginKey,
    left: &[u8],
    right: &[u8],
    sweep_cc1: bool,
) -> bool {
    same_favorite_state(plugin, left, right)
        || (sweep_cc1
            && (floe::same_sweep_sound(plugin, left, right)
                || vaporizer2::same_sweep_sound(plugin, left, right)))
}
