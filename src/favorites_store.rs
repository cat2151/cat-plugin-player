//! Favorite metadata and immutable binary snapshots, separate from autosave.
use crate::{config::PluginKey, sequence_pattern::SequencePattern, state_store::atomic_write};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(into = "FavoriteMetadata")]
pub struct Favorite {
    pub id: String,
    pub name: String,
    pub plugin: PluginKey,
    pub effect: bool,
    #[serde(default)]
    pub history: bool,
    #[serde(default = "default_favorite")]
    pub favorite: bool,
    #[serde(default)]
    pub registered_at: u64,
    #[serde(default)]
    pub sequence_pattern: SequencePattern,
    #[serde(default)]
    pub selected_sequence: Option<SequencePattern>,
    #[serde(default)]
    pub mml: Option<String>,
}

fn default_favorite() -> bool {
    true
}

impl Favorite {
    pub fn playback_pattern(&self, current: SequencePattern) -> SequencePattern {
        if self.effect
            || current == SequencePattern::Off
            || self.sequence_pattern == SequencePattern::Off
        {
            current
        } else {
            self.sequence_pattern
        }
    }
}

// Effect snapshots carry only the plugin state, never playback controls.
#[derive(Serialize)]
struct FavoriteMetadata {
    id: String,
    name: String,
    plugin: PluginKey,
    effect: bool,
    history: bool,
    favorite: bool,
    registered_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_pattern: Option<SequencePattern>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_sequence: Option<SequencePattern>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mml: Option<String>,
}

impl From<Favorite> for FavoriteMetadata {
    fn from(favorite: Favorite) -> Self {
        Self {
            id: favorite.id,
            name: favorite.name,
            plugin: favorite.plugin,
            effect: favorite.effect,
            history: favorite.history,
            favorite: favorite.favorite,
            registered_at: favorite.registered_at,
            sequence_pattern: (!favorite.effect).then_some(favorite.sequence_pattern),
            selected_sequence: if favorite.effect {
                None
            } else {
                favorite.selected_sequence
            },
            mml: if favorite.effect { None } else { favorite.mml },
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Library {
    pub entries: Vec<Favorite>,
}

#[derive(Clone)]
pub(crate) struct FavoriteCapture {
    pub sequence_pattern: SequencePattern,
    pub sweep_cc1: bool,
    pub selected_sequence: Option<SequencePattern>,
    pub mml: Option<String>,
}

impl From<SequencePattern> for FavoriteCapture {
    fn from(sequence_pattern: SequencePattern) -> Self {
        Self {
            sequence_pattern,
            sweep_cc1: false,
            selected_sequence: None,
            mml: None,
        }
    }
}

fn directory(config: &Path) -> Result<PathBuf, String> {
    Ok(config
        .parent()
        .ok_or("config path has no parent")?
        .join("favorites"))
}

fn state_path(config: &Path, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err("invalid favorite ID".into());
    }
    Ok(directory(config)?.join(format!("{id}.bin")))
}

impl Library {
    pub fn load(config: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(directory(config)?.join("index.toml")) {
            Ok(text) => {
                let mut library: Self = toml::from_str(&text).map_err(|e| e.to_string())?;
                let metadata: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
                // Keep every legacy snapshot for history, including old effects.
                // Serialization drops their obsolete playback controls without deleting state.
                if crate::history::migrate(&mut library, &metadata, config)? {
                    library.save(config)?;
                }
                Ok(library)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn save(&self, config: &Path) -> Result<(), String> {
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        atomic_write(&directory(config)?.join("index.toml"), text.as_bytes())
    }

    pub fn add(
        &mut self,
        config: &Path,
        plugin: PluginKey,
        effect: bool,
        state: &[u8],
        capture: impl Into<FavoriteCapture>,
    ) -> Result<Favorite, String> {
        let capture = capture.into();
        let name = plugin.name.clone();
        self.add_snapshot(
            config,
            Favorite {
                id: String::new(),
                name,
                plugin,
                effect,
                history: false,
                favorite: true,
                registered_at: 0,
                selected_sequence: if effect {
                    None
                } else {
                    capture.selected_sequence
                },
                mml: if effect { None } else { capture.mml },
                sequence_pattern: if effect {
                    SequencePattern::Off
                } else {
                    capture.sequence_pattern
                },
            },
            state,
            capture.sweep_cc1,
            false,
        )
    }

    pub fn add_automatic(
        &mut self,
        config: &Path,
        plugin: PluginKey,
        effect: bool,
        state: &[u8],
        capture: impl Into<FavoriteCapture>,
    ) -> Result<Favorite, String> {
        let capture = capture.into();
        let name = plugin.name.clone();
        self.add_snapshot(
            config,
            Favorite {
                id: String::new(),
                name,
                plugin,
                effect,
                history: true,
                favorite: false,
                registered_at: crate::history::now(),
                selected_sequence: if effect {
                    None
                } else {
                    capture.selected_sequence
                },
                mml: if effect { None } else { capture.mml },
                sequence_pattern: if effect {
                    SequencePattern::Off
                } else {
                    capture.sequence_pattern
                },
            },
            state,
            capture.sweep_cc1,
            true,
        )
    }

    fn add_snapshot(
        &mut self,
        config: &Path,
        mut favorite: Favorite,
        state: &[u8],
        sweep_cc1: bool,
        automatic: bool,
    ) -> Result<Favorite, String> {
        for entry in &self.entries {
            if entry.plugin.format == favorite.plugin.format
                && entry.plugin.id == favorite.plugin.id
                && entry.effect == favorite.effect
                && (favorite.effect
                    || entry.mml.as_deref().unwrap_or("") == favorite.mml.as_deref().unwrap_or(""))
                && (favorite.effect
                    || (automatic
                        && (entry.sequence_pattern == SequencePattern::Off
                            || favorite.sequence_pattern == SequencePattern::Off))
                    || entry
                        .sequence_pattern
                        .selection(entry.selected_sequence.unwrap_or_default())
                        == favorite
                            .sequence_pattern
                            .selection(favorite.selected_sequence.unwrap_or_default()))
                && (favorite.effect
                    || entry.sequence_pattern == favorite.sequence_pattern
                    // Stop is transport state, not a new automatic favorite.
                    || (automatic
                        && (entry.sequence_pattern == SequencePattern::Off
                            || favorite.sequence_pattern == SequencePattern::Off)))
                && crate::plugin_specific::same_favorite_state_with_sweep(
                    &favorite.plugin,
                    &self.state(config, &entry.id)?,
                    state,
                    sweep_cc1 && !favorite.effect,
                )
            {
                // Reuse the immutable snapshot, including a user-assigned name.
                // Replacing it would churn both its ID and its automatic number.
                let id = entry.id.clone();
                let previous = self.entries.clone();
                let index = self
                    .entries
                    .iter()
                    .position(|entry| entry.id == id)
                    .unwrap();
                // Only a new Favorites membership goes to the front. Recall,
                // re-saving and History registration preserve the manual order.
                let index = if !automatic && !self.entries[index].favorite {
                    let existing = self.entries.remove(index);
                    self.entries.insert(0, existing);
                    0
                } else {
                    index
                };
                let entry = &mut self.entries[index];
                if automatic {
                    entry.history = true;
                    entry.registered_at = crate::history::now();
                } else {
                    entry.favorite = true;
                }
                if let Err(error) = self.save(config) {
                    self.entries = previous;
                    return Err(error);
                }
                return Ok(self.entries[index].clone());
            }
        }
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        favorite.id = format!(
            "{time:x}-{:x}-{:x}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        favorite.name = (1..)
            .map(|n| format!("{} {n}", favorite.name))
            .find(|name| !self.entries.iter().any(|f| &f.name == name))
            .unwrap();
        let path = state_path(config, &favorite.id)?;
        atomic_write(&path, state)?;
        let previous = self.entries.clone();
        self.entries.insert(0, favorite.clone());
        if let Err(e) = self.save(config) {
            self.entries = previous;
            let _ = std::fs::remove_file(path);
            return Err(e);
        }
        Ok(favorite)
    }

    pub fn state(&self, config: &Path, id: &str) -> Result<Vec<u8>, String> {
        std::fs::read(state_path(config, id)?).map_err(|e| e.to_string())
    }

    pub fn rename(&mut self, config: &Path, id: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Name cannot be empty".into());
        }
        let entry = self
            .entries
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or("favorite not found")?;
        let old = std::mem::replace(&mut entry.name, name.into());
        if let Err(e) = self.save(config) {
            self.entries.iter_mut().find(|f| f.id == id).unwrap().name = old;
            return Err(e);
        }
        Ok(())
    }

    pub fn delete(&mut self, config: &Path, id: &str) -> Result<(), String> {
        let path = state_path(config, id)?;
        let index = self
            .entries
            .iter()
            .position(|f| f.id == id)
            .ok_or("favorite not found")?;
        let entry = self.entries.remove(index);
        if let Err(e) = self.save(config) {
            self.entries.insert(index, entry);
            return Err(e);
        }
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!(
                "Removed from list, but could not remove snapshot: {e}"
            )),
        }
    }
}

#[cfg(test)]
#[path = "favorites_store_tests.rs"]
mod tests;

#[path = "favorites_order.rs"]
mod order;

#[cfg(test)]
#[path = "favorites_mml_store_tests.rs"]
mod mml_tests;
