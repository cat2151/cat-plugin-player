use crate::{config, plugin_list::PluginKind, App, SequenceModulation};
use cmrt_tui_core::patch_catalog_cache::{self, cc1};
use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
};

type PitchChangingPatches = HashMap<String, HashSet<String>>;
pub(crate) type PatchSource<'a> = (&'a config::PluginKey, Option<&'a str>);

#[derive(Default)]
pub(crate) struct Cc1PitchCache(OnceCell<PitchChangingPatches>);

impl Cc1PitchCache {
    pub(crate) fn load_on_shortcut(&self) {
        self.0.get_or_init(Self::load);
    }

    pub(crate) fn effective(
        &self,
        choice: Option<SequenceModulation>,
        patch: Option<PatchSource<'_>>,
    ) -> Option<SequenceModulation> {
        if choice != Some(SequenceModulation::Sweep) {
            return choice;
        }
        let changes_pitch = self
            .0
            .get()
            .zip(patch)
            .is_some_and(|(patches, (plugin, display))| {
                plugin.format == "CLAP"
                    && display.is_some_and(|display| {
                        patches
                            .get(plugin.id.trim())
                            .is_some_and(|group| group.contains(display))
                    })
            });
        if changes_pitch {
            Some(SequenceModulation::Fixed(0))
        } else {
            choice
        }
    }

    fn load() -> PitchChangingPatches {
        let Some(path) = patch_catalog_cache::cache_file_path() else {
            return PitchChangingPatches::new();
        };
        match cc1::Cache::load_next_to(&path) {
            Ok(Some(cache)) => Self::index(cache),
            Ok(None) => PitchChangingPatches::new(),
            Err(error) => {
                eprintln!("CC1 pitch catalog unavailable: {error:#}");
                PitchChangingPatches::new()
            }
        }
    }

    fn index(cache: cc1::Cache) -> PitchChangingPatches {
        let mut patches = PitchChangingPatches::new();
        for record in cache.records.into_iter().filter(|r| r.pitch_change_likely) {
            let Some(id) = record.patch.plugin.as_str().strip_prefix("id:") else {
                continue;
            };
            patches
                .entry(id.into())
                .or_default()
                .insert(record.patch.display);
        }
        patches
    }
}

impl App {
    pub(crate) fn current_cc1_modulation(
        &self,
        choice: Option<SequenceModulation>,
    ) -> Option<SequenceModulation> {
        let patch = self
            .instances
            .iter()
            .find(|instance| instance.kind == PluginKind::Instrument)
            .map(|instance| (&instance.plugin, instance.preset_name.as_deref()));
        self.cc1_pitch_cache.effective(choice, patch)
    }

    pub(crate) fn refresh_cc1_modulation(&self) {
        if self.sequence_modulation != Some(SequenceModulation::Sweep)
            || self.cc1_pitch_cache.0.get().is_none()
        {
            return;
        }
        let modulation = self.current_cc1_modulation(self.sequence_modulation);
        for voice in self
            .instances
            .iter()
            .filter_map(|instance| instance.voice.as_ref())
        {
            voice.set_modulation(modulation);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SequenceModulation::{Fixed, Sweep};

    #[test]
    fn sweep_uses_zero_only_on_the_matching_pitch_changing_patch_after_load() {
        let plugin = cmrt_core::PluginKey::from_identity(Some("test.instrument"), "");
        let record = |display: &str, classification| {
            cc1::Record::new(
                cmrt_core::PatchRef {
                    plugin: plugin.clone(),
                    display: display.into(),
                },
                cc1::Observation::Routing {
                    plugin_version: "test".into(),
                    classification,
                    evidence: String::new(),
                },
            )
        };
        let patches = Cc1PitchCache::index(cc1::Cache {
            format_version: cc1::FORMAT_VERSION,
            records: vec![
                record("Factory/Vibrato", cc1::RoutingClassification::Vibrato),
                record(
                    "Factory/Filter",
                    cc1::RoutingClassification::NonPitchRouting,
                ),
            ],
        });
        let cache = Cc1PitchCache::default();
        let plugin = config::PluginKey {
            format: "CLAP".into(),
            id: "test.instrument".into(),
            name: String::new(),
            vendor: String::new(),
            bundle_path: String::new(),
        };
        let other_plugin = config::PluginKey {
            id: "other.instrument".into(),
            ..plugin.clone()
        };
        let vibrato = Some((&plugin, Some("Factory/Vibrato")));
        assert_eq!(cache.effective(Some(Sweep), vibrato), Some(Sweep));
        cache.0.set(patches).unwrap();
        assert_eq!(cache.effective(None, vibrato), None);
        assert_eq!(cache.effective(Some(Fixed(127)), vibrato), Some(Fixed(127)));
        assert_eq!(cache.effective(Some(Sweep), vibrato), Some(Fixed(0)));
        for other in [
            Some((&plugin, Some("Factory/Filter"))),
            Some((&plugin, Some("User/Vibrato"))),
            Some((&other_plugin, Some("Factory/Vibrato"))),
            Some((&plugin, None)),
            None,
        ] {
            assert_eq!(cache.effective(Some(Sweep), other), Some(Sweep));
        }
        assert_eq!(cache.effective(Some(Sweep), vibrato), Some(Fixed(0)));
    }
}
