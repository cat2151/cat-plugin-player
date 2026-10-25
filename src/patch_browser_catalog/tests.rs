use super::*;

#[test]
fn drum_categories_are_exclusive_and_dexed_shares_instrument_categories() {
    let snapshot = Snapshot::build(
        vec![
            candidate("vapor", "Vaporizer2", "DR Hat 001.vvp", Some("Drum")),
            candidate("vapor", "Vaporizer2", "DR Mystery.vvp", Some("Drum")),
            candidate("vapor", "Vaporizer2", "DR Kick.vvp", Some("Drum")),
            candidate("dexed", "Dexed", "bank.syx/01 E.PIANO", None),
            candidate("surge", "Surge XT", "Piano.fxp", Some("Keys")),
        ],
        &[],
    )
    .unwrap();
    assert_drum_partition(&snapshot);
    let chord = FilterGroup::ALL
        .iter()
        .position(|g| g.role() == Some(cmrt_patches::PatchRole::Chord))
        .unwrap();
    let piano = snapshot.presets[chord]
        .iter()
        .position(|p| p.label == "keyboard|keys|piano")
        .unwrap();
    assert_eq!(snapshot.filter(chord, piano, "").unwrap().len(), 2);
    assert_eq!(
        snapshot.filter(chord, piano, "plugin:dexed").unwrap().len(),
        1
    );
    assert_eq!(
        snapshot
            .filter(chord, piano, "-plugin:dexed")
            .unwrap()
            .len(),
        1
    );
    assert!(snapshot
        .presets
        .iter()
        .flatten()
        .all(|p| !p.label.contains("FM ")));
}

fn assert_drum_partition(snapshot: &Snapshot) {
    let drum = FilterGroup::ALL
        .iter()
        .position(|g| g.role() == Some(cmrt_patches::PatchRole::Drum))
        .unwrap();
    let parts: Vec<_> = snapshot.presets[drum]
        .iter()
        .enumerate()
        .filter(|(_, p)| p.pattern.is_some())
        .collect();
    assert_eq!(
        parts
            .iter()
            .map(|(_, p)| p.label.as_str())
            .collect::<Vec<_>>(),
        ["kick", "snare", "hi-hat", "percussion"]
    );
    let mut memberships = std::collections::HashMap::new();
    for (preset, part) in parts {
        for index in snapshot.filter(drum, preset, "").unwrap() {
            assert!(
                memberships.insert(index, part.label.as_str()).is_none(),
                "overlapping drum preset"
            );
        }
    }
    assert_eq!(
        memberships.len(),
        snapshot.filter(drum, 0, "").unwrap().len()
    );
    let hats: Vec<_> = snapshot
        .candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.plugin_name == "Vaporizer2" && c.display.to_lowercase().contains(" hat ")
        })
        .collect();
    assert!(!hats.is_empty());
    for (index, candidate) in &hats {
        assert_eq!(
            memberships.get(index),
            Some(&"hi-hat"),
            "{}",
            candidate.display
        );
    }
    println!(
        "Drum partition: {} patches, Vaporizer2 spaced hats: {}",
        memberships.len(),
        hats.len()
    );
}

#[test]
#[ignore = "read-only verification of the installed catalog; no GUI or audio"]
fn installed_drum_categories_survive_catalog_reload() {
    for _ in 0..2 {
        let snapshot = Snapshot::build(crate::patch_catalog::load().unwrap(), &[]).unwrap();
        assert_drum_partition(&snapshot);
        assert!(snapshot
            .presets
            .iter()
            .flatten()
            .all(|p| !p.label.contains("FM ")));
        let saved = crate::status::Status::load(&crate::config::path().unwrap())
            .unwrap()
            .patch_browser;
        let expected_preset = saved.preset.clone();
        let expected_selected = saved.selected.clone();
        let mut browser = crate::patch_browser::Browser::restored(saved);
        browser.snapshot = Some(snapshot);
        browser.resolve_restored_preset();
        browser.resolve_restored_playing();
        browser.filter(false);
        browser.reveal_playing();
        let restored = browser.saved_filter();
        assert_eq!(restored.preset, expected_preset);
        assert_eq!(restored.selected, expected_selected);
        assert!(
            browser.requests.desired.is_none(),
            "restore must not load audio"
        );
    }
}

pub(crate) fn candidate(id: &str, name: &str, display: &str, category: Option<&str>) -> Candidate {
    Candidate {
        format: "CLAP".into(),
        plugin_id: id.into(),
        plugin_name: name.into(),
        display: display.into(),
        path: format!("X:/patches/{id}/{display}").into(),
        bundle_path: format!("X:/plugins/{id}.clap").into(),
        entry: PatchCatalogEntry::new(
            display.into(),
            display.to_lowercase(),
            name.into(),
            category.map(str::to_owned),
        ),
        measurement: Default::default(),
    }
}

#[test]
fn shared_search_roles_aliases_and_sorted_identity_stay_aligned() {
    let mut a = candidate("a", "Alpha", "Same", Some("Bass"));
    a.entry = a.entry.with_merged(3, vec!["Hidden alias".into()]);
    let b = candidate("b", "Beta", "Same", Some("Lead"));
    let dexed = candidate("dexed", "Dexed", "Bank.syx/01 LAURIE", None);
    let snapshot = Snapshot::build(vec![b, dexed, a], &[]).unwrap();
    for (index, candidate) in snapshot.candidates.iter().enumerate() {
        assert_eq!(candidate.entry, snapshot.entries[index]);
    }
    let bass = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Bass))
        .unwrap();
    let lead = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Lead))
        .unwrap();
    assert_eq!(
        snapshot.candidates[snapshot.filter(bass, 0, "").unwrap()[0]].plugin_id,
        "a"
    );
    assert_eq!(
        snapshot.candidates[snapshot.filter(lead, 0, "").unwrap()[0]].plugin_id,
        "b"
    );
    let aliases = snapshot
        .filter(0, 0, "hidden -bright plugin:alpha")
        .unwrap();
    assert_eq!(aliases.len(), 1);
    assert_eq!(snapshot.candidates[aliases[0]].entry.merged_count(), 3);
    let indices: Vec<_> = (0..snapshot.entries.len()).collect();
    assert_eq!(
        snapshot.filter(0, 0, "plugin:alpha plugin:beta").unwrap(),
        filter_candidates(&snapshot.entries, &indices, "plugin:alpha plugin:beta").unwrap()
    );
    assert!(snapshot.filter(0, 0, "[").is_err());
    assert!(snapshot.filter(0, 0, "no-such-patch").unwrap().is_empty());
    let programs = snapshot.filter(0, 0, "plugin:dexed").unwrap();
    assert!(snapshot.candidates[programs[0]]
        .path
        .to_string_lossy()
        .ends_with("01 LAURIE"));
}

#[test]
fn user_conditions_reclassify_with_shared_rules() {
    let users = cmrt_patch_select::prepare_user_presets(vec![("bass".into(), "zebra".into())]);
    let snapshot = Snapshot::build(vec![candidate("a", "Alpha", "Zebra", None)], &users).unwrap();
    let bass = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Bass))
        .unwrap();
    let preset = snapshot.presets[bass]
        .iter()
        .position(|preset| preset.is_user)
        .unwrap();
    assert_eq!(snapshot.filter(bass, preset, "").unwrap(), [0]);
}

#[test]
#[ignore = "read-only installed catalog and performance acceptance"]
fn installed_browser_snapshot_and_condition_timings() {
    let started = std::time::Instant::now();
    let candidates = crate::patch_catalog::load().unwrap();
    let loaded = started.elapsed();
    let count = candidates.len();
    assert!(candidates
        .iter()
        .any(|candidate| candidate.entry.merged_count() > 1));
    assert!(candidates
        .iter()
        .any(|candidate| candidate.entry.selector_category().is_some()));
    assert!(candidates
        .iter()
        .any(|candidate| candidate.measurement.is_heavy_offline_load()));
    let started = std::time::Instant::now();
    let snapshot = Snapshot::build(candidates, &[]).unwrap();
    let classified = started.elapsed();
    for query in ["pad -bright", "plugin:floe", "plugin:surgext -bass"] {
        let started = std::time::Instant::now();
        let indices = snapshot.filter(0, 0, query).unwrap();
        println!(
            "{query:?}: {} results in {:?}",
            indices.len(),
            started.elapsed()
        );
        assert!(!indices.is_empty());
    }
    let mut browser = crate::patch_browser::Browser {
        open: true,
        snapshot: Some(snapshot),
        ..Default::default()
    };
    let started = std::time::Instant::now();
    browser.filter(false);
    let labeled = started.elapsed();
    assert_eq!(browser.row_labels.len(), count);
    println!("full results including cached display names: {labeled:?}");
    println!("{count} patches: read {loaded:?}, sorted/classified {classified:?}; immutable worker snapshot");
}

#[test]
fn browser_filter_survives_restart_by_label_and_unknown_preset_falls_back_to_all() {
    let users = cmrt_patch_select::prepare_user_presets(vec![("bass".into(), "zebra".into())]);
    let build = || Snapshot::build(vec![candidate("a", "Alpha", "Zebra", None)], &users).unwrap();
    let snapshot = build();
    let bass = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Bass))
        .unwrap();
    let preset = snapshot.presets[bass]
        .iter()
        .position(|preset| preset.is_user)
        .unwrap();
    let mut browser = crate::patch_browser::Browser {
        snapshot: Some(snapshot),
        role: bass,
        preset,
        query: "zeb".into(),
        ..Default::default()
    };
    browser.filter(false);
    let root = std::env::temp_dir().join(format!("cat-browser-filter-{}", std::process::id()));
    let config = root.join("config.toml");
    let status = crate::status::Status {
        patch_browser: browser.saved_filter(),
        ..Default::default()
    };
    status.save(&config).unwrap();
    let saved = crate::status::Status::load(&config).unwrap().patch_browser;
    std::fs::remove_dir_all(&root).unwrap();
    let mut restored = crate::patch_browser::Browser::restored(saved.clone());
    // A session that never opens the browser keeps the conditions for the next one.
    assert_eq!(restored.saved_filter(), saved);
    restored.snapshot = Some(build());
    restored.resolve_restored_preset();
    restored.filter(false);
    assert_eq!(
        (restored.role, restored.preset, restored.query.as_str()),
        (bass, preset, "zeb")
    );
    assert_eq!(restored.results, browser.results);
    let mut missing = crate::patch_browser::Browser::restored(crate::status::BrowserFilter {
        preset: "deleted condition".into(),
        ..saved
    });
    missing.snapshot = Some(build());
    missing.resolve_restored_preset();
    assert_eq!((missing.role, missing.preset), (bass, 0));
}

/// Each built-in preset row of the installed catalog lists a patch at most once across rows.
/// `ALL`, `★ Favorite` and user presets are views over the rows, so they are not counted.
#[test]
#[ignore = "read-only installed catalog and pitch tracking table; no GUI or audio"]
fn installed_patches_belong_to_one_preset_row() {
    let snapshot = Snapshot::build(crate::patch_catalog::load().unwrap(), &[]).unwrap();
    let mut rows_by_patch = vec![Vec::new(); snapshot.entries.len()];
    for (role, group) in FilterGroup::ALL.iter().enumerate().skip(1) {
        for preset in &snapshot.presets[role] {
            if preset.pattern.is_none() || preset.is_favorite || preset.is_user {
                continue;
            }
            println!(
                "row {} › {}: {}",
                group.label(),
                preset.label,
                preset.matches.len()
            );
            for &index in preset.matches.iter() {
                rows_by_patch[index].push(format!("{} › {}", group.label(), preset.label));
            }
        }
    }
    let mut overlaps = BTreeMap::<String, usize>::new();
    for rows in rows_by_patch.iter().filter(|rows| rows.len() > 1) {
        *overlaps.entry(rows.join(" + ")).or_default() += 1;
    }
    for (rows, count) in &overlaps {
        println!("overlap {rows}: {count}");
    }
    println!(
        "{} patches, overlapping patches: {}",
        snapshot.entries.len(),
        overlaps.values().sum::<usize>()
    );
    assert!(overlaps.is_empty());
}
