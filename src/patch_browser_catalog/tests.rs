use super::*;

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
    let candidates = crate::random_patch_catalog::load().unwrap();
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
