//! Owned conversion work only; native instances stay on the GUI thread.
use crate::{random_patch_catalog::Candidate, App};
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

    fn start(&mut self, candidate: Candidate, ctx: &egui::Context) {
        self.start_with(candidate, ctx.clone(), |candidate| {
            cmrt_patches::prepare_clap_patch_state(&candidate.plugin_id, &candidate.path)
                .map_err(|error| error.to_string())
        });
    }

    fn start_with(
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

    fn poll(&mut self) -> Option<Result<Prepared, String>> {
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
    pub(crate) fn random_busy(&self) -> bool {
        self.random_patch.busy()
            || self.pending.as_ref().is_some_and(|pending| {
                matches!(pending.purpose, crate::session_load::LoadPurpose::Random(_))
            })
    }

    pub(crate) fn actions_busy(&self) -> bool {
        crate::random_patch_catalog::busy(
            self.scanning || self.deferred_scan,
            self.restoring,
            self.pending.is_some(),
            self.random_patch.busy(),
        )
    }

    pub(crate) fn start_random_patch(&mut self, ctx: &egui::Context) {
        if self.actions_busy() {
            return;
        }
        let Some(candidate) = choose(
            self.random_patch_catalog.candidates(),
            &mut rand::thread_rng(),
        )
        .cloned() else {
            return;
        };
        self.status = format!(
            "Preparing {}: {}...",
            candidate.plugin_name, candidate.display
        );
        self.random_patch.start(candidate, ctx);
    }

    pub(crate) fn poll_random_patch(&mut self) {
        if let Some(result) = self.random_patch.poll() {
            match result {
                Ok(prepared) => self.apply_random_patch(prepared),
                Err(error) => self.status = format!("Random patch failed: {error}"),
            }
        }
    }
}

#[cfg(test)]
mod tests;
