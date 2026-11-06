use super::*;

pub(crate) fn candidate(id: &str) -> Candidate {
    let measurement = cmrt_tui_core::patch_load::PatchLoadMeasurement {
        drum_kit: true,
        drum_kit_notes: Some(vec![127, 0, 127, 255]),
        drum_kit_note_names: vec![(0, "Original name".into())],
        ..Default::default()
    };
    Candidate {
        format: "CLAP".into(),
        plugin_id: id.into(),
        plugin_name: id.into(),
        display: "Same kit".into(),
        path: "X:/kits/same.sfz".into(),
        bundle_path: "X:/plugins/synth.clap".into(),
        entry: cmrt_patch_select::PatchCatalogEntry::from_display("Same kit".into()),
        measurement,
    }
}

#[test]
fn metadata_preserves_unknown_empty_endpoints_and_source_names() {
    let mut kit = candidate("a");
    assert_eq!(notes(&kit), Some(vec![0, 127]));
    assert_eq!(note_name(&kit, 0), "Original name");
    assert_eq!(note_name(&kit, 127), "名前不明");
    assert_eq!(one_shot(&kit, 127), "not one-shot");
    kit.path = "X:/kits/percussion.floe-preset".into();
    assert_eq!(one_shot(&kit, 127), "one-shot: 未確認");
    kit.measurement.drum_kit_notes = None;
    assert_eq!(notes(&kit), None);
    kit.measurement.drum_kit_notes = Some(vec![]);
    assert_eq!(notes(&kit), Some(vec![]));
}

#[test]
fn independent_cursor_roundtrip_and_plugin_identity() {
    let a = candidate("a");
    let b = candidate("b");
    let mut saved = Saved {
        query: "kit".into(),
        selected: Some(identity(&b)),
        focus: Focus::Notes,
        ..Default::default()
    };
    saved.set_note(&a, 0);
    saved.set_note(&b, 127);
    let bytes = serde_json::to_vec(&saved).unwrap();
    let restored: Saved = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(saved, restored);
    assert_eq!(restored.note(&a), Some(0));
    assert_eq!(restored.note(&b), Some(127));
    let mut relocated = b.clone();
    relocated.bundle_path = "X:/moved/synth.clap".into();
    assert!(same(&identity(&b), &identity(&relocated)));
}

#[test]
#[ignore = "read-only installed catalog acceptance"]
fn installed_drum_metadata_matches_shared_reader() {
    let loaded = cmrt_tui_core::patch_catalog_cache::load().unwrap();
    let (snapshot, _) = loaded.into_parts();
    let expected: Vec<_> = snapshot
        .audio_patches()
        .iter()
        .filter(|p| {
            snapshot
                .load_measurements()
                .get(&p.reference.display)
                .is_some_and(|m| m.drum_kit)
        })
        .map(|p| p.reference.display.clone())
        .collect();
    let actual = crate::patch_catalog::load().unwrap();
    let kits: Vec<_> = actual.iter().filter(|c| c.measurement.drum_kit).collect();
    assert!(!kits.is_empty());
    assert_eq!(kits.len(), expected.len());
    for kit in &kits {
        assert!(expected.contains(&kit.display));
        let source = &snapshot.load_measurements()[&kit.display];
        assert_eq!(kit.measurement.drum_kit_notes, source.drum_kit_notes);
        assert_eq!(
            kit.measurement.drum_kit_note_names,
            source.drum_kit_note_names
        );
        assert_eq!(
            kit.measurement.drum_kit_one_shot_notes,
            source.drum_kit_one_shot_notes
        );
    }
    println!("{} kits match shared reader metadata", kits.len());
}

#[test]
fn native_drum_heavy_gate_stop_stale_results_and_cursor_file() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = crate::App::prepare();
    app.output = None;
    app.pending = None;
    app.restoring = false;
    app.deferred_scan = false;
    let mut heavy = candidate("heavy");
    heavy.measurement.sfz_sample_bytes = Some(64_000_000);
    app.drum_browser.kits = vec![candidate("light"), heavy];
    app.drum_browser.results = vec![0, 1];
    app.drum_browser.open = true;
    app.select_drum_kit(1);
    assert!(app.patch_browser.requests.desired.is_none());
    app.request_drum_audition(Audition::Note(127), false);
    assert!(app.patch_browser.requests.desired.is_none());
    app.request_drum_audition(Audition::Note(127), true);
    assert!(app.patch_browser.requests.desired.is_some());
    assert!(app.patch_browser.requests.confirmation.is_none());
    let old = app.patch_browser.requests.next().unwrap();
    app.select_drum_kit(0);
    assert!(!app.patch_browser.requests.current(&old));
    assert!(app.patch_browser.requests.ready().is_none());
    assert!(app.patch_browser.requests.next().is_some());
    assert!(app.patch_browser.requests.ready().is_some());
    app.close_drum_browser();
    app.patch_browser.requests.finish(true);
    app.drum_patch_finished(true);
    assert!(app.patch_browser.requests.applied.is_some());
    assert!(app.drum_browser.awaiting.is_none());
    assert_eq!(app.sequence_pattern, crate::SequencePattern::Off);
    let config = app.config_path.as_ref().unwrap();
    let restored = Saved::load(config).unwrap();
    assert!(same(
        restored.selected.as_ref().unwrap(),
        &identity(&app.drum_browser.kits[0])
    ));
    let file = config.with_file_name("drum-kit-browser.json");
    std::fs::write(&file, b"invalid").unwrap();
    assert!(restored.save(config).is_err());
    assert_eq!(std::fs::read(file).unwrap(), b"invalid");
}

#[test]
fn native_drum_startup_restores_cursor_with_autostart() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let config = crate::config::path().unwrap();
    let kit = candidate("kit-plugin");
    let mut saved = Saved {
        selected: Some(identity(&kit)),
        applied: Some(identity(&kit)),
        focus: Focus::Notes,
        ..Default::default()
    };
    saved.set_note(&kit, 127);
    saved.save(&config).unwrap();
    let mut status = crate::status::Status {
        sequence_pattern: crate::SequencePattern::Off,
        ..Default::default()
    };
    status.patch_browser.playing = Some(identity(&kit));
    status.write(&config).unwrap();
    let mut app = crate::App::prepare();
    assert!(app.auto_start_pending);
    assert_eq!(app.sequence_pattern, crate::SequencePattern::Off);
    assert_eq!(app.drum_browser.saved, saved);
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
}

#[test]
#[ignore = "exports an installed kit DTO/state to CAT_KIT_CONTRACT_FIXTURE for editor offline acceptance"]
fn installed_kit_contract_fixture() {
    let target =
        std::env::var_os("CAT_KIT_CONTRACT_FIXTURE").expect("external fixture destination");
    let catalog = crate::patch_catalog::load().unwrap();
    let candidate = catalog
        .iter()
        .find(|c| c.measurement.drum_kit && c.display.contains("CR-909"))
        .expect("installed CR-909 kit");
    let kit = kit_metadata(candidate, 1).unwrap();
    let reference = crate::patch_reference::PatchReference {
        plugin: crate::host::PluginKey {
            format: candidate.format.clone(),
            id: candidate.plugin_id.clone(),
            name: candidate.plugin_name.clone(),
            vendor: String::new(),
            bundle_path: candidate.bundle_path.to_string_lossy().into(),
        },
        source: crate::patch_reference::Source::Catalog {
            path: candidate.path.clone(),
            file: candidate.path.clone(),
        },
        display: candidate.display.clone(),
    };
    let state = crate::state_request::generate(&reference).unwrap();
    assert!(!state.is_empty());
    std::fs::write(
        target,
        serde_json::to_vec(&(reference, kit, state)).unwrap(),
    )
    .unwrap();
}
