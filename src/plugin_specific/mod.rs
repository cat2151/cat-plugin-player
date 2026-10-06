//! All production exceptions tied to a particular plugin live here.
//! See docs/adr/0002-plugin-specific-favorite-state-comparison.md for the decision and limits.
use crate::config::PluginKey;

mod tyrell_n6;
mod vaporizer2;

pub(crate) fn same_favorite_state(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    left == right
        || tyrell_n6::same_sound(plugin, left, right)
        || vaporizer2::same_sound(plugin, left, right)
}
