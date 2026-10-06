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

    pub(crate) fn routing_label(&self) -> String {
        let label = |kind| {
            self.instances
                .iter()
                .find(|i| i.kind == kind)
                .map_or("(empty)", |i| i.label.as_str())
        };
        format!(
            "{} -> {}{} -> Output",
            label(PluginKind::Instrument),
            label(PluginKind::Effect),
            if self.effect_bypassed {
                " [bypassed]"
            } else {
                ""
            }
        )
    }

    pub(crate) fn chain_processor(
        &self,
        instrument: i32,
        effect: Option<i32>,
    ) -> Result<ffi::Processor, String> {
        let effects: Vec<_> = effect.into_iter().collect();
        self.host
            .create_chain(
                instrument,
                &effects,
                self.effect_bypassed,
                self.sample_rate(),
                audio::MAX_BLOCK_FRAMES,
            )
            .ok_or_else(|| {
                "Cannot connect main audio buses (mono/stereo input and output required)".into()
            })
    }

    pub(crate) fn start_voice(&self, id: i32) -> Result<audio::Voice, String> {
        let output = self.output.as_ref().ok_or("no audio device")?;
        let processor = self.chain_processor(id, self.effect_id())?;
        startup::mark(startup::Stage::ProcessorReady);
        audio::Voice::start(
            output,
            processor,
            self.sequence_pattern,
            self.sequence_velocity,
            self.sequence_modulation,
        )
    }

    pub(crate) fn pause_audio(&mut self) {
        for instance in &mut self.instances {
            if let Some(voice) = &instance.voice {
                self.sequence_pattern = voice.sequence_pattern();
            }
            instance.voice = None;
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
        self.selected_sequence = self.sequence_pattern.selection(self.selected_sequence);
        // Removing the source keeps its last identity, so the next launch can
        // still restore the last instrument, as it did before effect routing.
        let last_played = self
            .instances
            .iter()
            .find(|i| i.kind == PluginKind::Instrument)
            .map(|i| i.plugin.clone())
            .or_else(|| self.restored.clone());
        let mut config = config::Config {
            last_played,
            effect: self
                .instances
                .iter()
                .find(|i| i.kind == PluginKind::Effect)
                .map(|i| i.plugin.clone()),
            effect_bypassed: self.effect_bypassed,
            favorites: config::FavoriteSelection {
                instrument: self.active_favorite_id(PluginKind::Instrument),
                effect: self.active_favorite_id(PluginKind::Effect),
            },
            sequence_pattern: self.sequence_pattern,
            selected_sequence: self.sequence_pattern.selection(self.selected_sequence),
            sequence_velocity: self.sequence_velocity,
            sequence_modulation: self.sequence_modulation,
            ..Default::default()
        };
        self.config_error = self
            .config_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| config.save_session(path))
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
        if self.pending.is_some() {
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
        self.instances.retain(|i| i.id != id);
        self.host.destroy_instance(id);
        if self.effect_id().is_none() {
            self.effect_bypassed = false;
        }
        self.status = match self.resume_audio() {
            Ok(()) => "Plugin state saved; removed".into(),
            Err(e) => format!("Removed, but no audio: {e}"),
        };
        self.save_session();
        true
    }

    pub(crate) fn set_effect_bypass(&mut self, bypass: bool) {
        if self.pending.is_some() || self.effect_id().is_none() {
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
        self.pause_audio();
        let ids: Vec<_> = self.instances.iter().map(|i| i.id).collect();
        for id in ids {
            if let Err(error) = self.save_plugin_state(id) {
                eprintln!("Could not save plugin state on exit: {error}");
            }
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routing_tests.rs"]
mod routing_tests;
