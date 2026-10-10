//! Shared effect catalog discovery stays off the UI/audio threads.
use crate::{ffi::PluginInfo, plugin_list::PluginKind};
use cmrt_core::{audio_effect::PresetLocation, AudioEffectCatalog, AudioEffectPreset};
use eframe::egui;
use std::sync::mpsc::{self, Receiver};

#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub plugin_id: String,
    pub plugin_name: String,
    pub preset: PresetLocation,
    /// `display` without the plugin prefix.
    pub name: String,
    pub category: String,
    pub kind: String,
}

impl Candidate {
    pub(crate) fn new(catalog: &AudioEffectCatalog, preset: &AudioEffectPreset) -> Option<Self> {
        let plugin = catalog.plugin(&preset.plugin).ok()?;
        Some(Self {
            plugin_id: plugin.plugin_id.clone(),
            plugin_name: plugin.name.clone(),
            preset: PresetLocation {
                path: preset.path.clone(),
                value: preset.value.clone(),
                display: preset.display.clone(),
            },
            name: preset.name.clone(),
            category: preset.category.clone(),
            kind: preset.kind.clone(),
        })
    }
}

#[derive(Default)]
pub(crate) struct Catalog {
    started: bool,
    receiver: Option<Receiver<Vec<Candidate>>>,
    candidates: Vec<Candidate>,
    /// Bumped when discovery delivers candidates.
    pub generation: u64,
}

impl Catalog {
    #[cfg(test)]
    pub(crate) fn from_candidates(candidates: Vec<Candidate>) -> Self {
        Self {
            started: true,
            receiver: None,
            candidates,
            generation: 1,
        }
    }
    pub fn update(&mut self, ready: bool, ctx: &egui::Context) {
        if ready && !self.started {
            self.started = true;
            let (tx, rx) = mpsc::channel();
            self.receiver = Some(rx);
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(candidates(&AudioEffectCatalog::discover()));
                ctx.request_repaint();
            });
        }
        if let Some(rx) = &self.receiver {
            match rx.try_recv() {
                Ok(candidates) => {
                    self.candidates = candidates;
                    self.receiver = None;
                    self.generation += 1;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.receiver = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }

    pub fn loading(&self) -> bool {
        !self.started || self.receiver.is_some()
    }

    pub fn installed<'a>(
        &'a self,
        plugins: &'a [PluginInfo],
    ) -> impl Iterator<Item = &'a Candidate> {
        self.candidates.iter().filter(|candidate| {
            plugins
                .iter()
                .any(|plugin| matches_plugin(candidate, plugin))
        })
    }
}

pub(crate) fn matches_plugin(candidate: &Candidate, plugin: &PluginInfo) -> bool {
    plugin.kind == PluginKind::Effect && plugin.format == "CLAP" && plugin.id == candidate.plugin_id
}

fn candidates(catalog: &AudioEffectCatalog) -> Vec<Candidate> {
    catalog
        .presets()
        .iter()
        .filter(|preset| cmrt_patch_select::auto_reverb::is_selectable_effect_preset(preset))
        .filter_map(|preset| Candidate::new(catalog, preset))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_exclusions_and_installed_effect_identity_are_shared() {
        let plugin = cmrt_core::AudioEffectPluginInfo::new(
            "Surge XT Effects",
            "X:/plugins/surge-fx.clap",
            "org.surge-synth-team.surge-xt-fx",
            "X:/presets",
        );
        let preset = |value: &str| cmrt_core::AudioEffectPreset {
            plugin: plugin.key.clone(),
            json_key: plugin.json_key.clone(),
            value: value.into(),
            display: value.into(),
            name: value.into(),
            category: "Space".into(),
            kind: "Reverb".into(),
            path: "X:/presets/test.srgfx".into(),
        };
        let catalog = AudioEffectCatalog::with_entries(
            vec![plugin.clone()],
            vec![preset("Reverb 1/excluded"), preset("Reverb 2/kept")],
        );
        let result = candidates(&catalog);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].preset.value, "Reverb 2/kept");
        let mut installed = PluginInfo {
            index: 7,
            name: plugin.name,
            vendor: "Vendor".into(),
            format: "CLAP".into(),
            id: plugin.plugin_id,
            bundle_path: "X:/relocated/surge.clap".into(),
            kind: PluginKind::Effect,
        };
        assert!(matches_plugin(&result[0], &installed));
        installed.kind = PluginKind::Instrument;
        assert!(!matches_plugin(&result[0], &installed));
        installed.kind = PluginKind::Effect;
        installed.format = "VST3".into();
        assert!(!matches_plugin(&result[0], &installed));
    }
}
