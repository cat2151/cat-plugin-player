//! Keep the old chain alive until a fresh effect and its connection succeed.
use crate::{
    audio,
    config::PluginKey,
    ffi::PluginInfo,
    on_instance,
    plugin_list::PluginKind,
    random_effect::Prepared,
    session_load::{LoadPurpose, PendingLoad},
    state_store, App, Instance,
};

pub(crate) struct Replacement {
    pub prepared: Prepared,
    pub old_state: Option<(PluginKey, Vec<u8>)>,
}

impl App {
    pub(crate) fn apply_random_effect(&mut self, prepared: Prepared) {
        let Some(plugin) = self
            .plugins
            .iter()
            .find(|plugin| {
                crate::random_effect_catalog::matches_plugin(&prepared.candidate, plugin)
            })
            .cloned()
        else {
            self.status = format!(
                "{} failed: plugin no longer available",
                prepared.operation()
            );
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
            self.status = format!(
                "{} failed: routing changed during preparation",
                prepared.operation()
            );
            return;
        }
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
                self.status = format!(
                    "{} failed: could not save previous state: {error}",
                    prepared.operation()
                );
                if let Err(error) = self.resume_audio() {
                    self.status.push_str(&format!("; no audio: {error}"));
                }
                return;
            }
        };
        self.status = format!("Loading effect: {}...", prepared.candidate.preset.display);
        self.pending = Some(PendingLoad {
            plugin: plugin.clone(),
            purpose: LoadPurpose::RandomEffect(Replacement {
                prepared,
                old_state,
            }),
        });
        self.host.create_instance(
            plugin.index,
            self.sample_rate(),
            audio::MAX_BLOCK_FRAMES,
            on_instance,
            std::ptr::null_mut(),
        );
    }

    pub(crate) fn random_effect_instance_created(
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
                self.status = format!("{} failed: {error}", prepared.operation());
                if let Err(error) = self.resume_audio() {
                    self.status.push_str(&format!("; no audio: {error}"));
                }
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
        self.status = format!(
            "{}: {}",
            prepared.operation(),
            prepared.candidate.preset.display
        );
        if prepared.browse {
            self.effect_browser.applied(id, prepared.candidate.clone());
        }
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
                state_store::save(path, &key, &prepared.state)
            });
        let resumed = self.resume_audio();
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
