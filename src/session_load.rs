//! Asynchronous loading and restoration of the instrument/effect slots.
use crate::{audio, config, on_instance, plugin_list::PluginKind, startup, App, Instance};

pub(crate) struct PendingLoad {
    pub plugin: crate::ffi::PluginInfo,
    pub purpose: LoadPurpose,
}
pub(crate) enum LoadPurpose {
    Manual,
    Restore,
    Favorite(
        crate::favorites_store::Favorite,
        Vec<u8>,
        crate::favorite_playback::Playback,
    ),
    Random(Box<crate::random_patch_apply::Replacement>),
    RandomEffect(crate::random_effect_apply::Replacement),
}

impl App {
    pub(crate) fn load_plugin(&mut self, index: usize) {
        if self.pending.is_some() || self.scanning || self.random_busy() {
            return;
        }
        let plugin = self.plugins[index].clone();
        // Restoring reloads the saved state; an effect leaves the instrument patch alone.
        if self.restoring || plugin.kind == PluginKind::Effect {
            self.patch_browser.requests.invalidate();
        } else {
            self.invalidate_browser_live();
        }
        if plugin.kind == PluginKind::Effect && self.instrument_id().is_none() {
            self.status = "Load an instrument first".into();
            return;
        }
        if plugin.kind == PluginKind::Effect
            && self
                .connected_effect(&config::PluginKey::from_plugin(&plugin))
                .is_some()
        {
            if self.restoring {
                self.load_failed("Duplicate saved effect identity".into());
            }
            return;
        }
        self.pause_audio();
        let previous = if plugin.kind == PluginKind::Effect {
            None
        } else {
            self.instrument_id()
        };
        if let Some(id) = previous.filter(|id| !self.unsafe_state.contains(id)) {
            if let Err(error) = self.save_plugin_state(id) {
                self.load_failed(format!("Could not save previous plugin: {error}"));
                return;
            }
            let different_plugin = self.instances.iter().any(|i| {
                i.id == id && (i.plugin.format != plugin.format || i.plugin.id != plugin.id)
            });
            if different_plugin && !self.restoring {
                if let Err(error) = self.save_automatic_favorite(id) {
                    self.load_failed(format!("Could not record history: {error}"));
                    return;
                }
            }
        }
        self.status = format!("Loading {}...", plugin.name);
        // The old slot remains alive until creation, state restoration and chain
        // preparation all succeed. A failed replacement resumes that old chain.
        self.pending = Some(PendingLoad {
            plugin: plugin.clone(),
            purpose: if self.restoring {
                LoadPurpose::Restore
            } else {
                LoadPurpose::Manual
            },
        });
        startup::mark(startup::Stage::LoadRequested);
        self.host.create_instance(
            plugin.index,
            self.sample_rate(),
            audio::MAX_BLOCK_FRAMES,
            on_instance,
            std::ptr::null_mut(),
        );
        startup::mark(startup::Stage::LoadReturned);
    }

    pub(crate) fn restore_session(&mut self) {
        if self.instrument_id().is_some() && self.restoring {
            self.restore_effect_after_instrument();
            return;
        }
        let Some(key) = self.restore.take() else {
            return;
        };
        match key.find(&self.plugins) {
            Some(index) => {
                self.restoring = true;
                // The saved slot defines the role, even if catalog metadata is unknown.
                self.plugins[index].kind = PluginKind::Instrument;
                self.load_plugin(index);
            }
            None => {
                self.restore_effect.clear();
                self.status = format!("Previous instrument not found: {} [{}]", key.id, key.format);
            }
        }
    }

    pub(crate) fn instance_created(&mut self, id: i32, error: Option<String>) {
        startup::mark(startup::Stage::InstanceHandled);
        let Some(pending) = self.pending.take() else {
            return;
        };
        let plugin = pending.plugin;
        let favorite = match pending.purpose {
            LoadPurpose::RandomEffect(prepared) => {
                self.random_effect_instance_created(plugin, prepared, id, error);
                return;
            }
            LoadPurpose::Random(replacement) => {
                self.random_instance_created(plugin, *replacement, id, error);
                return;
            }
            LoadPurpose::Favorite(favorite, state, playback) => Some((favorite, state, playback)),
            LoadPurpose::Manual | LoadPurpose::Restore => None,
        };
        let label = format!("{} [{}]", plugin.name, plugin.format);
        if let Some(error) = error.or_else(|| (id < 0).then(|| "creation failed".into())) {
            self.load_failed(format!("Could not load {label}: {error}"));
            return;
        }
        let key = config::PluginKey::from_plugin(&plugin);
        let restored = match &favorite {
            Some((_, state, _)) => self.host.load_state(id, state),
            None => self.restore_plugin_state(id, &key),
        };
        if let Err(error) = restored {
            self.host.destroy_instance(id);
            self.load_failed(format!("Could not restore {label}: {error}"));
            return;
        }
        let kind = if plugin.kind == PluginKind::Effect {
            PluginKind::Effect
        } else {
            PluginKind::Instrument
        };
        let instrument = if kind == PluginKind::Instrument {
            id
        } else {
            self.instrument_id().unwrap()
        };
        let mut effects = self.effect_ids();
        if kind == PluginKind::Effect {
            effects.push(id);
        }
        let wait_for_effect = self.restoring && !self.restore_effect.is_empty();
        let old_sequence = self.sequence_pattern;
        let old_selected = self.selected_sequence;
        let old_input = self.mml_input.clone();
        let favorite = favorite.map(|(favorite, state, playback)| {
            self.apply_playback(playback);
            (favorite, state)
        });
        let new_instrument = (kind == PluginKind::Instrument).then_some(&key);
        let prepared = self.prepare_voice(instrument, &effects, wait_for_effect, new_instrument);
        let voice = match prepared {
            Ok(voice) => voice,
            Err(error) => {
                self.sequence_pattern = old_sequence;
                self.selected_sequence = old_selected;
                self.mml_input = old_input;
                self.host.destroy_instance(id);
                self.load_failed(format!("Could not connect {label}: {error}"));
                return;
            }
        };
        if let Some(previous) = self
            .instances
            .iter()
            .find(|i| kind == PluginKind::Instrument && i.kind == kind)
            .map(|i| i.id)
        {
            self.instances.retain(|i| i.id != previous);
            self.unsafe_state.remove(&previous);
            self.host.destroy_instance(previous);
        }
        self.instances.push(Instance {
            id,
            label: label.clone(),
            plugin: key.clone(),
            kind,
            voice: None,
            sweep_cc1: false,
        });
        // Keep source at index zero without changing the effect order.
        if kind == PluginKind::Instrument {
            let source = self.instances.pop().unwrap();
            self.instances.insert(0, source);
        }
        self.instances
            .iter_mut()
            .find(|i| i.id == instrument)
            .unwrap()
            .voice = voice;
        if kind == PluginKind::Instrument {
            self.restored = Some(key);
            self.restore = None;
        }
        self.status = if self.output.is_some() {
            format!("Loaded {label}")
        } else {
            format!("Loaded {label}, but no audio device")
        };
        self.favorites
            .active
            .retain(|(instance, _)| self.instances.iter().any(|i| i.id == *instance));
        if let Some((favorite, _)) = favorite {
            self.mark_favorite(id, &favorite);
            self.status = format!(
                "Loaded favorite: {}{}",
                favorite.name,
                if self.output.is_none() {
                    "; no audio device"
                } else {
                    ""
                }
            );
        } else if self.restoring {
            self.restore_favorite_selection(id);
        }
        if wait_for_effect {
            self.restore_effect_after_instrument();
            return;
        }
        self.fast_restore = false;
        self.restoring = false;
        self.save_session();
    }

    pub(crate) fn load_failed(&mut self, message: String) {
        self.pending = None;
        if self.restoring {
            self.pause_audio();
            let effects = self.effect_ids();
            self.instances.retain(|i| i.kind != PluginKind::Effect);
            for id in effects {
                self.host.destroy_instance(id);
            }
            self.favorites
                .active
                .retain(|(id, _)| self.instances.iter().any(|i| i.id == *id));
            self.effect_bypassed = false;
        }
        self.fast_restore = false;
        self.restoring = false;
        self.restore = None;
        self.restore_effect.clear();
        if self.effect_id().is_none() {
            self.effect_bypassed = false;
        }
        self.config_error = Some(message.clone());
        self.status = match self.resume_audio() {
            Ok(()) => message,
            Err(error) => format!("{message}; no audio: {error}"),
        };
        // Keep the disk session intact on failure, including a missing effect.
    }

    fn restore_effect_after_instrument(&mut self) {
        let Some(key) = self.restore_effect.front().cloned() else {
            return;
        };
        let index = key.find(&self.plugins).or_else(|| {
            self.host.restore_plugin(&key).map(|plugin| {
                self.plugins.push(plugin);
                self.plugins.len() - 1
            })
        });
        if let Some(index) = index {
            self.plugins[index].kind = PluginKind::Effect;
            self.restore_effect.pop_front();
            self.load_plugin(index);
        } else if self.fast_restore {
            // A relocated bundle may still be found by the deferred scan. Keep
            // audio stopped until that scan has resolved the saved effect.
            self.fast_restore = false;
        } else {
            self.load_failed(format!(
                "Previous effect not found: {} [{}]; playing instrument directly",
                key.id, key.format
            ));
        }
    }
}
