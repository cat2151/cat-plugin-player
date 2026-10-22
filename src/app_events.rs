use crate::*;

impl App {
    pub(crate) fn start_scan(&mut self, rescan: bool) {
        if self.preset_change_busy() || self.catalog_startup.running {
            return;
        }
        startup::mark(Stage::ScanRequested);
        self.patch_browser.requests.invalidate();
        if unsafe {
            self.host
                .scan_async(rescan, on_scan_done, std::ptr::null_mut())
        } {
            self.catalog_startup.cancel_refresh();
            self.scanning = true;
            self.status = if rescan {
                "Rescanning plugins...".into()
            } else {
                "Loading plugin list...".into()
            };
        }
    }

    pub(crate) fn handle_events(&mut self) {
        let events = std::mem::take(&mut *EVENTS.lock().unwrap());
        for event in events {
            match event {
                Event::ScanDone(error) => {
                    if self.finish_background_scan(error.as_deref()) {
                        continue;
                    }
                    self.scanning = false;
                    self.plugins = self.host.plugins();
                    self.patch_catalog.reconcile(&self.plugins);
                    startup::mark(Stage::CatalogReady);
                    let scan_succeeded = error.is_none();
                    self.status = match error {
                        Some(e) => format!("Scan failed: {e}"),
                        None => String::new(),
                    };
                    if scan_succeeded {
                        self.save_plugin_cache();
                        if let Some(index) = self.first_instrument.on_scan(&self.plugins) {
                            // Restoring too would leave `restoring` set behind this pending load.
                            self.restore = None;
                            self.restore_effect.clear();
                            self.load_auto_instrument(index);
                        } else {
                            self.restore_session();
                        }
                    } else if self.restoring && self.instrument_id().is_some() {
                        self.load_failed(format!(
                            "{}; could not restore effect; playing instrument directly",
                            self.status
                        ));
                    }
                }
                Event::InstanceCreated { id, error } => {
                    self.instance_created(id, error);
                }
            }
        }
    }

    /// Waits for the first audio callback; a resident player has no audio to wait for.
    pub(crate) fn poll_patch_catalog(&mut self, ctx: &egui::Context) {
        let ready = self.resident.is_some()
            || self.instances.iter().any(|instance| {
                instance
                    .voice
                    .as_ref()
                    .is_some_and(audio::Voice::has_rendered)
            });
        if let Some(message) = self.patch_catalog.update(ready, ctx) {
            self.patch_catalog.reconcile(&self.plugins);
            if !message.is_empty() {
                eprintln!("{message}");
                self.status = message;
            }
        }
    }
}
