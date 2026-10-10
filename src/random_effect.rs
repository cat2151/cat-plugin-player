//! Prepare a shared-core effect preset in an owned temporary CLAP instance.
use crate::{random_effect_catalog::Candidate, random_patch::choose, App};
use eframe::egui;
use std::sync::mpsc::{self, Receiver, TryRecvError};

#[cfg(test)]
#[path = "random_effect_tests.rs"]
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
    let action = || {
        let entry = cmrt_core::load_entry(bundle)?;
        let mut renderer = cmrt_core::effect::EffectRenderer::new(
            &entry,
            &candidate.plugin_id,
            sample_rate,
            crate::audio::MAX_BLOCK_FRAMES as usize,
        )?;
        renderer.load_preset(&candidate.preset)?;
        renderer.save_state()
    };
    action().map_err(|error| format!("{error:#}"))
}

impl App {
    pub(crate) fn random_effect_candidates(
        &self,
        target: Option<i32>,
    ) -> impl Iterator<Item = &Candidate> {
        self.random_effect_catalog
            .installed(&self.plugins)
            .filter(move |candidate| {
                // Preserve the one-instance-per-identity routing rule. The selected
                // slot may use another preset of its own plugin.
                !self.instances.iter().any(|instance| {
                    instance.kind == crate::plugin_list::PluginKind::Effect
                        && instance.plugin.format == "CLAP"
                        && instance.plugin.id == candidate.plugin_id
                        && Some(instance.id) != target
                })
            })
    }

    pub(crate) fn start_random_effect(&mut self, ctx: &egui::Context) {
        if self.actions_busy() || self.instrument_id().is_none() {
            return;
        }
        let mut rng = rand::thread_rng();
        let target = choose(self.effect_ids().into_iter(), &mut rng);
        let Some(candidate) = choose(self.random_effect_candidates(target), &mut rng).cloned()
        else {
            self.status = "Random effect unavailable: no eligible preset for this slot".into();
            return;
        };
        let plugin = self
            .plugins
            .iter()
            .find(|plugin| crate::random_effect_catalog::matches_plugin(&candidate, plugin))
            .unwrap();
        let bundle = plugin.bundle_path.clone();
        self.status = format!("Preparing effect: {}...", candidate.preset.display);
        self.random_effect.start(
            candidate,
            target,
            bundle,
            f64::from(self.sample_rate()),
            ctx,
        );
    }

    pub(crate) fn poll_random_effect(&mut self) {
        if let Some(result) = self.random_effect.poll() {
            match result {
                Ok(prepared) => self.apply_random_effect(prepared),
                Err(error) => self.status = format!("Random effect failed: {error}"),
            }
        }
    }
}
