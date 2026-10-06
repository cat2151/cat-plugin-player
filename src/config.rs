use crate::sequence_modulation::SequenceModulation;
use crate::sequence_velocity::SequenceVelocity;
use crate::{ffi::PluginInfo, sequence_pattern::SequencePattern};
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

#[derive(Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub sequence_pattern: SequencePattern,
    #[serde(default)]
    pub selected_sequence: SequencePattern,
    #[serde(default)]
    pub sequence_velocity: SequenceVelocity,
    #[serde(default)]
    pub sequence_modulation: SequenceModulation,
    pub last_played: Option<PluginKey>,
    #[serde(default)]
    pub effect: Option<PluginKey>,
    #[serde(default)]
    pub effect_bypassed: bool,
    #[serde(default)]
    pub favorites: FavoriteSelection,
    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Default, Deserialize, Serialize)]
pub struct FavoriteSelection {
    pub instrument: Option<String>,
    pub effect: Option<String>,
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

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let contents = toml::to_string_pretty(self).map_err(|error| error.to_string())?;
        let parent = path.parent().ok_or("config path has no parent")?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        std::fs::write(path, contents).map_err(|error| error.to_string())
    }

    /// Keep the current build settings, including edits made while the GUI is open.
    pub fn save_session(&mut self, path: &Path) -> Result<(), String> {
        self.build = Self::load(path)?.build;
        self.save(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_roundtrips_and_old_sessions_default_to_100() {
        let old: Config = toml::from_str("sequence_pattern = 'steps'").unwrap();
        assert_eq!(old.sequence_velocity, SequenceVelocity::V100);
        for &velocity in SequenceVelocity::TYPES {
            let config = Config {
                sequence_velocity: velocity,
                ..Default::default()
            };
            let restored: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            assert_eq!(restored.sequence_velocity, velocity);
        }
    }

    #[test]
    fn every_pattern_including_stopped_survives_serialization() {
        for pattern in
            std::iter::once(SequencePattern::Off).chain(SequencePattern::TYPES.iter().copied())
        {
            let config = Config {
                sequence_pattern: pattern,
                selected_sequence: pattern,
                ..Default::default()
            };
            let saved = toml::to_string(&config).unwrap();
            let restored: Config = toml::from_str(&saved).unwrap();
            assert_eq!(restored.sequence_pattern, pattern);
            assert_eq!(restored.selected_sequence, pattern);
        }
    }

    #[test]
    fn stopped_selection_survives_and_legacy_history_keeps_transport() {
        let config = Config {
            sequence_pattern: SequencePattern::Off,
            selected_sequence: SequencePattern::GuitarArpeggio,
            ..Default::default()
        };
        let saved = toml::to_string(&config).unwrap();
        let restored: Config = toml::from_str(&saved).unwrap();
        assert_eq!(restored.sequence_pattern, SequencePattern::Off);
        assert_eq!(
            restored
                .sequence_pattern
                .selection(restored.selected_sequence),
            SequencePattern::GuitarArpeggio
        );
        for pattern in ["off", "steps", "guitar_arpeggio"] {
            let old: Config = toml::from_str(&format!("sequence_pattern = '{pattern}'")).unwrap();
            let expected = if pattern == "guitar_arpeggio" {
                SequencePattern::GuitarArpeggio
            } else {
                SequencePattern::Steps
            };
            assert_eq!(
                old.sequence_pattern.selection(old.selected_sequence),
                expected
            );
            assert_eq!(
                old.sequence_pattern == SequencePattern::Off,
                pattern == "off"
            );
        }
        assert_eq!(
            SequencePattern::Off.selection(SequencePattern::Off),
            SequencePattern::Steps
        );
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
                name: "音源".into(),
                vendor: "Vendor".into(),
                bundle_path: "X:/plugins/音源.clap".into(),
            }),
            effect: Some(PluginKey {
                format: "VST3".into(),
                id: "reverb".into(),
                name: "Reverb".into(),
                vendor: String::new(),
                bundle_path: "X:/plugins/reverb.vst3".into(),
            }),
            effect_bypassed: true,
            favorites: FavoriteSelection {
                instrument: Some("instrument-favorite".into()),
                effect: Some("effect-favorite".into()),
            },
            sequence_pattern: SequencePattern::GuitarArpeggio,
            selected_sequence: SequencePattern::GuitarArpeggio,
            build: BuildConfig {
                uapmd_dir: Some("X:/dependencies/UAPMD 日本語".into()),
            },
            sequence_velocity: SequenceVelocity::Range80To127,
            sequence_modulation: SequenceModulation::Sweep,
        };
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.last_played, config.last_played);
        assert_eq!(loaded.effect, config.effect);
        assert!(loaded.effect_bypassed);
        assert_eq!(loaded.favorites.instrument, config.favorites.instrument);
        assert_eq!(loaded.favorites.effect, config.favorites.effect);
        assert_eq!(loaded.sequence_pattern, SequencePattern::GuitarArpeggio);
        assert_eq!(loaded.build.uapmd_dir, config.build.uapmd_dir);
        let mut session = Config::default();
        session.save_session(&path).unwrap();
        assert_eq!(
            Config::load(&path).unwrap().build.uapmd_dir,
            config.build.uapmd_dir
        );
        std::fs::write(&path, "[invalid").unwrap();
        assert!(session.save_session(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[invalid");
        assert!(Config::load(&path).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn old_history_can_fall_back_to_scanning() {
        let config: Config =
            toml::from_str("[last_played]\nformat = 'CLAP'\nid = 'synth'\n").unwrap();
        assert_eq!(config.sequence_pattern, SequencePattern::Steps);
        assert!(config.effect.is_none());
        assert!(!config.effect_bypassed);
        assert!(config.favorites.instrument.is_none());
        assert!(config.favorites.effect.is_none());
        assert!(config.build.uapmd_dir.is_none());
        let key = config.last_played.unwrap();
        assert!(key.bundle_path.is_empty());
        assert_eq!(key.id, "synth");
    }
}
