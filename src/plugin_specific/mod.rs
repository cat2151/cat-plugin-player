//! All production exceptions tied to a particular plugin live here.
//! See docs/adr/0002-plugin-specific-favorite-state-comparison.md for the decision and limits.
use crate::config::PluginKey;

mod floe;
mod patch_settling;
mod sforzando;
mod surge_xt;
mod tyrell_n6;
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
