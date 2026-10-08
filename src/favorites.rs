//! Favorite actions run on the UI thread, with audio stopped for state APIs.
use crate::{
    favorites_store::{Favorite, FavoriteCapture, Library},
    plugin_list::PluginKind,
    App,
};

#[derive(Default)]
pub struct Favorites {
    pub library: Library,
    pub initialized: bool,
    pub error: Option<String>,
    pub show: bool,
    pub restore_show: Option<bool>,
    pub active: Vec<(i32, String)>,
    pub restore: crate::status::FavoriteSelection,
    pub pending: Option<(Favorite, Vec<u8>)>,
    pub rename: Option<(String, String)>,
}

#[cfg(test)]
#[path = "favorites_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "effect_favorite_tests.rs"]
mod effect_tests;

impl App {
    pub(crate) fn initialize_favorites(&mut self) {
        if self.favorites.initialized {
            return;
        }
        self.favorites.initialized = true;
        match self
            .config_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| Library::load(path))
        {
            Ok(library) => {
                self.favorites.show = self
                    .favorites
                    .restore_show
                    .take()
                    .unwrap_or(!library.entries.is_empty());
                self.favorites.library = library;
            }
            Err(e) => self.favorites.error = Some(format!("Could not read favorites: {e}")),
        }
    }

    pub(crate) fn add_favorite(&mut self, id: i32) {
        if self.pending.is_some() || self.restoring || self.favorites.error.is_some() {
            return;
        }
        let previous_count = self.favorites.library.entries.len();
        self.pause_audio();
        let result = (|| {
            let instance = self
                .instances
                .iter()
                .find(|i| i.id == id)
                .ok_or("instance not found")?;
            let state = self.host.save_state(id)?;
            let path = self.config_path.as_ref().map_err(Clone::clone)?;
            self.favorites.library.add(
                path,
                instance.plugin.clone(),
                instance.kind == PluginKind::Effect,
                &state,
                FavoriteCapture {
                    sequence_pattern: self.sequence_pattern,
                    sweep_cc1: instance.sweep_cc1,
                },
            )
        })();
        let resumed = self.resume_audio();
        self.status = match result {
            Ok(favorite) => {
                self.mark_favorite(id, &favorite);
                self.favorites.show = true;
                self.filter.clear();
                self.save_session();
                if self.favorites.library.entries.len() == previous_count {
                    format!("Favorite already saved: {}", favorite.name)
                } else {
                    format!("Added favorite: {}", favorite.name)
                }
            }
            Err(e) => format!("Could not add favorite: {e}"),
        };
        if let Err(e) = resumed {
            self.status.push_str(&format!("; no audio: {e}"));
        }
    }

    pub(crate) fn mark_favorite(&mut self, id: i32, favorite: &Favorite) {
        self.favorites.active.retain(|(instance, favorite_id)| {
            *instance != id
                && self.instances.iter().any(|i| i.id == *instance)
                && self
                    .favorites
                    .library
                    .entries
                    .iter()
                    .any(|f| &f.id == favorite_id)
        });
        self.favorites.active.push((id, favorite.id.clone()));
    }

    pub(crate) fn restore_favorite_selection(&mut self, id: i32) {
        self.initialize_favorites();
        let Some(instance) = self.instances.iter().find(|i| i.id == id) else {
            return;
        };
        if let Some(favorite) = self
            .favorites
            .take_restored(instance.kind, &instance.plugin)
        {
            self.mark_favorite(id, &favorite);
        }
    }

    /// Called before replacing a slot, while the entire audio chain is stopped.
    pub(crate) fn save_automatic_favorite(&mut self, id: i32) -> Result<(), String> {
        self.initialize_favorites();
        if let Some(error) = &self.favorites.error {
            return Err(error.clone());
        }
        let instance = self
            .instances
            .iter()
            .find(|i| i.id == id)
            .ok_or("instance not found")?;
        let state = self.host.save_state(id)?;
        let path = self.config_path.as_ref().map_err(Clone::clone)?;
        let favorite = self.favorites.library.add_automatic(
            path,
            instance.plugin.clone(),
            instance.kind == PluginKind::Effect,
            &state,
            FavoriteCapture {
                sequence_pattern: self.sequence_pattern,
                sweep_cc1: instance.sweep_cc1,
            },
        )?;
        self.mark_favorite(id, &favorite);
        Ok(())
    }

    pub(crate) fn load_favorite(&mut self, favorite_id: &str) {
        if self.pending.is_some() || self.scanning || self.restoring {
            return;
        }
        let result = self.prepare_favorite(favorite_id);
        if let Err(e) = result {
            self.status = format!("Could not load favorite: {e}");
        }
    }

    pub(crate) fn record_favorite_use(&mut self, favorite_id: &str) {
        let result = self
            .config_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| self.favorites.library.record_use(path, favorite_id));
        if let Err(error) = result {
            self.status
                .push_str(&format!("; could not save favorite order: {error}"));
        }
    }

    fn prepare_favorite(&mut self, favorite_id: &str) -> Result<(), String> {
        let favorite = self
            .favorites
            .library
            .entries
            .iter()
            .find(|f| f.id == favorite_id)
            .ok_or("favorite not found")?
            .clone();
        if favorite.effect && self.instrument_id().is_none() {
            return Err("Load an instrument first".into());
        }
        let path = self.config_path.as_ref().map_err(Clone::clone)?;
        // Read before touching the playing chain; a missing file leaves it intact.
        let state = self.favorites.library.state(path, favorite_id)?;
        let kind = if favorite.effect {
            PluginKind::Effect
        } else {
            PluginKind::Instrument
        };
        let existing = self
            .instances
            .iter()
            .find(|i| {
                i.kind == kind
                    && i.plugin.format == favorite.plugin.format
                    && i.plugin.id == favorite.plugin.id
            })
            .map(|i| i.id);
        if let Some(id) = existing {
            self.apply_favorite(id, &favorite, &state)?;
        } else {
            let index = favorite.plugin.find(&self.plugins).ok_or_else(|| {
                format!(
                    "Plugin not found: {} [{}]",
                    favorite.plugin.name, favorite.plugin.format
                )
            })?;
            self.plugins[index].kind = kind;
            self.load_plugin(index);
            if self.pending.is_some() {
                self.favorites.pending = Some((favorite, state));
            }
        }
        Ok(())
    }

    fn apply_favorite(&mut self, id: i32, favorite: &Favorite, state: &[u8]) -> Result<(), String> {
        self.pause_audio();
        let old_sequence = self.sequence_pattern;
        let old_sweep = self
            .instances
            .iter()
            .find(|i| i.id == id)
            .unwrap()
            .sweep_cc1;
        let mut backup = None;
        let result = (|| {
            backup = Some(self.host.save_state(id)?);
            self.host.load_state(id, state)?;
            self.instances
                .iter_mut()
                .find(|i| i.id == id)
                .unwrap()
                .sweep_cc1 = false;
            self.sequence_pattern = favorite.playback_pattern(self.sequence_pattern);
            self.resume_audio()
        })();
        if let Err(e) = result {
            self.pause_audio();
            self.sequence_pattern = old_sequence;
            self.instances
                .iter_mut()
                .find(|i| i.id == id)
                .unwrap()
                .sweep_cc1 = old_sweep;
            let rollback = backup
                .as_ref()
                .map(|data| self.host.load_state(id, data))
                .transpose();
            let resumed = self.resume_audio();
            return Err(format!(
                "{e}{}{}",
                rollback
                    .err()
                    .map_or(String::new(), |e| format!("; state rollback failed: {e}")),
                resumed
                    .err()
                    .map_or(String::new(), |e| format!("; no audio: {e}"))
            ));
        }
        self.mark_favorite(id, favorite);
        self.status = format!(
            "Loaded favorite: {}{}",
            favorite.name,
            if self.output.is_none() {
                "; no audio device"
            } else {
                ""
            }
        );
        self.record_favorite_use(&favorite.id);
        self.save_session();
        Ok(())
    }

    pub(crate) fn rename_favorite(&mut self, id: &str, name: &str) -> bool {
        let result = self
            .config_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| self.favorites.library.rename(path, id, name));
        match result {
            Ok(()) => {
                self.status = "Favorite renamed".into();
                true
            }
            Err(e) => {
                self.status = format!("Could not rename favorite: {e}");
                false
            }
        }
    }

    pub(crate) fn delete_favorite(&mut self, id: &str) {
        let result = self
            .config_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| self.favorites.library.delete(path, id));
        if !self.favorites.library.entries.iter().any(|f| f.id == id) {
            self.favorites.active.retain(|(_, favorite)| favorite != id);
        }
        self.status = match result {
            Ok(()) => {
                self.save_session();
                "Favorite deleted".into()
            }
            Err(e) => format!("Could not delete favorite: {e}"),
        };
    }
}

impl Favorites {
    // Match the slot and plugin identity; names and native instance IDs may change.
    fn take_restored(
        &mut self,
        kind: PluginKind,
        plugin: &crate::config::PluginKey,
    ) -> Option<Favorite> {
        let id = match kind {
            PluginKind::Instrument => self.restore.instrument.take(),
            PluginKind::Effect => {
                if self.restore.effects.is_empty() {
                    None
                } else {
                    Some(self.restore.effects.remove(0))
                }
            }
            PluginKind::Unknown => return None,
        }?;
        self.library
            .entries
            .iter()
            .find(|favorite| {
                favorite.id == id
                    && favorite.effect == (kind == PluginKind::Effect)
                    && favorite.plugin.format == plugin.format
                    && favorite.plugin.id == plugin.id
            })
            .cloned()
    }
}
