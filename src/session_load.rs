//! Asynchronous loading and restoration of the instrument/effect slots.
use crate::{audio, config, on_instance, plugin_list::PluginKind, startup, App, Instance};

pub(crate) struct PendingLoad {
    pub plugin: crate::ffi::PluginInfo,
    pub purpose: LoadPurpose,
    pub data_home: Option<crate::plugin_specific::StartupDataHome>,
}
pub(crate) enum LoadPurpose {
    Manual,
    AutoInstrument,
    BrowserEffect,
    Restore,
    Favorite(
        crate::favorites_store::Favorite,
        Vec<u8>,
        crate::favorite_playback::Playback,
    ),
    Patch(Box<crate::patch_apply::Replacement>),
    EffectPreset(crate::effect_apply::Replacement),
    /// Reloads the instance with this id with the normal data home, then opens its editor.
    Editor(i32),
}

impl App {
    pub(crate) fn load_plugin(&mut self, index: usize) {
        if self.pending.is_some() || self.scanning || self.preset_change_busy() {
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
            data_home: None,
        });
        startup::mark(startup::Stage::LoadRequested);
        unsafe {
            self.host.create_instance(
                plugin.index,
                self.sample_rate(),
                audio::MAX_BLOCK_FRAMES,
                on_instance,
                std::ptr::null_mut(),
            )
        };
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
        let Some(mut pending) = self.pending.take() else {
            return;
        };
        let minimal_data_home = pending.data_home.take().is_some();
        let plugin = pending.plugin;
        let auto_play = matches!(pending.purpose, LoadPurpose::AutoInstrument);
        let browser_effect = matches!(pending.purpose, LoadPurpose::BrowserEffect);
        let editor_of = match pending.purpose {
            LoadPurpose::Editor(previous) => Some(previous),
            _ => None,
        };
        let favorite = match pending.purpose {
            LoadPurpose::EffectPreset(prepared) => {
                self.effect_instance_created(plugin, prepared, id, error);
                return;
            }
            LoadPurpose::Patch(replacement) => {
                self.patch_instance_created(plugin, *replacement, id, error);
                return;
            }
            LoadPurpose::Favorite(favorite, state, playback) => Some((favorite, state, playback)),
            LoadPurpose::Manual
            | LoadPurpose::Restore
            | LoadPurpose::AutoInstrument
            | LoadPurpose::BrowserEffect
            | LoadPurpose::Editor(_) => None,
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
        if auto_play {
            self.sequence_pattern = self.sequence_pattern.selection(self.selected_sequence);
        }
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
            minimal_data_home,
            preset_name: self.selection.as_ref().map_or_else(
                || {
                    self.config_path
                        .as_ref()
                        .ok()
                        .and_then(|path| crate::state_store::load_preset_name(path, &key))
                },
                |selection| {
                    selection
                        .settings
                        .instrument
                        .iter()
                        .chain(&selection.settings.effects)
                        .find(|slot| slot.plugin.format == key.format && slot.plugin.id == key.id)
                        .and_then(|slot| slot.preset_name.clone())
                },
            ),
        });
        if browser_effect && self.effect_browser.open {
            self.effect_browser.target = Some(id);
        }
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
        if let Some(previous) = editor_of {
            // The same sound continues in the new instance.
            for (instance, _) in &mut self.favorites.active {
                if *instance == previous {
                    *instance = id;
                }
            }
        }
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
        if editor_of.is_some() {
            self.show_editor(id);
        }
        if wait_for_effect {
            self.restore_effect_after_instrument();
            return;
        }
        self.fast_restore = false;
        self.restoring = false;
        // Selection mode opens the browser before the chain exists.
        if self.effect_browser.open {
            let ids = self.effect_ids();
            self.effect_browser.resolve_restored_slot(&ids);
        }
        self.selection_committed();
        self.save_session();
    }

    pub(crate) fn load_failed(&mut self, message: String) {
        self.pending = None;
        if self.restoring {
            if let Some(selection) = &mut self.selection {
                selection.restoration_failed = true;
            }
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

    /// An instance created with Surge XT's minimal data home is first reloaded with
    /// the normal one, so that its editor lists the installed patches and wavetables.
    pub(crate) fn open_editor(&mut self, id: i32) {
        let Some(instance) = self.instances.iter().find(|i| i.id == id) else {
            return;
        };
        if !instance.minimal_data_home {
            self.show_editor(id);
            return;
        }
        if self.pending.is_some() {
            return;
        }
        if self.actions_busy() {
            self.status = "Wait for loading to finish, then open the plugin UI again".into();
            return;
        }
        let Some(index) = instance.plugin.find(&self.plugins) else {
            self.status = format!("Plugin not found: {}", instance.label);
            return;
        };
        self.load_plugin(index);
        if let Some(pending) = &mut self.pending {
            pending.purpose = LoadPurpose::Editor(id);
        }
    }

    fn show_editor(&mut self, id: i32) {
        if let Err(code) = self.host.show_ui(id) {
            self.status = format!("Could not open the plugin UI (code {code})");
        }
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
