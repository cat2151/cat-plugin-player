use super::PluginInfo;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct PluginKey {
    pub format: String,
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub vendor: String,
    /// Empty in SMF references, which leave the location to each machine.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bundle_path: String,
}

impl PluginKey {
    pub fn from_plugin(plugin: &PluginInfo) -> Self {
        Self {
            format: plugin.format.clone(),
            id: plugin.id.clone(),
            name: plugin.name.clone(),
            vendor: plugin.vendor.clone(),
            bundle_path: plugin.bundle_path.clone(),
        }
    }

    pub fn find(&self, plugins: &[PluginInfo]) -> Option<usize> {
        plugins
            .iter()
            .position(|plugin| plugin.format == self.format && plugin.id == self.id)
    }
}

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
}
