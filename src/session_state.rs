use crate::{config, state_store, App};

impl App {
    pub(crate) fn restore_plugin_state(
        &mut self,
        id: i32,
        key: &config::PluginKey,
    ) -> Result<(), String> {
        if let Some(selection) = &self.selection {
            if self.restoring {
                if let Some(slot) = selection
                    .settings
                    .instrument
                    .iter()
                    .chain(&selection.settings.effects)
                    .find(|s| s.plugin.format == key.format && s.plugin.id == key.id)
                {
                    return self.host.load_state(id, &slot.state);
                }
            }
            return Ok(());
        }
        let path = self.config_path.as_ref().map_err(Clone::clone)?;
        if let Some(state) = state_store::load(path, key)? {
            self.host.load_state(id, &state)?;
            self.surge_start_delay.restore(id, key, &state);
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
        state_store::save_named(
            path,
            &instance.plugin,
            &state,
            instance.preset_name.as_deref(),
        )?;
        self.surge_start_delay.saved_state(id, &state);
        self.save_session();
        Ok(())
    }
}
