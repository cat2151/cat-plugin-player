//! Audio-first startup, on the same main thread that will later run eframe.

use crate::{audio, config, ffi, on_wake, startup, App, Stage};
use std::time::{Duration, Instant};

impl App {
    pub(crate) fn prepare() -> Self {
        let (restore, config_error) =
            match config::path().and_then(|path| config::Config::load(&path)) {
                Ok(config) => (config.last_played, None),
                Err(error) => (None, Some(format!("Could not read session: {error}"))),
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
            restore,
            config_error,
            scanning: false,
            deferred_scan: true,
            fast_restore: false,
            filter: String::new(),
            status,
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
