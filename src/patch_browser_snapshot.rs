//! Build roles/presets once on a worker; adopt only a matching scan generation.
use crate::{patch_browser_catalog::Snapshot, patch_filter_store::Store, App};
use eframe::egui;
use std::sync::mpsc::{self, TryRecvError};

impl App {
    pub(crate) fn update_browser_snapshot(&mut self, ctx: &egui::Context) {
        let browser = &mut self.patch_browser;
        if browser.generation != self.patch_catalog.generation {
            browser.generation = self.patch_catalog.generation;
            browser.solo_draft = None;
            browser.condition_menu = false;
            browser.restore_selected = browser.saved_cursor();
            browser.snapshot = None;
            browser.results.clear();
            browser.selected = None;
            browser.requests.invalidate();
            browser.build_error = None;
            browser.reveal_pending = true;
        }
        if let Some(rx) = &browser.build {
            let result = match rx.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("Browser catalog worker disconnected".into()))
                }
            };
            if let Some(result) = result {
                browser.build = None;
                if browser.build_generation == browser.generation {
                    match result {
                        Ok((snapshot, store)) => {
                            if let Some(store) = store {
                                browser.store = Some(store);
                            }
                            browser.snapshot = Some(snapshot);
                            browser.resolve_restored_preset();
                            browser.resolve_restored_playing();
                            browser.filter(false); // Merely opening must never change sound.
                            eprintln!(
                                "patch-browser published: generation={} candidates={} results={} query={:?} role={} preset={}",
                                browser.generation,
                                browser.snapshot.as_ref().unwrap().candidates.len(),
                                browser.results.len(),
                                browser.query,
                                browser.role,
                                browser.preset
                            );
                        }
                        Err(error) => browser.build_error = Some(error),
                    }
                }
            }
        }
        if !browser.open
            // Editor selection must not resolve saved filters against the
            // single instrument known by the audio-first startup restore.
            || (self.selection.is_some() && (self.scanning || self.deferred_scan))
            || browser.snapshot.is_some()
            || browser.build.is_some()
            || browser.build_error.is_some()
            || self.patch_catalog.notice().is_some()
        {
            return;
        }
        let candidates: Vec<_> = self.patch_catalog.candidates().cloned().collect();
        eprintln!(
            "patch-browser build: generation={} plugins={} candidates={}",
            browser.generation,
            self.plugins.len(),
            candidates.len()
        );
        let config = self.config_path.clone();
        let presets = browser.store.as_ref().map(|store| store.presets.clone());
        let (tx, rx) = mpsc::channel();
        browser.build = Some(rx);
        browser.build_generation = browser.generation;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let store = presets.is_none().then(|| Store::load(&config));
            let users = presets
                .as_deref()
                .or_else(|| store.as_ref().map(|store| store.presets.as_slice()))
                .unwrap_or(&[]);
            let result = Snapshot::build(candidates, users).map(|snapshot| (snapshot, store));
            eprintln!(
                "patch-browser build finished: elapsed_ms={} success={}",
                started.elapsed().as_millis(),
                result.is_ok()
            );
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
}

#[cfg(test)]
mod tests;
