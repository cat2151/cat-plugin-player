//! Shared effect catalog discovery stays off the UI/audio threads.
use crate::{ffi::PluginInfo, plugin_list::PluginKind};
use cmrt_core::{audio_effect::PresetLocation, AudioEffectCatalog, AudioEffectPreset};
use eframe::egui;
use std::path::Path;
use std::sync::{
    mpsc::{self, Receiver},
    Arc,
};

#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub plugin_id: String,
    pub plugin_name: String,
    pub preset: PresetLocation,
    /// `display` without the plugin prefix.
    pub name: String,
    pub category: String,
    pub kind: String,
    /// Prepared ahead (ADR 0028), so no temporary instance is created for it.
    pub state: Option<Arc<[u8]>>,
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
            state: None,
        })
    }
}

#[derive(Default)]
pub(crate) struct Catalog {
    started: bool,
    receiver: Option<Receiver<(Vec<Candidate>, Option<String>)>>,
    candidates: Vec<Candidate>,
    /// Bumped when discovery delivers candidates.
    pub generation: u64,
}

impl Catalog {
    /// Installation changes both plugin files and preset roots. Drop the old
    /// receiver too, so a pre-install result cannot replace the new discovery.
    pub(crate) fn refresh_after_install(&mut self) {
        self.started = false;
        self.receiver = None;
        self.candidates.clear();
        self.generation += 1;
    }

    #[cfg(test)]
    pub(crate) fn from_candidates(candidates: Vec<Candidate>) -> Self {
        Self {
            started: true,
            receiver: None,
            candidates,
            generation: 1,
        }
    }
    /// Returns a notice once, when discovery delivers candidates without some presets.
    pub fn update(&mut self, ready: bool, ctx: &egui::Context) -> Option<String> {
        if ready && !self.started {
            self.started = true;
            let (tx, rx) = mpsc::channel();
            self.receiver = Some(rx);
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let catalog = if cfg!(feature = "recording-mode") {
                    AudioEffectCatalog::scan(
                        cmrt_core::audio_effect::builtin_effect_plugins()
                            .into_iter()
                            .filter(|plugin| plugin.plugin_id == "org.surge-synth-team.surge-xt-fx")
                            .collect(),
                    )
                } else {
                    AudioEffectCatalog::discover()
                };
                let found = candidates(&catalog);
                let table = found
                    .iter()
                    .any(is_surge_fx)
                    .then(crate::surge_fx_states::load);
                let _ = tx.send(attach_surge_fx_states(found, table, |path| {
                    std::fs::read(path)
                }));
                ctx.request_repaint();
            });
        }
        let rx = self.receiver.as_ref()?;
        match rx.try_recv() {
            Ok((candidates, notice)) => {
                self.candidates = candidates;
                self.receiver = None;
                self.generation += 1;
                notice
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.receiver = None;
                None
            }
            Err(mpsc::TryRecvError::Empty) => None,
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

fn is_surge_fx(candidate: &Candidate) -> bool {
    candidate.plugin_id == crate::surge_fx_states::PLUGIN_ID
}

/// Surge XT Effects presets stay only with a prepared state: preparing one in a
/// temporary instance off the main thread crashes the plugin (ADR 0028).
fn attach_surge_fx_states(
    candidates: Vec<Candidate>,
    table: Option<Result<crate::surge_fx_states::Table, String>>,
    read: impl Fn(&Path) -> std::io::Result<Vec<u8>>,
) -> (Vec<Candidate>, Option<String>) {
    let table = match table {
        None => return (candidates, None),
        Some(Ok(table)) => Ok(table),
        Some(Err(error)) => Err(error),
    };
    let mut dropped = 0;
    let kept = candidates
        .into_iter()
        .filter_map(|mut candidate| {
            if !is_surge_fx(&candidate) {
                return Some(candidate);
            }
            let state = table
                .as_ref()
                .ok()
                .and_then(|table| table.state(&read(&candidate.preset.path).ok()?));
            if state.is_none() {
                dropped += 1;
            }
            candidate.state = Some(state?.into());
            Some(candidate)
        })
        .collect();
    let notice = match table {
        Err(error) => Some(format!("Surge XT Effects presets unavailable: {error}")),
        Ok(_) if dropped > 0 => Some(format!(
            "Surge XT Effects: {dropped} presets unavailable (no prepared state)"
        )),
        Ok(_) => None,
    };
    (kept, notice)
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

    fn surge(name: &str) -> Candidate {
        let mut candidate =
            crate::effect_browser::tests::candidate("surge", name, "Space", "Reverb");
        candidate.plugin_id = crate::surge_fx_states::PLUGIN_ID.into();
        candidate
    }

    fn table(preset: &[u8]) -> crate::surge_fx_states::Table {
        use sha2::{Digest, Sha256};
        let hash: String = Sha256::digest(preset)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let json = serde_json::json!({
            "format_version": 1,
            "plugin_id": crate::surge_fx_states::PLUGIN_ID,
            "states": {hash: {"path": "Space/Known", "state": "0102"}}
        });
        crate::surge_fx_states::Table::parse(json.to_string().as_bytes()).unwrap()
    }

    fn read(path: &Path) -> std::io::Result<Vec<u8>> {
        match path.to_string_lossy().rsplit('/').next() {
            Some("Known") => Ok(b"known".to_vec()),
            Some("Edited") => Ok(b"edited".to_vec()),
            _ => Err(std::io::ErrorKind::NotFound.into()),
        }
    }

    fn names(candidates: &[Candidate]) -> Vec<&str> {
        candidates.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn surge_fx_presets_keep_only_prepared_states_and_report_the_rest() {
        let other = crate::effect_browser::tests::candidate("dragonfly", "Hall", "Space", "Reverb");
        let (kept, notice) = attach_surge_fx_states(
            vec![other, surge("Known"), surge("Edited"), surge("Missing")],
            Some(Ok(table(b"known"))),
            read,
        );
        assert_eq!(names(&kept), ["Hall", "Known"]);
        assert!(kept[0].state.is_none());
        assert_eq!(kept[1].state.as_deref(), Some(&[1, 2][..]));
        let notice = notice.unwrap();
        assert!(notice.contains("2 presets unavailable"), "{notice}");
        assert!(crate::error_log::is_error(&notice));
    }

    #[test]
    fn missing_table_drops_every_surge_fx_preset_with_its_reason() {
        let other = crate::effect_browser::tests::candidate("dragonfly", "Hall", "Space", "Reverb");
        let (kept, notice) = attach_surge_fx_states(
            vec![other, surge("Known")],
            Some(Err("offline".into())),
            read,
        );
        assert_eq!(names(&kept), ["Hall"]);
        assert!(notice.unwrap().contains("offline"));
    }

    #[test]
    fn catalogs_without_surge_fx_need_no_table() {
        let other = crate::effect_browser::tests::candidate("dragonfly", "Hall", "Space", "Reverb");
        let (kept, notice) = attach_surge_fx_states(vec![other], None, read);
        assert_eq!(names(&kept), ["Hall"]);
        assert!(notice.is_none());
    }

    #[test]
    fn install_refresh_discards_preinstall_worker_and_allows_discovery_again() {
        let (sender, receiver) = mpsc::channel();
        let mut catalog = Catalog {
            started: true,
            receiver: Some(receiver),
            generation: 4,
            ..Default::default()
        };
        catalog.refresh_after_install();
        assert!(!catalog.started);
        assert!(catalog.receiver.is_none());
        assert_eq!(catalog.generation, 5);
        assert!(sender.send((Vec::new(), None)).is_err());
    }

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
