//! ADR 0034: release Surge's minimal DLL before normal creation, with bounded recovery.
use crate::{
    audio, config::PluginKey, ffi::PluginInfo, on_instance, session_load::*, App, Instance,
};

pub(crate) struct Rebuild {
    previous: Instance,
    pub state: Vec<u8>,
    error: Option<String>,
    failed: bool,
}

impl Rebuild {
    pub fn matches(&self, key: &PluginKey) -> bool {
        self.previous.plugin.format == key.format && self.previous.plugin.id == key.id
    }
}

#[cfg(test)]
thread_local! {
    pub(crate) static FAILURES: std::cell::RefCell<Vec<&'static str>> = const { std::cell::RefCell::new(Vec::new()) };
}

// Fault injection is confined to this transaction, and consumed once per point.
pub(crate) fn checkpoint(_point: &'static str) -> Result<(), String> {
    #[cfg(test)]
    if FAILURES.with(|f| {
        let mut f = f.borrow_mut();
        if f.first() == Some(&_point) {
            f.remove(0);
            true
        } else {
            false
        }
    }) {
        return Err(format!("injected rebuild {_point} failure"));
    }
    Ok(())
}

impl App {
    /// Returns true when this request was handled, including a safe preflight failure.
    pub(crate) fn rebuild_minimal(&mut self, index: usize, editor: bool) -> bool {
        let plugin = self.plugins[index].clone();
        let Some(slot) = self.instances.iter().position(|i| {
            i.minimal_data_home && i.plugin.format == plugin.format && i.plugin.id == plugin.id
        }) else {
            return false;
        };
        let id = self.instances[slot].id;
        self.pause_audio();
        let saved = checkpoint("save")
            .and_then(|()| self.save_plugin_state(id))
            .and_then(|()| {
                let path = self.config_path.as_ref().map_err(Clone::clone)?;
                crate::state_store::load(path, &self.instances[slot].plugin)?
                    .ok_or_else(|| "saved rebuild state is missing".to_string())
            });
        let state = match saved {
            Ok(state) => state,
            Err(error) => {
                self.load_failed(format!("Could not preserve plugin before rebuild: {error}"));
                return true;
            }
        };
        // Persist the latest favorite/playback selection before losing the old id.
        self.save_session();
        if let Some(error) = self.config_error.clone() {
            self.load_failed(format!(
                "Could not preserve session before rebuild: {error}"
            ));
            return true;
        }
        self.editor_rebuild = Some(Rebuild {
            previous: self.instances.remove(slot),
            state,
            error: None,
            failed: false,
        });
        self.host.destroy_instance(id);
        self.status = format!("Reloading {} with normal catalogs...", plugin.name);
        self.create_rebuild(
            plugin,
            if editor {
                LoadPurpose::Editor(id)
            } else {
                LoadPurpose::Manual
            },
        );
        true
    }

    fn create_rebuild(&mut self, plugin: PluginInfo, purpose: LoadPurpose) {
        self.pending = Some(PendingLoad {
            plugin: plugin.clone(),
            purpose,
            data_home: None,
        });
        if let Err(error) = checkpoint("create") {
            // Use the same event path as a failed native creation.
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
            );
        }
    }

    pub(crate) fn recover_editor_rebuild(&mut self, message: &str) -> bool {
        let Some(rebuild) = &mut self.editor_rebuild else {
            return false;
        };
        if rebuild.failed {
            return false;
        }
        if rebuild.error.is_some() {
            rebuild.failed = true;
            self.config_error = Some(format!("{message}; recovery also failed. Latest state and session remain saved; load the plugin again to retry."));
            self.status = self.config_error.clone().unwrap();
            return true;
        }
        rebuild.error = Some(message.to_string());
        let key = rebuild.previous.plugin.clone();
        let Some(index) = key.find(&self.plugins) else {
            rebuild.failed = true;
            self.status = format!("{message}; recovery plugin missing; latest state remains saved");
            self.config_error = Some(self.status.clone());
            return true;
        };
        let plugin = self.plugins[index].clone();
        self.status = format!("{message}; recovering saved sound...");
        self.create_rebuild(plugin, LoadPurpose::Manual);
        true
    }

    pub(crate) fn finish_editor_rebuild(&mut self, id: i32) -> Option<String> {
        let rebuild = self.editor_rebuild.take()?;
        let instance = self.instances.iter_mut().find(|i| i.id == id)?;
        if rebuild.matches(&instance.plugin) {
            instance.preset_name = rebuild.previous.preset_name;
            for (active, _) in &mut self.favorites.active {
                if *active == rebuild.previous.id {
                    *active = id;
                }
            }
        }
        rebuild.error
    }
}
