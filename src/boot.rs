//! Audio-first startup, on the same main thread that will later run eframe.

use crate::{audio, config, ffi, on_wake, startup, App, Stage};
use std::time::{Duration, Instant};

impl App {
    #[cfg(test)]
    pub(crate) fn prepare() -> Self {
        Self::prepare_mode(false)
    }

    pub(crate) fn prepare_mode(selecting: bool) -> Self {
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
            browser_filter,
            effect_filter,
            window_config,
            prefer_clap,
            config_error,
        ) = match config_path.as_ref().map_err(Clone::clone).and_then(|path| {
            let settings = config::Config::load(path)?;
            let status = if selecting {
                crate::status::Status::default()
            } else {
                crate::status::Status::load(path)?
            };
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
                status.patch_browser,
                status.effect_browser,
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
                Default::default(),
                Default::default(),
                true,
                Some(format!("Could not read session: {error}")),
            ),
        };
        startup::mark(Stage::HistoryRead);
        let host = unsafe { ffi::Host::new(on_wake, std::ptr::null_mut()) }
            .expect("uh_create() failed: could not set up the uapmd plugin host");
        startup::mark(Stage::HostReady);
        #[cfg(test)]
        ffi::trace::mark("device query begin");
        let (output, status) = match audio::Output::open_default() {
            Ok(output) => (Some(output), String::new()),
            Err(error) => (None, format!("No audio: {error}")),
        };
        #[cfg(test)]
        ffi::trace::mark("device query end");
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
        let mut app = Self {
            editor_rebuild: None,
            instances: Vec::new(),
            selection: None,
            resident: None,
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
            catalog_startup: Default::default(),
            confirm_rescan: false,
            first_instrument: Default::default(),
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
            patch_catalog: Default::default(),
            patch_browser: crate::patch_browser::Browser::restored(browser_filter),
            effect_browser: crate::effect_browser::EffectBrowser::restored(effect_filter),
            auto_start_pending: true,
            browser_auto_instrument: false,
            browser_auto_effect: None,
            effect_preparation: Default::default(),
            effect_swap_fade: false,
            effect_catalog: Default::default(),
            unsafe_state: Default::default(),
            shutdown: Default::default(),
            #[cfg(test)]
            patch_failures: Default::default(),
        };
        app.shutdown.selection_requested = selecting;
        app
    }

    pub(crate) fn restore_before_gui(&mut self) {
        let data_home = self.startup_data_home();
        let requested = self.request_startup_restore();
        // The pending load restores the environment when the creation finishes or fails.
        if let Some(pending) = &mut self.pending {
            pending.data_home = data_home;
        }
        if requested {
            self.wait_for_startup_audio();
        }
    }

    /// Returns whether a restoration was requested before the GUI opens.
    fn request_startup_restore(&mut self) -> bool {
        if !self.deferred_scan {
            // A newly imported SMF may name a valid plugin absent from the
            // last complete scan. Add that target without losing the cache.
            if let Some(key) = self.restore.as_ref() {
                if key.find(&self.plugins).is_none() {
                    if let Some(mut plugin) = self.host.restore_plugin(key) {
                        plugin.kind = crate::plugin_list::PluginKind::Instrument;
                        self.plugins.push(plugin);
                        self.patch_catalog.reconcile(&self.plugins);
                    } else {
                        // A relocated saved target still needs scan-based
                        // restoration; keep its key until the scan completes.
                        self.deferred_scan = true;
                        self.catalog_startup.cancel_refresh();
                        return false;
                    }
                }
            }
            self.restore_session();
            return true;
        }
        let Some(plugin) = self
            .restore
            .as_ref()
            .and_then(|key| self.host.restore_plugin(key))
        else {
            // Old history, missing bundle, or no history: scan after the GUI opens.
            return false;
        };
        self.fast_restore = true;
        self.restoring = true;
        let mut plugin = plugin;
        plugin.kind = crate::plugin_list::PluginKind::Instrument;
        self.plugins.push(plugin);
        self.load_plugin(0);
        true
    }

    /// Only the startup creation of the last instrument skips Surge XT's browser
    /// scans; an editor opened later needs the full menus, so other loads keep the
    /// normal data home. SMF selection and resident players are not startups.
    fn startup_data_home(&self) -> Option<crate::plugin_specific::StartupDataHome> {
        let key = self.restore.as_ref()?;
        if self.selection.is_some() || self.resident.is_some() {
            return None;
        }
        let cache = crate::native_library::cache_root().map(|root| root.join("cache"));
        match cache.and_then(|cache| crate::plugin_specific::startup_data_home(key, &cache)) {
            Ok(guard) => guard,
            Err(error) => {
                self.log_error(&format!(
                    "Could not prepare Surge XT minimal data home; using the normal one: {error}"
                ));
                None
            }
        }
    }

    fn wait_for_startup_audio(&mut self) {
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
