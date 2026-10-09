//! Session ownership, state persistence and changes to a stopped audio chain.
use crate::{audio, config, ffi, plugin_list::PluginKind, startup, state_store, App};

impl App {
    pub(crate) fn instrument_id(&self) -> Option<i32> {
        self.instances
            .iter()
            .find(|i| i.kind == PluginKind::Instrument)
            .map(|i| i.id)
    }

    pub(crate) fn effect_id(&self) -> Option<i32> {
        self.instances
            .iter()
            .find(|i| i.kind == PluginKind::Effect)
            .map(|i| i.id)
    }

    pub(crate) fn effect_ids(&self) -> Vec<i32> {
        self.instances
            .iter()
            .filter(|i| i.kind == PluginKind::Effect)
            .map(|i| i.id)
            .collect()
    }

    pub(crate) fn connected_effect(&self, key: &config::PluginKey) -> Option<i32> {
        self.instances
            .iter()
            .find(|i| {
                i.kind == PluginKind::Effect
                    && i.plugin.format == key.format
                    && i.plugin.id == key.id
            })
            .map(|i| i.id)
    }

    pub(crate) fn routing_label(&self) -> String {
        let mut labels: Vec<_> = self
            .instances
            .iter()
            .filter(|i| !self.effect_bypassed || i.kind != PluginKind::Effect)
            .map(|i| i.label.as_str())
            .collect();
        labels.push("Output");
        labels.join(" -> ")
    }

    pub(crate) fn chain_processor(
        &self,
        instrument: i32,
        effects: &[i32],
    ) -> Result<ffi::Processor, String> {
        self.host
            .create_chain(
                instrument,
                effects,
                self.effect_bypassed,
                self.sample_rate(),
                audio::MAX_BLOCK_FRAMES,
            )
            .ok_or_else(|| {
                "Cannot connect main audio buses (mono/stereo input and output required)".into()
            })
    }

    pub(crate) fn prepare_voice(
        &self,
        instrument: i32,
        effects: &[i32],
        waiting: bool,
        new_instrument: Option<&config::PluginKey>,
    ) -> Result<Option<audio::Voice>, String> {
        let processor = self.chain_processor(instrument, effects)?;
        if waiting || self.output.is_none() {
            return Ok(None);
        }
        startup::mark(startup::Stage::ProcessorReady);
        let instrument_plugin = new_instrument.or_else(|| {
            self.instances
                .iter()
                .find(|i| i.id == instrument)
                .map(|i| &i.plugin)
        });
        let start_delay = instrument_plugin
            .map(crate::plugin_specific::automatic_note_start_delay)
            .unwrap_or_default();
        audio::Voice::start(
            self.output.as_ref().unwrap(),
            processor,
            self.sequence_pattern,
            self.sequence_velocity,
            self.sequence_modulation,
            start_delay,
            self.mml_input.phrase.clone(),
        )
        .map(Some)
    }

    pub(crate) fn start_voice(&self, id: i32) -> Result<audio::Voice, String> {
        self.prepare_voice(id, &self.effect_ids(), false, None)?
            .ok_or_else(|| "no audio device".into())
    }

    pub(crate) fn pause_audio(&mut self) {
        for instance in &mut self.instances {
            if let Some(mut voice) = instance.voice.take() {
                self.sequence_pattern = voice.sequence_pattern();
                if let Some(sweep) = voice.stop_and_capture_sweep() {
                    instance.sweep_cc1 = sweep;
                }
            }
        }
    }

    pub(crate) fn resume_audio(&mut self) -> Result<(), String> {
        let Some(id) = self.instrument_id() else {
            return Ok(());
        };
        if self.output.is_none() {
            return Ok(());
        }
        let voice = self.start_voice(id)?;
        self.instances
            .iter_mut()
            .find(|i| i.id == id)
            .unwrap()
            .voice = Some(voice);
        Ok(())
    }

    pub(crate) fn save_session(&mut self) {
        if !self.unsafe_state.is_empty() || self.random_busy() {
            return;
        }
        self.selected_sequence = self.sequence_pattern.selection(self.selected_sequence);
        // Removing the source keeps its last identity, so the next launch can
        // still restore the last instrument, as it did before effect routing.
        let last_played = self
            .instances
            .iter()
            .find(|i| i.kind == PluginKind::Instrument)
            .map(|i| i.plugin.clone())
            .or_else(|| self.restored.clone());
        let status = crate::status::Status {
            show_favorites: Some(self.favorites.show),
            show_history: self.favorites.show_history,
            on_right: self.scope_ui.on_right,
            last_played,
            effects: self
                .instances
                .iter()
                .filter(|i| i.kind == PluginKind::Effect)
                .map(|i| i.plugin.clone())
                .collect(),
            effect_bypassed: self.effect_bypassed,
            favorites: crate::status::FavoriteSelection {
                instrument: self.active_favorite_id(PluginKind::Instrument),
                effects: self
                    .instances
                    .iter()
                    .filter(|i| i.kind == PluginKind::Effect)
                    .map(|i| {
                        self.favorites
                            .active
                            .iter()
                            .find(|(id, _)| *id == i.id)
                            .map(|(_, favorite)| favorite.clone())
                            .unwrap_or_default()
                    })
                    .collect(),
                ..Default::default()
            },
            sequence_pattern: self.sequence_pattern.persisted(),
            selected_sequence: self
                .sequence_pattern
                .selection(self.selected_sequence)
                .persisted(),
            sequence_velocity: self.sequence_velocity,
            sequence_modulation: self.sequence_modulation,
            ..Default::default()
        };
        self.config_error = self
            .config_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| status.save(path))
            .err()
            .map(|error| format!("Could not save session: {error}"));
    }

    fn active_favorite_id(&self, kind: PluginKind) -> Option<String> {
        let instance = self
            .instances
            .iter()
            .find(|instance| instance.kind == kind)?;
        self.favorites
            .active
            .iter()
            .find(|(id, _)| *id == instance.id)
            .map(|(_, favorite)| favorite.clone())
    }

    pub(crate) fn restore_plugin_state(
        &self,
        id: i32,
        key: &config::PluginKey,
    ) -> Result<(), String> {
        let path = self.config_path.as_ref().map_err(Clone::clone)?;
        if let Some(state) = state_store::load(path, key)? {
            self.host.load_state(id, &state)?;
        }
        Ok(())
    }

    pub(crate) fn save_plugin_state(&mut self, id: i32) -> Result<(), String> {
        if self.unsafe_state.contains(&id) {
            return Err("state saving suppressed after failed rollback".into());
        }
        // Any instance can be an effect in the instrument's stream. Stop the
        // whole stream before invoking a plugin state API.
        self.pause_audio();
        let instance = self
            .instances
            .iter()
            .find(|i| i.id == id)
            .ok_or("instance not found")?;
        let state = self.host.save_state(id)?;
        let path = self.config_path.as_ref().map_err(Clone::clone)?;
        state_store::save(path, &instance.plugin, &state)
    }

    pub(crate) fn remove_plugin(&mut self, id: i32) -> bool {
        if self.actions_busy() {
            return false;
        }
        if let Err(error) = self.save_plugin_state(id) {
            let resumed = self.resume_audio();
            self.config_error = Some(format!("Could not save plugin state: {error}"));
            self.status = format!(
                "Plugin kept because saving failed{}",
                resumed
                    .err()
                    .map_or(String::new(), |e| format!("; no audio: {e}"))
            );
            return false;
        }
        let effects: Vec<_> = self
            .effect_ids()
            .into_iter()
            .filter(|effect| *effect != id)
            .collect();
        let source = self.instrument_id().filter(|source| *source != id);
        let old_bypass = self.effect_bypassed;
        if effects.is_empty() {
            self.effect_bypassed = false;
        }
        let prepared = source
            .map(|source| self.prepare_voice(source, &effects, false, None))
            .transpose();
        let voice = match prepared {
            Ok(voice) => voice.flatten(),
            Err(error) => {
                self.effect_bypassed = old_bypass;
                self.load_failed(format!("Could not remove plugin: {error}"));
                return false;
            }
        };
        self.instances.retain(|i| i.id != id);
        self.favorites
            .active
            .retain(|(instance, _)| *instance != id);
        self.host.destroy_instance(id);
        if let Some(source) = source {
            self.instances
                .iter_mut()
                .find(|i| i.id == source)
                .unwrap()
                .voice = voice;
        }
        self.status = "Plugin state saved; removed".into();
        self.save_session();
        true
    }

    pub(crate) fn reorder_effect(&mut self, id: i32, target: i32, after: bool) -> bool {
        if self.actions_busy() || self.restoring || id == target {
            return false;
        }
        let mut effects = self.effect_ids();
        let Some(from) = effects.iter().position(|effect| *effect == id) else {
            return false;
        };
        if !effects.contains(&target) {
            return false;
        }
        effects.remove(from);
        let to = effects.iter().position(|effect| *effect == target).unwrap() + usize::from(after);
        effects.insert(to, id);
        if effects == self.effect_ids() {
            return false;
        }
        self.pause_audio();
        let voice = match self
            .instrument_id()
            .map(|source| self.prepare_voice(source, &effects, false, None))
            .transpose()
        {
            Ok(voice) => voice.flatten(),
            Err(error) => {
                self.load_failed(format!("Could not reorder effects: {error}"));
                return false;
            }
        };
        self.instances.sort_by_key(|instance| {
            if instance.kind == PluginKind::Instrument {
                0
            } else {
                effects.iter().position(|id| *id == instance.id).unwrap() + 1
            }
        });
        if let Some(source) = self.instrument_id() {
            self.instances
                .iter_mut()
                .find(|i| i.id == source)
                .unwrap()
                .voice = voice;
        }
        self.save_session();
        true
    }

    pub(crate) fn set_effect_bypass(&mut self, bypass: bool) {
        if self.actions_busy() || self.effect_id().is_none() {
            return;
        }
        self.pause_audio();
        let previous = self.effect_bypassed;
        self.effect_bypassed = bypass;
        if let Err(error) = self.resume_audio() {
            self.effect_bypassed = previous;
            let restored = self.resume_audio();
            self.status = format!(
                "Could not change bypass: {error}{}",
                restored
                    .err()
                    .map_or(String::new(), |e| format!("; no audio: {e}"))
            );
            return;
        }
        self.save_session();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.save_on_shutdown();
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routing_tests.rs"]
mod routing_tests;
