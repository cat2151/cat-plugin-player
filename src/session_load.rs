//! Asynchronous loading and restoration of the instrument/effect slots.
use crate::{audio, config, on_instance, plugin_list::PluginKind, startup, App, Instance};

impl App {
    pub(crate) fn load_plugin(&mut self, index: usize) {
        if self.pending.is_some() || self.scanning {
            return;
        }
        let plugin = self.plugins[index].clone();
        if plugin.kind == PluginKind::Effect && self.instrument_id().is_none() {
            self.status = "Load an instrument first".into();
            return;
        }
        self.pause_audio();
        let previous = if plugin.kind == PluginKind::Effect {
            self.effect_id()
        } else {
            self.instrument_id()
        };
        if let Some(id) = previous {
            if let Err(error) = self.save_plugin_state(id) {
                self.load_failed(format!("Could not save previous plugin: {error}"));
                return;
            }
            let different_plugin = self.instances.iter().any(|i| {
                i.id == id && (i.plugin.format != plugin.format || i.plugin.id != plugin.id)
            });
            if different_plugin && !self.restoring {
                if let Err(error) = self.save_automatic_favorite(id) {
                    self.load_failed(format!("Could not save automatic favorite: {error}"));
                    return;
                }
            }
        }
        self.status = format!("Loading {}...", plugin.name);
        // The old slot remains alive until creation, state restoration and chain
        // preparation all succeed. A failed replacement resumes that old chain.
        self.pending = Some(plugin.clone());
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
                self.restore_effect = None;
                self.status = format!("Previous instrument not found: {} [{}]", key.id, key.format);
            }
        }
    }

    pub(crate) fn instance_created(&mut self, id: i32, error: Option<String>) {
        startup::mark(startup::Stage::InstanceHandled);
        let Some(plugin) = self.pending.take() else {
            return;
        };
        let favorite = self.favorites.pending.take();
        let label = format!("{} [{}]", plugin.name, plugin.format);
        if let Some(error) = error.or_else(|| (id < 0).then(|| "creation failed".into())) {
            self.load_failed(format!("Could not load {label}: {error}"));
            return;
        }
        let key = config::PluginKey::from_plugin(&plugin);
        let restored = match &favorite {
            Some((_, state)) => self.host.load_state(id, state),
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
        let effect = if kind == PluginKind::Effect {
            Some(id)
        } else {
            self.effect_id()
        };
        let wait_for_effect =
            self.restoring && kind == PluginKind::Instrument && self.restore_effect.is_some();
        let old_sequence = self.sequence_pattern;
        if let Some((favorite, _)) = &favorite {
            self.sequence_pattern = favorite.playback_pattern(self.sequence_pattern);
        }
        let prepared = self
            .chain_processor(instrument, effect)
            .and_then(|processor| {
                if wait_for_effect || self.output.is_none() {
                    return Ok(None);
                }
                startup::mark(startup::Stage::ProcessorReady);
                audio::Voice::start(
                    self.output.as_ref().unwrap(),
                    processor,
                    self.sequence_pattern,
                    self.sequence_velocity,
                    self.sequence_modulation,
                )
                .map(Some)
            });
        let voice = match prepared {
            Ok(voice) => voice,
            Err(error) => {
                self.sequence_pattern = old_sequence;
                self.host.destroy_instance(id);
                self.load_failed(format!("Could not connect {label}: {error}"));
                return;
            }
        };
        if let Some(previous) = self.instances.iter().find(|i| i.kind == kind).map(|i| i.id) {
            self.instances.retain(|i| i.id != previous);
            self.host.destroy_instance(previous);
        }
        self.instances.push(Instance {
            id,
            label: label.clone(),
            plugin: key.clone(),
            kind,
            voice: None,
        });
        self.instances.sort_by_key(|i| i.kind);
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
        self.favorites.pending = None;
        self.pending = None;
        self.fast_restore = false;
        self.restoring = false;
        self.restore = None;
        self.restore_effect = None;
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
        let Some(key) = self.restore_effect.clone() else {
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
            self.restore_effect = None;
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
