//! Cache-first startup and one refresh after the UI becomes idle.
use crate::{on_scan_done, plugin_cache, plugin_list::PluginKind, startup, App, Stage};
use eframe::egui;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct Startup {
    refresh_pending: bool,
    pub running: bool,
    idle_since: Option<Instant>,
}

impl Startup {
    pub(crate) fn cancel_refresh(&mut self) {
        self.refresh_pending = false;
    }
    pub(crate) fn observe_activity(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| !input.events.is_empty() || input.pointer.any_down()) {
            self.idle_since = None;
        }
    }
}

#[cfg(test)]
mod tests;

impl App {
    pub(crate) fn load_plugin_cache(&mut self) {
        if cfg!(feature = "recording-mode") {
            return;
        }
        let Some(config) = self.config_path.as_ref().ok() else {
            return;
        };
        let entries = match plugin_cache::load(config) {
            Ok(Some(entries)) => entries,
            Ok(None) => return,
            Err(error) => {
                eprintln!("plugin-cache read failed; scanning required: {error}");
                return;
            }
        };
        for entry in entries {
            if let Some(mut plugin) = self.host.restore_plugin(&entry.plugin) {
                plugin.kind = PluginKind::from_metadata(&entry.kind);
                self.plugins.push(plugin);
            }
        }
        self.deferred_scan = false;
        self.catalog_startup.refresh_pending = true;
        self.patch_catalog.reconcile(&self.plugins);
        self.first_instrument.on_scan(&self.plugins);
        startup::mark(Stage::CatalogReady);
        eprintln!("plugin-cache loaded: plugins={}", self.plugins.len());
    }

    pub(crate) fn poll_catalog_startup(&mut self, ctx: &egui::Context) {
        self.catalog_startup.observe_activity(ctx);
        if self.deferred_scan && self.pending.is_none() {
            self.deferred_scan = false;
            self.start_scan(false);
            return;
        }
        if !self.catalog_startup.refresh_pending {
            return;
        }
        let active = ctx.input(|input| !input.events.is_empty() || input.pointer.any_down());
        if active
            || self.scanning
            || self.restoring
            || self.pending.is_some()
            || self.preset_change_busy()
            || self.patch_browser.requests.desired.is_some()
            || self.effect_browser.desired.is_some()
            || self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.exit_requested)
            || (self.patch_browser.open
                && self.patch_browser.snapshot.is_none()
                && self.instrument_id().is_some())
        {
            self.catalog_startup.idle_since = None;
            return;
        }
        let idle = self
            .catalog_startup
            .idle_since
            .get_or_insert_with(Instant::now);
        ctx.request_repaint_after(Duration::from_millis(100));
        if idle.elapsed() < Duration::from_secs(2) {
            return;
        }
        if unsafe {
            self.host
                .scan_background(on_scan_done, std::ptr::null_mut())
        } {
            self.catalog_startup.refresh_pending = false;
            self.catalog_startup.running = true;
            eprintln!("plugin-cache idle refresh started");
        }
    }

    /// Also refresh native indices when unchanged, without rebuilding the UI.
    pub(crate) fn finish_background_scan(&mut self, error: Option<&str>) -> bool {
        if !std::mem::take(&mut self.catalog_startup.running) {
            return false;
        }
        if let Some(error) = error {
            eprintln!("plugin-cache idle refresh failed; keeping cache: {error}");
            return true;
        }
        self.host.accept_scan();
        let plugins = self.host.plugins();
        let changed = plugin_cache::entries(&plugins) != plugin_cache::entries(&self.plugins);
        self.plugins = plugins;
        if changed {
            self.patch_catalog.reconcile(&self.plugins);
        }
        self.first_instrument.on_scan(&self.plugins);
        self.save_plugin_cache();
        eprintln!(
            "plugin-cache idle refresh finished: changed={changed} plugins={}",
            self.plugins.len()
        );
        true
    }

    pub(crate) fn save_plugin_cache(&self) {
        if cfg!(feature = "recording-mode") {
            return;
        }
        if let Ok(config) = &self.config_path {
            match plugin_cache::save(config, &self.plugins) {
                Ok(changed) => eprintln!("plugin-cache saved: changed={changed}"),
                Err(error) => eprintln!("plugin-cache write failed: {error}"),
            }
        }
    }
}
