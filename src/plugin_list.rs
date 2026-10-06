//! Presentation order is separate from the native catalog indices used to load plugins.
use crate::{config::PluginKey, ffi::PluginInfo};
use eframe::egui::Color32;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginKind {
    Instrument,
    Effect,
    Unknown,
}

impl PluginKind {
    pub fn from_metadata(value: &str) -> Self {
        match value {
            "instrument" => Self::Instrument,
            "effect" => Self::Effect,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Instrument => "Instrument",
            Self::Effect => "Effect",
            Self::Unknown => "Unknown",
        }
    }

    pub fn color(self, dark: bool) -> Color32 {
        match (self, dark) {
            (Self::Instrument, true) => Color32::from_rgb(115, 205, 255),
            (Self::Instrument, false) => Color32::from_rgb(0, 90, 150),
            (Self::Effect, true) => Color32::from_rgb(255, 185, 110),
            (Self::Effect, false) => Color32::from_rgb(150, 70, 0),
            (Self::Unknown, true) => Color32::LIGHT_GRAY,
            (Self::Unknown, false) => Color32::DARK_GRAY,
        }
    }
}

fn group_key(plugin: &PluginInfo) -> (String, String) {
    (
        plugin.name.trim().to_lowercase(),
        plugin.vendor.trim().to_lowercase(),
    )
}

pub fn ordered_indices(plugins: &[PluginInfo], restored: Option<&PluginKey>) -> Vec<usize> {
    let restored_index = restored.and_then(|key| key.find(plugins));
    let restored_group = restored_index.map(|index| group_key(&plugins[index]));
    let mut groups: BTreeMap<_, Vec<usize>> = BTreeMap::new();
    for (index, plugin) in plugins.iter().enumerate() {
        groups.entry(group_key(plugin)).or_default().push(index);
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(key, indices)| {
        (
            restored_group.as_ref() != Some(key),
            indices.iter().map(|&i| plugins[i].kind).min().unwrap(),
            key.clone(),
        )
    });
    groups
        .into_iter()
        .flat_map(|(_, mut indices)| {
            indices.sort_by_key(|&i| {
                (
                    restored_index != Some(i),
                    plugins[i].format.clone(),
                    plugins[i].id.clone(),
                )
            });
            indices
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin(index: i32, name: &str, format: &str, kind: PluginKind) -> PluginInfo {
        PluginInfo {
            index,
            name: name.into(),
            vendor: "Vendor".into(),
            format: format.into(),
            id: format!("{name}-{format}"),
            bundle_path: String::new(),
            kind,
        }
    }

    #[test]
    fn restored_effect_and_its_other_format_precede_instruments() {
        let plugins = vec![
            plugin(5, "Synth", "VST3", PluginKind::Instrument),
            plugin(9, "Reverb", "CLAP", PluginKind::Effect),
            plugin(3, "Synth", "CLAP", PluginKind::Instrument),
            plugin(7, "Reverb", "VST3", PluginKind::Effect),
            plugin(1, "Unknown", "CLAP", PluginKind::Unknown),
        ];
        let restored = PluginKey::from_plugin(&plugins[3]);
        assert_eq!(ordered_indices(&plugins, Some(&restored)), [3, 1, 2, 0, 4]);
        assert_eq!(ordered_indices(&plugins, None), [2, 0, 1, 3, 4]);
        assert_eq!(plugins[3].index, 7);
    }

    #[test]
    fn grouping_does_not_mix_different_vendors_or_effect_versions() {
        let mut plugins = vec![
            plugin(0, "Synth", "CLAP", PluginKind::Instrument),
            plugin(1, "Synth Effects", "CLAP", PluginKind::Effect),
            plugin(2, "Synth", "VST3", PluginKind::Instrument),
        ];
        plugins[2].vendor = "Other".into();
        let restored = PluginKey::from_plugin(&plugins[0]);
        assert_eq!(ordered_indices(&plugins, Some(&restored)), [0, 2, 1]);
    }
}
