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
    old_state: Option<(PluginKey, Vec<u8>)>,
    old_name: Option<String>,
    #[cfg(test)]
    create_started: Option<std::time::Instant>,
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
        #[cfg(test)]
        let _span = crate::session::headless::span(match operation {
            Operation::Backup => "backup",
            Operation::Apply => "load",
            Operation::Chain => "route",
            Operation::Rollback => "rollback",
            Operation::Resume => "resume",
            Operation::Create => "create_request",
        });
        action(self)
    }

    pub(crate) fn apply_patch(&mut self, prepared: Prepared) {
        #[cfg(test)]
        let _span = crate::session::headless::span("apply_transaction");
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
            self.drum_patch_finished(false);
            return;
        };
        let old_name = self
            .instances
            .iter()
            .find(|i| i.kind == PluginKind::Instrument)
            .and_then(|i| i.preset_name.clone());
        self.pause_audio();
        self.patch_changing();
        let old_pattern = self.sequence_pattern;
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
                self.patch_failed(old_pattern, format!("Browse patch failed: {error}"));
                return;
            }
        };
        self.sequence_pattern = if prepared.candidate.measurement.drum_kit {
            SequencePattern::Off
        } else {
            old_pattern
        };
        let replacement = Replacement {
            prepared,
            old_pattern,
            old_state,
            old_name,
            #[cfg(test)]
            create_started: None,
        };
        let existing = self
            .instances
            .iter()
            .find(|i| {
                i.kind == PluginKind::Instrument
                    && i.plugin.format == plugin.format
                    && i.plugin.id == plugin.id
                    && (plugin.format != "CLAP"
                        || self.instrument_origin.as_ref().is_some_and(|origin| {
                            crate::retained_instruments::Identity::read(&plugin, self.sample_rate())
                                .as_ref()
                                == Some(origin)
                        }))
            })
            .map(|i| i.id);
        if let Some(id) = existing {
            self.patch_current(id, replacement);
        } else if let Some(retained) = self.take_retained(&plugin) {
            self.restore_retained_patch(retained, replacement);
        } else {
            #[cfg(test)]
            let replacement = Replacement {
                create_started: Some(std::time::Instant::now()),
                ..replacement
            };
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
        #[cfg(test)]
        let _span = crate::session::headless::span("instance_created_to_finish");
        #[cfg(test)]
        if let Some(start) = replacement.create_started {
            crate::session::headless::record_elapsed("create_wait", start);
        }
        if let Some(error) = error.or_else(|| (id < 0).then(|| "creation failed".into())) {
            if id >= 0 {
                self.host.destroy_instance(id);
            }
            self.patch_failed(
                replacement.old_pattern,
                format!("Browse patch failed: {error}"),
            );
            return;
        }
        let key = PluginKey::from_plugin(&plugin);
        let identity = crate::retained_instruments::Identity::read(&plugin, self.sample_rate());
        self.patch_replacement(
            Instance {
                id,
                label: format!("{} [{}]", plugin.name, plugin.format),
                plugin: key,
                kind: PluginKind::Instrument,
                voice: None,
                minimal_data_home: false,
                preset_name: None,
            },
            identity,
            replacement,
        );
    }

    pub(crate) fn patch_replacement(
        &mut self,
        mut instance: Instance,
        identity: Option<crate::retained_instruments::Identity>,
        replacement: Replacement,
    ) {
        let id = instance.id;
        let key = instance.plugin.clone();
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
                self.destroy_inactive(instance);
                self.patch_failed(
                    replacement.old_pattern,
                    format!("Browse patch failed: {error}"),
                );
                return;
            }
        };
        self.retire_browser_instrument();
        instance.voice = voice;
        self.instances.insert(0, instance);
        self.instrument_origin = identity;
        self.restored = Some(key);
        self.patch_succeeded(id, replacement);
    }

    fn patch_failed(&mut self, pattern: SequencePattern, message: String) {
        self.patch_browser.requests.finish(false);
        self.sequence_pattern = pattern;
        let resumed = self.patch_operation(Operation::Resume, |app| app.resume_audio());
        self.drum_patch_finished(false);
        self.status = format!(
            "{message}{}",
            resumed
                .err()
                .map_or(String::new(), |error| format!("; no audio: {error}"))
        );
    }

    fn patch_succeeded(&mut self, id: i32, replacement: Replacement) {
        #[cfg(test)]
        let _span = crate::session::headless::span("persist_and_finish");
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
        if self.resident_kit() {
            self.drum_patch_finished(true);
            return;
        }
        // Owned bytes need no native state access: keep the committed voice running.
        let saved = (|| {
            let path = self.config_path.as_ref().map_err(Clone::clone)?;
            if let Some((key, state)) = &replacement.old_state {
                let current = &self.instances.iter().find(|i| i.id == id).unwrap().plugin;
                if key.format != current.format || key.id != current.id {
                    state_store::save_named(path, key, state, replacement.old_name.as_deref())?;
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
        self.selection_committed();
        self.save_session();
        let error = saved.err().or_else(|| self.config_error.clone());
        if let Some(error) = error {
            self.status
                .push_str(&format!("; applied, saving failed: {error}"));
        }
        self.drum_patch_finished(true);
    }
}
