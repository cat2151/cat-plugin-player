use crate::{audio, config, on_instance, startup, App};

impl App {
    pub(crate) fn load_plugin(&mut self, index: usize) {
        let plugin = self.plugins[index].clone();
        self.status = format!("Loading {}...", plugin.name);
        startup::mark(startup::Stage::LoadRequested);
        self.host.create_instance(
            plugin.index,
            self.sample_rate(),
            audio::MAX_BLOCK_FRAMES,
            on_instance,
            std::ptr::null_mut(),
        );
        startup::mark(startup::Stage::LoadReturned);
        self.pending = Some(plugin);
    }

    pub(crate) fn restore_session(&mut self) {
        let Some(last_played) = self.restore.take() else {
            return;
        };
        match last_played.find(&self.plugins) {
            Some(index) => self.load_plugin(index),
            None => {
                self.status = format!(
                    "Previous plugin not found: {} [{}]",
                    last_played.id, last_played.format
                );
            }
        }
    }

    pub(crate) fn remember_played(&mut self, id: i32) {
        let Some(instance) = self.instances.iter().find(|instance| instance.id == id) else {
            return;
        };
        let config = config::Config {
            last_played: Some(instance.plugin.clone()),
        };
        self.config_error = config::path()
            .and_then(|path| config.save(&path))
            .err()
            .map(|error| format!("Could not save session: {error}"));
    }
}
