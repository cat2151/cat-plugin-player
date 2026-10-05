use crate::ffi::PluginInfo;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PluginKey {
    pub format: String,
    pub id: String,
}

impl PluginKey {
    pub fn from_plugin(plugin: &PluginInfo) -> Self {
        Self {
            format: plugin.format.clone(),
            id: plugin.id.clone(),
        }
    }

    pub fn find(&self, plugins: &[PluginInfo]) -> Option<usize> {
        plugins
            .iter()
            .position(|plugin| plugin.format == self.format && plugin.id == self.id)
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Config {
    pub last_played: Option<PluginKey>,
}

pub fn path() -> Result<PathBuf, String> {
    dirs::config_local_dir()
        .map(|directory| directory.join("cat-plugin-player").join("config.toml"))
        .ok_or_else(|| "could not locate the local config directory".into())
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).map_err(|error| error.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let contents = toml::to_string_pretty(self).map_err(|error| error.to_string())?;
        let parent = path.parent().ok_or("config path has no parent")?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        std::fs::write(path, contents).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_survives_reordering_and_distinguishes_formats() {
        let plugin = |index, format: &str, id: &str| PluginInfo {
            index,
            name: "Same name".into(),
            vendor: String::new(),
            format: format.into(),
            id: id.into(),
        };
        let key = PluginKey::from_plugin(&plugin(0, "CLAP", "synth"));
        let plugins = [plugin(0, "VST3", "synth"), plugin(8, "CLAP", "synth")];
        assert_eq!(key.find(&plugins), Some(1));
        assert_eq!(key.find(&plugins[..1]), None);
    }

    #[test]
    fn config_round_trip_missing_and_invalid_file() {
        let directory = std::env::temp_dir().join(format!(
            "cat-plugin-player-config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = directory.join("nested").join("config.toml");
        assert!(Config::load(&path).unwrap().last_played.is_none());
        let config = Config {
            last_played: Some(PluginKey {
                format: "CLAP".into(),
                id: "X:/plugins/音源\".clap".into(),
            }),
        };
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap().last_played, config.last_played);
        std::fs::write(&path, "[invalid").unwrap();
        assert!(Config::load(&path).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
