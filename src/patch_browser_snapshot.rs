//! Build roles/presets once on a worker; adopt only a matching scan generation.
use crate::{patch_browser_catalog::Snapshot, patch_filter_store::Store, App};
use eframe::egui;
use std::sync::mpsc::{self, TryRecvError};

impl App {
    pub(crate) fn update_browser_snapshot(&mut self, ctx: &egui::Context) {
        let browser = &mut self.patch_browser;
        if browser.generation != self.random_patch_catalog.generation {
            browser.generation = self.random_patch_catalog.generation;
            browser.solo_draft = None;
            browser.condition_menu = false;
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
                        }
                        Err(error) => browser.build_error = Some(error),
                    }
                }
            }
        }
        if !browser.open
            || browser.snapshot.is_some()
            || browser.build.is_some()
            || browser.build_error.is_some()
            || self.random_patch_catalog.notice().is_some()
        {
            return;
        }
        let candidates = self.random_patch_catalog.candidates().cloned().collect();
        let config = self.config_path.clone();
        let presets = browser.store.as_ref().map(|store| store.presets.clone());
        let (tx, rx) = mpsc::channel();
        browser.build = Some(rx);
        browser.build_generation = browser.generation;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let store = presets.is_none().then(|| Store::load(&config));
            let users = presets
                .as_deref()
                .or_else(|| store.as_ref().map(|store| store.presets.as_slice()))
                .unwrap_or(&[]);
            let result = Snapshot::build(candidates, users).map(|snapshot| (snapshot, store));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
}
