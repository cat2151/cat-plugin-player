//! User-owned settings. Application status is stored separately.
use crate::ffi::PluginInfo;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PluginKey {
    pub format: String,
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub vendor: String,
    #[serde(default)]
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

#[derive(Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub plugins: PluginDisplayConfig,
    #[serde(default)]
    pub window: crate::window_config::WindowConfig,
    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Deserialize)]
pub struct PluginDisplayConfig {
    #[serde(default = "default_prefer_clap")]
    pub prefer_clap: bool,
}

fn default_prefer_clap() -> bool {
    true
}

impl Default for PluginDisplayConfig {
    fn default() -> Self {
        Self { prefer_clap: true }
    }
}

pub fn save_prefer_clap(path: &Path, enabled: bool) -> Result<(), String> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.to_string()),
    };
    let _: Config = toml::from_str(&contents).map_err(|error| error.to_string())?;
    let mut document = contents
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| error.to_string())?;
    document["plugins"]["prefer_clap"] = toml_edit::value(enabled);
    crate::state_store::atomic_write(path, document.to_string().as_bytes())
}

#[derive(Default, Deserialize, Serialize)]
pub struct BuildConfig {
    #[serde(default, alias = "UAPMD_DIR", skip_serializing_if = "Option::is_none")]
    pub uapmd_dir: Option<PathBuf>,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_display_defaults_to_on_in_old_and_new_configs() {
        assert!(Config::default().plugins.prefer_clap);
        for source in ["", "[window.main]\nx = 100.0", "[plugins]"] {
            assert!(
                toml::from_str::<Config>(source)
                    .unwrap()
                    .plugins
                    .prefer_clap
            );
        }
        assert!(
            !toml::from_str::<Config>("[plugins]\nprefer_clap = false")
                .unwrap()
                .plugins
                .prefer_clap
        );
    }

    #[test]
    fn display_setting_save_preserves_other_settings_and_rejects_invalid_config() {
        let directory =
            std::env::temp_dir().join(format!("cat-plugin-display-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("config.toml");
        let original =
            "# user notes\n[window.main]\nx = 42.0 # position\n[custom]\nvalue = 'keep'\n";
        std::fs::write(&path, original).unwrap();
        save_prefer_clap(&path, false).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains(original));
        assert!(!Config::load(&path).unwrap().plugins.prefer_clap);
        save_prefer_clap(&path, true).unwrap();
        assert!(Config::load(&path).unwrap().plugins.prefer_clap);
        std::fs::write(&path, "[plugins]\nprefer_clap = 'invalid'").unwrap();
        assert!(save_prefer_clap(&path, false).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[plugins]\nprefer_clap = 'invalid'"
        );
        std::fs::remove_file(&path).unwrap();
        save_prefer_clap(&path, false).unwrap();
        assert!(!Config::load(&path).unwrap().plugins.prefer_clap);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn identity_survives_reordering_and_distinguishes_formats() {
        let plugin = |index, format: &str, id: &str| PluginInfo {
            index,
            name: "Same name".into(),
            vendor: String::new(),
            format: format.into(),
            id: id.into(),
            bundle_path: "X:/plugins/synth.clap".into(),
            kind: crate::plugin_list::PluginKind::Unknown,
        };
        let key = PluginKey::from_plugin(&plugin(0, "CLAP", "synth"));
        let plugins = [plugin(0, "VST3", "synth"), plugin(8, "CLAP", "synth")];
        assert_eq!(key.find(&plugins), Some(1));
        assert_eq!(key.find(&plugins[..1]), None);
    }
}
