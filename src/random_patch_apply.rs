//! Main-thread patch transaction. Disk writes begin only after native commit.
use crate::{
    audio,
    config::PluginKey,
    ffi::PluginInfo,
    on_instance,
    plugin_list::PluginKind,
    random_patch::Prepared,
    session_load::{LoadPurpose, PendingLoad},
    state_store, App, Instance, SequencePattern,
};

pub(crate) struct Replacement {
    prepared: Prepared,
    old_pattern: SequencePattern,
    old_sweeps: Vec<(i32, bool)>,
    old_state: Option<(PluginKey, Vec<u8>)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    Backup,
    Apply,
    Chain,
    Rollback,
    Resume,
    Create,
}

pub(crate) fn backup(key: PluginKey, state: Vec<u8>) -> Result<(PluginKey, Vec<u8>), String> {
    if key.format == "CLAP" && key.id == crate::random_patch_catalog::SURGE_ID && state.is_empty() {
        Err("empty Surge instrument backup".into())
    } else {
        Ok((key, state))
    }
}

impl App {
    fn random_operation<T>(
        &mut self,
        operation: Operation,
        action: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        #[cfg(test)]
        if self.random_failures.contains(&operation) {
            return Err("injected random transaction failure".into());
        }
        #[cfg(not(test))]
        let _ = operation;
        action(self)
    }

    pub(crate) fn apply_random_patch(&mut self, prepared: Prepared) {
        let candidate = &prepared.candidate;
        // Resolve current scan identity again, never retain an index in a candidate.
        let Some(plugin) = self
            .plugins
            .iter()
            .find(|plugin| {
                plugin.format == candidate.format
                    && plugin.id == candidate.plugin_id
                    && plugin.kind == PluginKind::Instrument
            })
            .cloned()
        else {
            self.status = "Random patch failed: target plugin no longer available".into();
            return;
        };
        self.pause_audio();
        let old_pattern = self.sequence_pattern;
        let old_sweeps: Vec<_> = self.instances.iter().map(|i| (i.id, i.sweep_cc1)).collect();
        let old_state = match self
            .instrument_id()
            .map(|id| {
                self.random_operation(Operation::Backup, |app| app.host.save_state(id))
                    .and_then(|state| {
                        backup(
                            self.instances
                                .iter()
                                .find(|i| i.id == id)
                                .unwrap()
                                .plugin
                                .clone(),
                            state,
                        )
                    })
            })
            .transpose()
        {
            Ok(state) => state,
            Err(error) => {
                self.random_failed(
                    old_pattern,
                    &old_sweeps,
                    format!("Random patch failed: {error}"),
                );
                return;
            }
        };
        self.sequence_pattern = old_pattern;
        let replacement = Replacement {
            prepared,
            old_pattern,
            old_sweeps,
            old_state,
        };
        let existing = self
            .instances
            .iter()
            .find(|i| {
                i.kind == PluginKind::Instrument
                    && i.plugin.format == plugin.format
                    && i.plugin.id == plugin.id
            })
            .map(|i| i.id);
        if let Some(id) = existing {
            self.random_current(id, replacement);
        } else {
            self.status = format!("Loading random patch: {}...", plugin.name);
            self.pending = Some(PendingLoad {
                plugin: plugin.clone(),
                purpose: LoadPurpose::Random(replacement),
            });
            if let Err(error) = self.random_operation(Operation::Create, |_| Ok(())) {
                self.instance_created(-1, Some(error));
                return;
            }
            self.host.create_instance(
                plugin.index,
                self.sample_rate(),
                audio::MAX_BLOCK_FRAMES,
                on_instance,
                std::ptr::null_mut(),
            );
        }
    }

    fn random_current(&mut self, id: i32, replacement: Replacement) {
        let result = self
            .random_operation(Operation::Apply, |app| {
                app.host.load_state(id, &replacement.prepared.state)
            })
            .and_then(|()| {
                self.random_operation(Operation::Chain, |app| {
                    app.prepare_voice(id, &app.effect_ids(), false, None)
                })
            });
        match result {
            Ok(voice) => {
                let instance = self.instances.iter_mut().find(|i| i.id == id).unwrap();
                instance.voice = voice;
                instance.sweep_cc1 = false;
                self.unsafe_state.remove(&id);
                self.random_succeeded(id, replacement);
            }
            Err(error) => {
                self.pause_audio();
                let state = &replacement.old_state.as_ref().unwrap().1;
                let rollback = self
                    .random_operation(Operation::Rollback, |app| app.host.load_state(id, state));
                if rollback.is_err() {
                    self.unsafe_state.insert(id);
                }
                self.random_failed(
                    replacement.old_pattern,
                    &replacement.old_sweeps,
                    format!(
                        "Random patch failed: {error}{}",
                        rollback.err().map_or(String::new(), |error| format!(
                            "; state rollback failed: {error}; saving suppressed"
                        ))
                    ),
                );
            }
        }
    }

    pub(crate) fn random_instance_created(
        &mut self,
        plugin: PluginInfo,
        replacement: Replacement,
        id: i32,
        error: Option<String>,
    ) {
        if let Some(error) = error.or_else(|| (id < 0).then(|| "creation failed".into())) {
            if id >= 0 {
                self.host.destroy_instance(id);
            }
            self.random_failed(
                replacement.old_pattern,
                &replacement.old_sweeps,
                format!("Random patch failed: {error}"),
            );
            return;
        }
        let key = PluginKey::from_plugin(&plugin);
        let result = self
            .random_operation(Operation::Apply, |app| {
                app.host.load_state(id, &replacement.prepared.state)
            })
            .and_then(|()| {
                self.random_operation(Operation::Chain, |app| {
                    app.prepare_voice(id, &app.effect_ids(), false, Some(&key))
                })
            });
        let voice = match result {
            Ok(voice) => voice,
            Err(error) => {
                self.host.destroy_instance(id);
                self.random_failed(
                    replacement.old_pattern,
                    &replacement.old_sweeps,
                    format!("Random patch failed: {error}"),
                );
                return;
            }
        };
        if let Some(previous) = self.instrument_id() {
            self.instances.retain(|i| i.id != previous);
            self.unsafe_state.remove(&previous);
            self.host.destroy_instance(previous);
        }
        self.instances.insert(
            0,
            Instance {
                id,
                label: format!("{} [{}]", plugin.name, plugin.format),
                plugin: key.clone(),
                kind: PluginKind::Instrument,
                voice,
                sweep_cc1: false,
            },
        );
        self.restored = Some(key);
        self.random_succeeded(id, replacement);
    }

    fn random_failed(&mut self, pattern: SequencePattern, sweeps: &[(i32, bool)], message: String) {
        self.sequence_pattern = pattern;
        for &(id, sweep) in sweeps {
            if let Some(instance) = self.instances.iter_mut().find(|i| i.id == id) {
                instance.sweep_cc1 = sweep;
            }
        }
        let resumed = self.random_operation(Operation::Resume, |app| app.resume_audio());
        self.status = format!(
            "{message}{}",
            resumed
                .err()
                .map_or(String::new(), |error| format!("; no audio: {error}"))
        );
    }

    fn random_succeeded(&mut self, id: i32, replacement: Replacement) {
        self.favorites.active.retain(|(instance, _)| {
            *instance != id && self.instances.iter().any(|i| i.id == *instance)
        });
        self.status = format!(
            "Random patch: {} — {}{}",
            replacement.prepared.candidate.plugin_name,
            replacement.prepared.candidate.display,
            if self.output.is_none() {
                "; no audio device"
            } else {
                ""
            }
        );
        // Save both old and applied bytes with the existing state store. No favorite capture.
        self.pause_audio();
        let saved = (|| {
            let path = self.config_path.as_ref().map_err(Clone::clone)?;
            if let Some((key, state)) = &replacement.old_state {
                let current = &self.instances.iter().find(|i| i.id == id).unwrap().plugin;
                if key.format != current.format || key.id != current.id {
                    state_store::save(path, key, state)?;
                }
            }
            // Surge may accept state.load before its audio thread installs it.
            // Persist the successful owned input rather than rereading old sound.
            let current = &self.instances.iter().find(|i| i.id == id).unwrap().plugin;
            state_store::save(path, current, &replacement.prepared.state)
        })();
        let resumed = self.resume_audio();
        self.save_session();
        let error = saved.err().or_else(|| self.config_error.clone());
        if let Some(error) = error {
            self.status
                .push_str(&format!("; applied, saving failed: {error}"));
        }
        if let Err(error) = resumed {
            self.status
                .push_str(&format!("; applied, no audio: {error}"));
        }
    }
}
