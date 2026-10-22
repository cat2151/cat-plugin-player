use crate::{config::PluginKey, favorites_store::Library, patch_catalog::Candidate};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

pub(super) const SEED: u64 = 0x5355_5247_4541_5454;

pub(super) struct Patch {
    pub label: String,
    pub plugin: PluginKey,
    pub state: Vec<u8>,
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn select(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut groups: BTreeMap<(String, String), Vec<Candidate>> = BTreeMap::new();
    for candidate in candidates {
        if candidate.plugin_id != crate::patch_catalog::SURGE_ID {
            continue;
        }
        let source = candidate.display.split('/').next().unwrap_or_default();
        if !matches!(source, "patches_factory" | "patches_3rdparty") {
            continue;
        }
        groups
            .entry((
                source.to_owned(),
                candidate
                    .entry
                    .selector_category()
                    .unwrap_or("unclassified")
                    .into(),
            ))
            .or_default()
            .push(candidate);
    }
    let mut rng = StdRng::seed_from_u64(SEED);
    groups
        .into_values()
        .flat_map(|mut group| {
            group.sort_by(|a, b| a.display.cmp(&b.display));
            group.shuffle(&mut rng);
            group.into_iter().take(2)
        })
        .collect()
}

pub(super) fn load_saved(config: &Path) -> Vec<Patch> {
    let root = config.parent().unwrap();
    // Library::load may migrate and save; deserialize directly for this read-only probe.
    let library: Library =
        toml::from_str(&std::fs::read_to_string(root.join("favorites/index.toml")).unwrap())
            .unwrap();
    library
        .entries
        .iter()
        .filter(|favorite| {
            !favorite.effect
                && matches!(
                    (favorite.plugin.format.as_str(), favorite.plugin.id.as_str()),
                    ("CLAP", "org.surge-synth-team.surge-xt")
                        | ("VST3", "ABCDEF019182FAEB566D624153675854")
                )
        })
        .map(|favorite| Patch {
            label: format!("saved/{}/{}", favorite.id, favorite.name),
            plugin: favorite.plugin.clone(),
            state: library.state(config, &favorite.id).unwrap(),
        })
        .collect()
}

pub(super) fn load(config: &Path) -> Vec<Patch> {
    let root = config.parent().unwrap();
    let mut patches = load_saved(config);
    let cached = cmrt_tui_core::patch_catalog_cache::cache_file_path()
        .filter(|path| path.is_file())
        .unwrap_or_else(|| root.join("patch-catalog/surge-xt.json"));
    let catalog = cmrt_tui_core::patch_catalog_cache::load_from(&cached).unwrap();
    let selected = select(crate::patch_catalog::candidates(catalog).unwrap());
    assert!(selected.len() >= 50, "need at least 50 catalog patches");
    for candidate in selected {
        assert!(candidate.path.is_file(), "missing {}", candidate.display);
        let plugin = PluginKey {
            format: candidate.format,
            id: candidate.plugin_id,
            name: candidate.plugin_name,
            vendor: "Surge Synth Team".into(),
            bundle_path: candidate.bundle_path.to_string_lossy().into_owned(),
        };
        let state = crate::state_request::catalog(
            &plugin.id,
            Path::new(&plugin.bundle_path),
            &candidate.path,
        )
        .unwrap();
        patches.push(Patch {
            label: candidate.display,
            plugin,
            state,
        });
    }
    assert!(patches.iter().any(|patch| patch.plugin.format == "CLAP"));
    patches
}

#[test]
fn fixed_seed_sampling_covers_sources_and_categories_independent_of_input_order() {
    let candidate = |source: &str, category: &str, number: u32| {
        let display = format!("{source}/{category}/{number}.fxp");
        Candidate {
            format: "CLAP".into(),
            plugin_id: crate::patch_catalog::SURGE_ID.into(),
            plugin_name: "Surge XT".into(),
            display: display.clone(),
            path: display.clone().into(),
            bundle_path: "X:/plugins/Surge XT.clap".into(),
            entry: cmrt_patch_select::PatchCatalogEntry::new(
                display.clone(),
                display.to_lowercase(),
                "Surge XT".into(),
                Some(category.into()),
            ),
            measurement: Default::default(),
        }
    };
    let mut items = Vec::new();
    for source in ["patches_factory", "patches_3rdparty"] {
        for category in ["Basses", "Pads"] {
            for number in 0..10 {
                items.push(candidate(source, category, number));
            }
        }
    }
    let expected: Vec<_> = select(items.clone())
        .into_iter()
        .map(|x| x.display)
        .collect();
    assert_eq!(expected.len(), 8);
    items.reverse();
    let actual: Vec<_> = select(items).into_iter().map(|x| x.display).collect();
    assert_eq!(actual, expected);
    assert_eq!(
        select(vec![candidate("patches_factory", "Single", 0)]).len(),
        1
    );
    assert!(select(vec![candidate("other", "Pads", 0)]).is_empty());
}
