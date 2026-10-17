//! Short automatic labels; the original preset description remains searchable.
use super::{FavoriteCapture, PluginKey};

pub(super) fn default_name(capture: &FavoriteCapture, plugin: &PluginKey, effect: bool) -> String {
    capture
        .preset_name
        .as_deref()
        .map(|name| stem(name, &plugin.name, effect))
        .filter(|name| !name.is_empty())
        .unwrap_or(&plugin.name)
        .to_owned()
}

fn stem<'a>(name: &'a str, plugin: &str, effect: bool) -> &'a str {
    let mut name = name.trim();
    if effect && !plugin.is_empty() {
        if let Some(preset) = name
            .strip_prefix(plugin)
            .and_then(|name| name.strip_prefix(':'))
        {
            name = preset.trim();
        }
    }
    let leaf = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    // Only preset-file suffixes are extensions. Program labels and already
    // extensionless names can contain dots (e.g. "22 <Flute.22>" or "FLUTE 22.6").
    if let Some((stem, extension)) = leaf.rsplit_once('.') {
        if !stem.is_empty()
            && [
                "fxp",
                "fxb",
                "syx",
                "sfz",
                "floe-preset",
                "sxsnp",
                "h2p",
                "vvp",
                "srgfx",
                "t3kpreset",
                "pst",
            ]
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
        {
            return stem.trim();
        }
    }
    leaf
}

#[cfg(test)]
#[path = "favorites_naming_tests.rs"]
mod tests;
