//! Audio-first startup, on the same main thread that will later run eframe.

use crate::{audio, config, ffi, on_wake, startup, App, Stage};
use std::time::{Duration, Instant};

impl App {
    pub(crate) fn prepare() -> Self {
        let config_path = config::path();
        let (
            restore,
            restore_effect,
            effect_bypassed,
            sequence_pattern,
            selected_sequence,
            sequence_velocity,
            sequence_modulation,
            favorite_selection,
            config_error,
        ) = match config_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| config::Config::load(path))
        {
            Ok(config) => (
                config.last_played,
                config.effect,
                config.effect_bypassed,
                config.sequence_pattern,
                config.sequence_pattern.selection(config.selected_sequence),
                config.sequence_velocity,
                config.sequence_modulation,
                config.favorites,
                None,
            ),
            Err(error) => (
                None,
                None,
                false,
                crate::SequencePattern::default(),
                crate::SequencePattern::default(),
                crate::SequenceVelocity::default(),
                crate::SequenceModulation::default(),
                config::FavoriteSelection::default(),
                Some(format!("Could not read session: {error}")),
            ),
        };
        startup::mark(Stage::HistoryRead);
        let host = ffi::Host::new(on_wake, std::ptr::null_mut())
            .expect("uh_create() failed: could not set up the uapmd plugin host");
        startup::mark(Stage::HostReady);
        let (output, status) = match audio::Output::open_default() {
            Ok(output) => (Some(output), String::new()),
            Err(error) => (None, format!("No audio: {error}")),
        };
        startup::mark(Stage::DeviceReady);
        Self {
            instances: Vec::new(),
            host,
            output,
            plugins: Vec::new(),
            pending: None,
            restore_effect,
            effect_bypassed,
            sequence_pattern,
            selected_sequence,
            sequence_velocity,
            sequence_modulation,
            restore,
            restored: None,
            restoring: false,
            config_error,
            config_path,
            scanning: false,
            confirm_rescan: false,
            deferred_scan: true,
            fast_restore: false,
            filter: String::new(),
            status,
            favorites: crate::favorites::Favorites {
                restore: favorite_selection,
                ..Default::default()
            },
            scope_ui: Default::default(),
            plugin_icons: Default::default(),
        }
    }

    pub(crate) fn restore_before_gui(&mut self) {
        let Some(plugin) = self
            .restore
            .as_ref()
            .and_then(|key| self.host.restore_plugin(key))
        else {
            // Old history, missing bundle, or no history: scan after the GUI opens.
            return;
        };
        self.fast_restore = true;
        self.restoring = true;
        let mut plugin = plugin;
        plugin.kind = crate::plugin_list::PluginKind::Instrument;
        self.plugins.push(plugin);
        self.load_plugin(0);

        // Async plugin creation may need both shim tasks and native messages.
        // Yield to the GUI after a bounded wait; it continues pumping a pending
        // creation. A synchronous plugin call itself cannot be interrupted here.
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            self.host.pump_startup();
            self.handle_events();
            let waiting_for_audio = self.instances.iter().any(|instance| {
                instance
                    .voice
                    .as_ref()
                    .is_some_and(|voice| !voice.has_rendered())
            });
            if (self.pending.is_none() && !waiting_for_audio) || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
