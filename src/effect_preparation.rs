//! Prepare an effect preset state: prepared ahead (ADR 0028), or in an owned temporary CLAP instance.
use crate::{effect_catalog::Candidate, App};
use eframe::egui;
use std::sync::mpsc::{self, Receiver, TryRecvError};

#[cfg(test)]
#[path = "effect_preparation_tests.rs"]
mod tests;

pub(crate) struct Prepared {
    pub candidate: Candidate,
    pub target: Option<i32>,
    pub state: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct Preparation {
    receiver: Option<Receiver<Result<Prepared, String>>>,
}

impl Preparation {
    pub fn busy(&self) -> bool {
        self.receiver.is_some()
    }

    fn start(
        &mut self,
        candidate: Candidate,
        target: Option<i32>,
        bundle_path: String,
        sample_rate: f64,
        ctx: &egui::Context,
    ) {
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        if let Some(state) = candidate.state.clone() {
            let _ = tx.send(Ok(Prepared {
                candidate,
                target,
                state: state.to_vec(),
            }));
            ctx.request_repaint();
            return;
        }
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = prepare(&candidate, &bundle_path, sample_rate).map(|state| Prepared {
                candidate,
                target,
                state,
            });
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

fn prepare(candidate: &Candidate, bundle: &str, sample_rate: f64) -> Result<Vec<u8>, String> {
    crate::surge_fx_states::prepare_state(
        bundle,
        &candidate.plugin_id,
        &candidate.preset,
        sample_rate,
        crate::audio::MAX_BLOCK_FRAMES as usize,
    )
}

impl App {
    /// The one-instance-per-identity routing rule. The target slot may use
    /// another preset of its own plugin.
    pub(crate) fn effect_plugin_taken(&self, plugin_id: &str, target: Option<i32>) -> bool {
        self.instances.iter().any(|instance| {
            instance.kind == crate::plugin_list::PluginKind::Effect
                && instance.plugin.format == "CLAP"
                && instance.plugin.id == plugin_id
                && Some(instance.id) != target
        })
    }

    /// The caller has checked that the actions are idle and an instrument is loaded.
    pub(crate) fn start_effect(
        &mut self,
        candidate: Candidate,
        target: Option<i32>,
        ctx: &egui::Context,
    ) {
        let Some(plugin) = self
            .plugins
            .iter()
            .find(|plugin| crate::effect_catalog::matches_plugin(&candidate, plugin))
        else {
            self.status = "Browse effect failed: plugin no longer available".into();
            return;
        };
        let bundle = plugin.bundle_path.clone();
        // The instrument patch keeps playing; only pending browser requests are stale.
        self.patch_browser.requests.invalidate();
        self.status = format!("Preparing effect: {}...", candidate.preset.display);
        self.effect_preparation.start(
            candidate,
            target,
            bundle,
            f64::from(self.sample_rate()),
            ctx,
        );
    }

    pub(crate) fn poll_effect_preparation(&mut self) {
        if let Some(result) = self.effect_preparation.poll() {
            match result {
                // A newer browser selection replaces this one before it is heard.
                Ok(_) if self.effect_browser.desired.is_some() => {}
                Ok(prepared) => self.apply_effect(prepared),
                Err(error) => self.status = format!("Browse effect failed: {error}"),
            }
        }
    }
}
