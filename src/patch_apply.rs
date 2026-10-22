//! Main-thread patch transaction. Disk writes begin only after native commit.
use crate::{
    audio,
    config::PluginKey,
    ffi::PluginInfo,
    on_instance,
    patch_preparation::Prepared,
    plugin_list::PluginKind,
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
    if key.format == "CLAP" && key.id == crate::patch_catalog::SURGE_ID && state.is_empty() {
        Err("empty Surge instrument backup".into())
    } else {
        Ok((key, state))
    }
}

impl App {
    fn patch_operation<T>(
        &mut self,
        operation: Operation,
        action: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        #[cfg(test)]
        if self.patch_failures.contains(&operation) {
            return Err("injected patch transaction failure".into());
        }
        #[cfg(not(test))]
        let _ = operation;
        action(self)
    }

    pub(crate) fn apply_patch(&mut self, prepared: Prepared) {
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
            self.status = "Browse patch failed: target plugin no longer available".into();
            self.patch_browser.requests.finish(false);
            return;
        };
        self.pause_audio();
        self.patch_changing();
        let old_pattern = self.sequence_pattern;
        let old_sweeps: Vec<_> = self.instances.iter().map(|i| (i.id, i.sweep_cc1)).collect();
        let old_state = match self
            .instrument_id()
            .map(|id| {
                self.patch_operation(Operation::Backup, |app| app.host.save_state(id))
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
                self.patch_failed(
                    old_pattern,
                    &old_sweeps,
                    format!("Browse patch failed: {error}"),
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
            self.patch_current(id, replacement);
        } else {
            self.status = format!("Loading Browse patch: {}...", plugin.name);
            self.pending = Some(PendingLoad {
                plugin: plugin.clone(),
                purpose: LoadPurpose::Patch(Box::new(replacement)),
                data_home: None,
            });
            if let Err(error) = self.patch_operation(Operation::Create, |_| Ok(())) {
                self.instance_created(-1, Some(error));
                return;
            }
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
    }

    fn patch_current(&mut self, id: i32, replacement: Replacement) {
        if !self.host.ui_is_visible(id) {
            self.surge_start_delay.editor_closed(id);
        }
        let permit = self.surge_start_delay.authorize(id, &replacement.prepared);
        let previous_permit = self.surge_start_delay.replace(id, permit);
        let result = self
            .patch_operation(Operation::Apply, |app| {
                app.host.load_state(id, &replacement.prepared.state)
            })
            .and_then(|()| {
                self.patch_operation(Operation::Chain, |app| {
                    app.prepare_voice(id, &app.effect_ids(), false, None)
                })
            });
        match result {
            Ok(voice) => {
                let instance = self.instances.iter_mut().find(|i| i.id == id).unwrap();
                instance.voice = voice;
                instance.sweep_cc1 = false;
                self.unsafe_state.remove(&id);
                self.patch_succeeded(id, replacement);
            }
            Err(error) => {
                self.surge_start_delay.replace(id, previous_permit);
                self.pause_audio();
                let state = &replacement.old_state.as_ref().unwrap().1;
                let rollback =
                    self.patch_operation(Operation::Rollback, |app| app.host.load_state(id, state));
                if rollback.is_err() {
                    self.unsafe_state.insert(id);
                }
                self.patch_failed(
                    replacement.old_pattern,
                    &replacement.old_sweeps,
                    format!(
                        "Browse patch failed: {error}{}",
                        rollback.err().map_or(String::new(), |error| format!(
                            "; state rollback failed: {error}; saving suppressed"
                        ))
                    ),
                );
            }
        }
    }

    pub(crate) fn patch_instance_created(
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
            self.patch_failed(
                replacement.old_pattern,
                &replacement.old_sweeps,
                format!("Browse patch failed: {error}"),
            );
            return;
        }
        let key = PluginKey::from_plugin(&plugin);
        self.surge_start_delay.forget(id);
        let permit = self.surge_start_delay.authorize(id, &replacement.prepared);
        self.surge_start_delay.replace(id, permit);
        let result = self
            .patch_operation(Operation::Apply, |app| {
                app.host.load_state(id, &replacement.prepared.state)
            })
            .and_then(|()| {
                self.patch_operation(Operation::Chain, |app| {
                    app.prepare_voice(id, &app.effect_ids(), false, Some(&key))
                })
            });
        let voice = match result {
            Ok(voice) => voice,
            Err(error) => {
                self.surge_start_delay.forget(id);
                self.host.destroy_instance(id);
                self.patch_failed(
                    replacement.old_pattern,
                    &replacement.old_sweeps,
                    format!("Browse patch failed: {error}"),
                );
                return;
            }
        };
        if let Some(previous) = self.instrument_id() {
            self.instances.retain(|i| i.id != previous);
            self.unsafe_state.remove(&previous);
            self.host.destroy_instance(previous);
            self.surge_start_delay.forget(previous);
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
                minimal_data_home: false,
                preset_name: None,
            },
        );
        self.restored = Some(key);
        self.patch_succeeded(id, replacement);
    }

    fn patch_failed(&mut self, pattern: SequencePattern, sweeps: &[(i32, bool)], message: String) {
        self.patch_browser.requests.finish(false);
        self.sequence_pattern = pattern;
        for &(id, sweep) in sweeps {
            if let Some(instance) = self.instances.iter_mut().find(|i| i.id == id) {
                instance.sweep_cc1 = sweep;
            }
        }
        let resumed = self.patch_operation(Operation::Resume, |app| app.resume_audio());
        self.status = format!(
            "{message}{}",
            resumed
                .err()
                .map_or(String::new(), |error| format!("; no audio: {error}"))
        );
    }

    fn patch_succeeded(&mut self, id: i32, replacement: Replacement) {
        self.instances
            .iter_mut()
            .find(|i| i.id == id)
            .unwrap()
            .preset_name = Some(replacement.prepared.candidate.display.clone());
        self.favorites.active.retain(|(instance, _)| {
            *instance != id && self.instances.iter().any(|i| i.id == *instance)
        });
        self.status = format!(
            "Browse patch: {} — {}{}",
            replacement.prepared.candidate.plugin_name,
            replacement.prepared.candidate.display,
            if self.output.is_none() {
                "; no audio device"
            } else {
                ""
            }
        );
        self.patch_browser.requests.finish(true);
        self.patch_browser.requests.applied = Some(replacement.prepared.candidate.clone());
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
            state_store::save_named(
                path,
                current,
                &replacement.prepared.state,
                Some(&replacement.prepared.candidate.display),
            )
        })();
        let resumed = self.resume_audio();
        self.selection_committed();
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
