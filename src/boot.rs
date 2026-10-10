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
            mml,
            sequence_velocity,
            sequence_modulation,
            favorite_selection,
            show_favorites,
            show_history,
            on_right,
            window_config,
            prefer_clap,
            config_error,
        ) = match config_path.as_ref().map_err(Clone::clone).and_then(|path| {
            let settings = config::Config::load(path)?;
            let status = crate::status::Status::load(path)?;
            Ok((settings, status))
        }) {
            Ok((settings, status)) => (
                status.last_played,
                status.effects.into(),
                status.effect_bypassed,
                status.sequence_pattern,
                status.sequence_pattern.selection(status.selected_sequence),
                status.mml,
                status.sequence_velocity,
                status.sequence_modulation,
                status.favorites,
                status.show_favorites,
                status.show_history,
                status.on_right,
                settings.window,
                settings.plugins.prefer_clap,
                None,
            ),
            Err(error) => (
                None,
                Default::default(),
                false,
                crate::SequencePattern::default(),
                crate::SequencePattern::default(),
                String::new(),
                crate::SequenceVelocity::default(),
                crate::SequenceModulation::default(),
                crate::status::FavoriteSelection::default(),
                None,
                false,
                crate::status::Status::default().on_right,
                Default::default(),
                true,
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
        let mut mml_input = crate::mml_input::MmlInput::default();
        mml_input.confirmed = mml;
        let rate = output
            .as_ref()
            .map_or(crate::FALLBACK_SAMPLE_RATE, |o| o.sample_rate());
        let restored_mml =
            mml_input.restore_phrase(rate, sequence_pattern.selection(selected_sequence));
        let config_error = restored_mml
            .err()
            .map(|e| format!("Could not restore MML: {e}"))
            .or(config_error);
        Self {
            instances: Vec::new(),
            window_config,
            host,
            output,
            plugins: Vec::new(),
            prefer_clap,
            pending: None,
            restore_effect,
            effect_bypassed,
            sequence_pattern,
            selected_sequence,
            sequence_velocity,
            sequence_modulation,
            restore,
            mml_input,
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
                show: show_favorites.unwrap_or_default(),
                restore_show: show_favorites,
                show_history,
                ..Default::default()
            },
            scope_ui: crate::scope_ui::ScopeUi::with_on_right(on_right),
            plugin_icons: Default::default(),
            repaint_heartbeat: None,
            random_patch_catalog: Default::default(),
            random_patch: Default::default(),
            random_effect: Default::default(),
            random_effect_catalog: Default::default(),
            unsafe_state: Default::default(),
            shutdown: Default::default(),
            #[cfg(test)]
            random_failures: Default::default(),
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
