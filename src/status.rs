//! Machine-owned session status, alongside the user-owned config.toml.
use crate::config::PluginKey;
use crate::sequence_modulation::SequenceModulation;
use crate::sequence_pattern::SequencePattern;
use crate::sequence_velocity::SequenceVelocity;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize)]
pub struct Status {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_favorites: Option<bool>,
    #[serde(default)]
    pub show_history: bool,
    #[serde(default = "default_on_right")]
    pub on_right: bool,
    #[serde(default)]
    pub sequence_pattern: SequencePattern,
    #[serde(default)]
    pub selected_sequence: SequencePattern,
    #[serde(default)]
    pub sequence_velocity: SequenceVelocity,
    #[serde(default)]
    pub sequence_modulation: SequenceModulation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played: Option<PluginKey>,
    #[serde(default, skip_serializing)]
    pub effect: Option<PluginKey>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<PluginKey>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub effect_bypassed: bool,
    #[serde(default, skip_serializing_if = "FavoriteSelection::is_empty")]
    pub favorites: FavoriteSelection,
}

fn default_on_right() -> bool {
    true
}

impl Default for Status {
    fn default() -> Self {
        Self {
            show_favorites: None,
            show_history: false,
            on_right: default_on_right(),
            sequence_pattern: Default::default(),
            selected_sequence: Default::default(),
            sequence_velocity: Default::default(),
            sequence_modulation: Default::default(),
            last_played: None,
            effect: None,
            effects: Vec::new(),
            effect_bypassed: false,
            favorites: Default::default(),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct FavoriteSelection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument: Option<String>,
    #[serde(default, skip_serializing)]
    pub effect: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<String>,
}

fn is_false(value: &bool) -> bool {
    !value
}

impl FavoriteSelection {
    fn is_empty(&self) -> bool {
        self.instrument.is_none() && self.effects.is_empty()
    }
}

pub fn path(config: &Path) -> PathBuf {
    config.with_file_name("status.json")
}

impl Status {
    pub fn load(config: &Path) -> Result<Self, String> {
        // Migration validates both files and never replaces an invalid status.
        crate::status_migration::migrate(config)?;
        Self::read(config).map(|status| status.unwrap_or_default())
    }

    pub(crate) fn read(config: &Path) -> Result<Option<Self>, String> {
        let destination = path(config);
        match std::fs::read(&destination) {
            Ok(contents) => {
                let mut status: Self = serde_json::from_slice(&contents)
                    .map_err(|error| format!("{}: {error}", destination.display()))?;
                status.migrate_effects();
                Ok(Some(status))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("{}: {error}", destination.display())),
        }
    }

    pub(crate) fn migrate_effects(&mut self) {
        if self.effects.is_empty() {
            if let Some(effect) = self.effect.take() {
                self.effects.push(effect);
                self.favorites
                    .effects
                    .push(self.favorites.effect.take().unwrap_or_default());
            }
        }
        self.effect = None;
        self.favorites.effect = None;
    }

    pub fn save(&self, config: &Path) -> Result<(), String> {
        // A failed read/migration must not be hidden by a subsequent UI save.
        Self::load(config)?;
        self.write(config)
    }

    pub(crate) fn write(&self, config: &Path) -> Result<(), String> {
        let contents = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        crate::state_store::atomic_write(&path(config), &contents)
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
