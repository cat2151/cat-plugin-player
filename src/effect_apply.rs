//! Keep the old chain alive until a fresh effect and its connection succeed.
use crate::{
    audio,
    config::PluginKey,
    effect_preparation::Prepared,
    ffi::PluginInfo,
    on_instance,
    plugin_list::PluginKind,
    session_load::{LoadPurpose, PendingLoad},
    state_store, App, Instance,
};

pub(crate) struct Replacement {
    pub prepared: Prepared,
    pub old_state: Option<(PluginKey, Vec<u8>)>,
}

impl App {
    pub(crate) fn apply_effect(&mut self, prepared: Prepared) {
        let Some(plugin) = self
            .plugins
            .iter()
            .find(|plugin| crate::effect_catalog::matches_plugin(&prepared.candidate, plugin))
            .cloned()
        else {
            self.status = "Browse effect failed: plugin no longer available".into();
            return;
        };
        if self.instrument_id().is_none()
            || prepared
                .target
                .is_some_and(|id| !self.effect_ids().contains(&id))
            || self.instances.iter().any(|instance| {
                instance.kind == PluginKind::Effect
                    && instance.plugin.format == plugin.format
                    && instance.plugin.id == plugin.id
                    && Some(instance.id) != prepared.target
            })
        {
            self.status = "Browse effect failed: routing changed during preparation".into();
            return;
        }
        self.start_effect_swap_fade(&plugin, prepared.target);
        self.pause_audio();
        let old_state = match prepared
            .target
            .map(|target| {
                let instance = self
                    .instances
                    .iter()
                    .find(|instance| instance.id == target)
                    .unwrap();
                self.host
                    .save_state(target)
                    .map(|state| (instance.plugin.clone(), state))
            })
            .transpose()
        {
            Ok(state) => state,
            Err(error) => {
                self.status =
                    format!("Browse effect failed: could not save previous state: {error}");
                if let Err(error) = self.resume_audio() {
                    self.status.push_str(&format!("; no audio: {error}"));
                }
                self.effect_swap_fade = false;
                return;
            }
        };
        self.status = format!("Loading effect: {}...", prepared.candidate.preset.display);
        self.pending = Some(PendingLoad {
            plugin: plugin.clone(),
            purpose: LoadPurpose::EffectPreset(Replacement {
                prepared,
                old_state,
            }),
        });
        unsafe {
            self.host.create_instance(
                plugin.index,
                self.sample_rate(),
                audio::MAX_BLOCK_FRAMES,
                on_instance,
                std::ptr::null_mut(),
            )
        };
    }

    /// ADR 0027: fade out now, and fade in the voices started until the swap ends.
    fn start_effect_swap_fade(&mut self, plugin: &PluginInfo, target: Option<i32>) {
        let old = target
            .and_then(|target| self.instances.iter().find(|instance| instance.id == target))
            .map(|instance| &instance.plugin);
        self.effect_swap_fade =
            crate::plugin_specific::fades_effect_swap(&PluginKey::from_plugin(plugin))
                || old.is_some_and(crate::plugin_specific::fades_effect_swap);
        if self.effect_swap_fade {
            for voice in self.instances.iter().filter_map(|i| i.voice.as_ref()) {
                voice.fade_out();
            }
        }
    }

    pub(crate) fn effect_instance_created(
        &mut self,
        plugin: PluginInfo,
        replacement: Replacement,
        id: i32,
        error: Option<String>,
    ) {
        let Replacement {
            prepared,
            old_state,
        } = replacement;
        let result = (|| {
            if let Some(error) = error.or_else(|| (id < 0).then(|| "creation failed".into())) {
                return Err(error);
            }
            self.host.load_state(id, &prepared.state)?;
            let mut effects = self.effect_ids();
            if let Some(target) = prepared.target {
                let slot = effects
                    .iter_mut()
                    .find(|effect| **effect == target)
                    .ok_or("replacement slot no longer connected")?;
                *slot = id;
            } else {
                effects.push(id);
            }
            self.prepare_voice(
                self.instrument_id()
                    .ok_or("instrument no longer connected")?,
                &effects,
                false,
                None,
            )
        })();
        let voice = match result {
            Ok(voice) => voice,
            Err(error) => {
                if id >= 0 {
                    self.host.destroy_instance(id);
                }
                self.status = format!("Browse effect failed: {error}");
                if let Err(error) = self.resume_audio() {
                    self.status.push_str(&format!("; no audio: {error}"));
                }
                self.effect_swap_fade = false;
                return;
            }
        };
        let key = PluginKey::from_plugin(&plugin);
        let instance = Instance {
            id,
            label: format!("{} [{}]", plugin.name, plugin.format),
            plugin: key.clone(),
            kind: PluginKind::Effect,
            voice: None,
            sweep_cc1: false,
            preset_name: Some(prepared.candidate.preset.display.clone()),
        };
        if let Some(target) = prepared.target {
            let slot = self
                .instances
                .iter_mut()
                .find(|instance| instance.id == target)
                .unwrap();
            *slot = instance;
            self.favorites
                .active
                .retain(|(instance, _)| *instance != target);
            self.unsafe_state.remove(&target);
            self.host.destroy_instance(target);
        } else {
            self.instances.push(instance);
        }
        let source = self.instrument_id().unwrap();
        self.instances
            .iter_mut()
            .find(|instance| instance.id == source)
            .unwrap()
            .voice = voice;
        self.status = format!("Browse effect: {}", prepared.candidate.preset.display);
        self.effect_browser.applied(id, prepared.candidate.clone());
        self.pause_audio();
        let saved = self
            .config_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| {
                if let Some((previous, state)) = old_state {
                    if previous.format != key.format || previous.id != key.id {
                        state_store::save(path, &previous, &state)?;
                    }
                }
                state_store::save_named(
                    path,
                    &key,
                    &prepared.state,
                    Some(&prepared.candidate.preset.display),
                )
            });
        let resumed = self.resume_audio();
        self.effect_swap_fade = false;
        self.selection_committed();
        self.save_session();
        if let Some(error) = saved.err().or_else(|| self.config_error.clone()) {
            self.status
                .push_str(&format!("; applied, saving failed: {error}"));
        }
        if let Err(error) = resumed {
            self.status
                .push_str(&format!("; applied, no audio: {error}"));
        }
    }
}
