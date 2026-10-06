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
    pub sequence_pattern: SequencePattern,
}

impl Favorite {
    pub fn playback_pattern(&self, current: SequencePattern) -> SequencePattern {
        if self.effect {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_pattern: Option<SequencePattern>,
}

impl From<Favorite> for FavoriteMetadata {
    fn from(favorite: Favorite) -> Self {
        Self {
            id: favorite.id,
            name: favorite.name,
            plugin: favorite.plugin,
            effect: favorite.effect,
            sequence_pattern: (!favorite.effect).then_some(favorite.sequence_pattern),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Library {
    pub entries: Vec<Favorite>,
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
                // Old effect entries always serialized sequence_pattern.
                // New entries omit it, making this migration repeat-safe.
                if let Some(entries) = metadata.get("entries").and_then(toml::Value::as_array) {
                    let legacy: Vec<_> = library
                        .entries
                        .iter()
                        .zip(entries)
                        .filter(|(favorite, entry)| {
                            favorite.effect && entry.get("sequence_pattern").is_some()
                        })
                        .map(|(favorite, _)| favorite.id.clone())
                        .collect();
                    if !legacy.is_empty() {
                        // Leave legacy metadata in place until every file is removed;
                        // interruption or a write failure can safely retry migration.
                        for id in &legacy {
                            match std::fs::remove_file(state_path(config, id)?) {
                                Ok(()) => {}
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                                Err(e) => {
                                    return Err(format!(
                                        "Could not remove legacy effect favorite {id}: {e}"
                                    ))
                                }
                            }
                        }
                        library
                            .entries
                            .retain(|favorite| !legacy.contains(&favorite.id));
                        library.save(config)?;
                    }
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
        sequence_pattern: SequencePattern,
    ) -> Result<Favorite, String> {
        let name = plugin.name.clone();
        self.add_snapshot(
            config,
            Favorite {
                id: String::new(),
                name,
                plugin,
                effect,
                sequence_pattern: if effect {
                    SequencePattern::Off
                } else {
                    sequence_pattern
                },
            },
            state,
        )
    }

    pub fn add_automatic(
        &mut self,
        config: &Path,
        plugin: PluginKey,
        effect: bool,
        state: &[u8],
        sequence_pattern: SequencePattern,
    ) -> Result<Favorite, String> {
        let name = format!("auto {}", plugin.name);
        self.add_snapshot(
            config,
            Favorite {
                id: String::new(),
                name,
                plugin,
                effect,
                sequence_pattern: if effect {
                    SequencePattern::Off
                } else {
                    sequence_pattern
                },
            },
            state,
        )
    }

    fn add_snapshot(
        &mut self,
        config: &Path,
        mut favorite: Favorite,
        state: &[u8],
    ) -> Result<Favorite, String> {
        for entry in &self.entries {
            if entry.plugin.format == favorite.plugin.format
                && entry.plugin.id == favorite.plugin.id
                && entry.effect == favorite.effect
                && (favorite.effect || entry.sequence_pattern == favorite.sequence_pattern)
                && crate::plugin_specific::same_favorite_state(
                    &favorite.plugin,
                    &self.state(config, &entry.id)?,
                    state,
                )
            {
                // Reuse the immutable snapshot, including a user-assigned name.
                // Replacing it would churn both its ID and its automatic number.
                return Ok(entry.clone());
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
