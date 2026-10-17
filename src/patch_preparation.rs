//! Owned preparation work; only temporary cmrt instances live on the worker.
use crate::{patch_catalog::Candidate, App};
use eframe::egui;
use rand::Rng;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub(crate) struct Prepared {
    pub candidate: Candidate,
    pub state: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct Preparation {
    receiver: Option<Receiver<Result<Prepared, String>>>,
}

pub(crate) fn choose<T>(items: impl Iterator<Item = T>, rng: &mut impl Rng) -> Option<T> {
    use rand::seq::IteratorRandom;
    items.choose(rng)
}

impl Preparation {
    pub fn busy(&self) -> bool {
        self.receiver.is_some()
    }

    pub(crate) fn start(&mut self, candidate: Candidate, ctx: &egui::Context) {
        self.start_with(candidate, ctx.clone(), |candidate| {
            crate::state_request::catalog(
                &candidate.plugin_id,
                &candidate.bundle_path,
                &candidate.path,
            )
        });
    }

    pub(crate) fn start_with(
        &mut self,
        candidate: Candidate,
        ctx: egui::Context,
        prepare: impl FnOnce(&Candidate) -> Result<Vec<u8>, String> + Send + 'static,
    ) {
        if self.busy() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        std::thread::spawn(move || {
            let result = prepare(&candidate).map(|state| Prepared { candidate, state });
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    pub(crate) fn poll(&mut self) -> Option<Result<Prepared, String>> {
        let result = match self.receiver.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("preparation worker disconnected".into()),
        };
        self.receiver = None;
        Some(result)
    }
}

impl App {
    pub(crate) fn preset_change_busy(&self) -> bool {
        self.patch_browser.preparation.busy()
            || self.effect_preparation.busy()
            || self.pending.as_ref().is_some_and(|pending| {
                matches!(
                    pending.purpose,
                    crate::session_load::LoadPurpose::Patch(_)
                        | crate::session_load::LoadPurpose::EffectPreset(_)
                )
            })
    }

    pub(crate) fn actions_busy(&self) -> bool {
        crate::patch_catalog::busy(
            self.scanning || self.deferred_scan,
            self.restoring,
            self.pending.is_some(),
            self.effect_preparation.busy() || self.patch_browser.preparation.busy(),
        )
    }
}

#[cfg(test)]
mod tests;
