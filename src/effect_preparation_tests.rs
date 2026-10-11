use super::*;
use crate::{
    config::PluginKey,
    effect_catalog::{Candidate, Catalog},
    plugin_list::PluginKind,
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn settle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.scanning || app.pending.is_some() || app.effect_preparation.busy() {
        assert!(Instant::now() < deadline, "{}", app.status);
        app.host.pump();
        app.handle_events();
        app.poll_effect_preparation();
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn candidate(app: &App, name: &str) -> Candidate {
    app.effect_catalog
        .installed(&app.plugins)
        .find(|candidate| candidate.preset.display.starts_with(name))
        .expect("required effect catalog preset missing")
        .clone()
}

fn apply(app: &mut App, candidate: &Candidate, target: Option<i32>, bytes: Option<Vec<u8>>) {
    let plugin = app
        .plugins
        .iter()
        .find(|plugin| crate::effect_catalog::matches_plugin(candidate, plugin))
        .unwrap();
    let state = bytes.unwrap_or_else(|| prepare(candidate, &plugin.bundle_path, 48000.0).unwrap());
    app.apply_effect(Prepared {
        candidate: candidate.clone(),
        target,
        state,
    });
    settle(app);
}

#[test]
#[ignore = "requires installed Dexed and Dragonfly Hall/Room/Plate CLAP; real shim test"]
fn native_effect_add_replace_failure_and_restore() {
    let _library = crate::native_library::load().unwrap();
    let directory =
        std::env::temp_dir().join(format!("cat-effect-preparation-{}", std::process::id()));
    assert!(!directory.exists());
    let path = directory.join("config.toml");
    let mut app = App::prepare();
    app.config_path = Ok(path.clone());
    app.restore = None;
    app.restore_effect.clear();
    app.output = None;
    app.deferred_scan = false;
    app.start_scan(false);
    settle(&mut app);
    app.effect_catalog = Catalog::from_candidates({
        let catalog = cmrt_core::AudioEffectCatalog::discover();
        catalog
            .presets()
            .iter()
            .filter(|preset| preset.display.starts_with("Dragonfly"))
            .map(|preset| Candidate::new(&catalog, preset).unwrap())
            .collect()
    });
    let source = app
        .plugins
        .iter()
        .position(|plugin| plugin.name == "Dexed" && plugin.format == "CLAP")
        .unwrap();
    app.load_plugin(source);
    settle(&mut app);
    let source_id = app.instrument_id().unwrap();
    app.sequence_pattern = crate::SequencePattern::Off;
    app.selected_sequence = crate::SequencePattern::GuitarArpeggio;
    app.initialize_favorites();
    let hall = candidate(&app, "Dragonfly Hall");
    let room = candidate(&app, "Dragonfly Room");
    let plate = candidate(&app, "Dragonfly Plate");
    // Empty chain adds exactly one; subsequent clicks replace its slot.
    apply(&mut app, &hall, None, None);
    assert_eq!(app.effect_ids().len(), 1, "{}", app.status);
    let first = app.effect_ids()[0];
    apply(&mut app, &room, Some(first), None);
    assert_eq!(app.effect_ids().len(), 1, "{}", app.status);
    assert_eq!(app.instances[1].plugin.id, room.plugin_id);
    let hall_index = app
        .plugins
        .iter()
        .position(|plugin| plugin.id == hall.plugin_id && plugin.format == "CLAP")
        .unwrap();
    app.load_plugin(hall_index);
    settle(&mut app);
    let before = app.effect_ids();
    app.effect_bypassed = true;
    let neighbor_state = app.host.save_state(before[1]).unwrap();
    assert!(app.effect_plugin_taken(&hall.plugin_id, Some(before[0])));
    assert!(!app.effect_plugin_taken(&hall.plugin_id, Some(before[1])));
    apply(&mut app, &plate, Some(before[0]), None);
    assert_eq!(app.effect_ids().len(), 2);
    assert_eq!(app.effect_ids()[1], before[1]);
    assert_eq!(app.host.save_state(before[1]).unwrap(), neighbor_state);
    assert_eq!(app.instances[1].plugin.id, plate.plugin_id);
    assert!(app.effect_bypassed);
    // A native state rejection must preserve instance IDs and the disk session.
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/shim/Release/clap_state_test_plugin.clap");
    let key = PluginKey {
        format: "CLAP".into(),
        id: "cat.test.state.effect".into(),
        name: "State effect".into(),
        vendor: "cat test".into(),
        bundle_path: fixture_path.to_string_lossy().into_owned(),
    };
    let mut plugin = app.host.restore_plugin(&key).unwrap();
    plugin.kind = PluginKind::Effect;
    app.plugins.push(plugin);
    let mut bad = hall.clone();
    bad.plugin_id = key.id.clone();
    let ids = app.effect_ids();
    let disk = std::fs::read(crate::status::path(&path)).unwrap();
    apply(&mut app, &bad, Some(ids[0]), Some(vec![0]));
    assert!(
        app.status.contains("Browse effect failed"),
        "{}",
        app.status
    );
    assert_eq!(app.effect_ids(), ids);
    assert_eq!(std::fs::read(crate::status::path(&path)).unwrap(), disk);
    // A plugin with no effect input can load state but cannot form the new chain.
    app.effect_bypassed = false;
    app.save_session();
    let disk = std::fs::read(crate::status::path(&path)).unwrap();
    let source_state = app.host.save_state(source_id).unwrap();
    let mut incompatible = app.plugins[source].clone();
    incompatible.kind = PluginKind::Effect;
    let mut bad_chain = hall.clone();
    bad_chain.plugin_id.clone_from(&incompatible.id);
    app.plugins.push(incompatible);
    apply(&mut app, &bad_chain, Some(ids[0]), Some(source_state));
    assert!(
        app.status.contains("Browse effect failed"),
        "{}",
        app.status
    );
    assert_eq!(app.effect_ids(), ids);
    assert_eq!(std::fs::read(crate::status::path(&path)).unwrap(), disk);
    app.plugins.pop();
    app.effect_bypassed = true;
    app.save_session();
    let disk = std::fs::read(crate::status::path(&path)).unwrap();
    // Creation failure follows the same path without destroying the old effect.
    app.pause_audio();
    let plugin = app.plugins.last().unwrap().clone();
    app.effect_instance_created(
        plugin,
        crate::effect_apply::Replacement {
            prepared: Prepared {
                candidate: bad,
                target: Some(ids[0]),
                state: vec![1],
            },
            old_state: None,
        },
        -1,
        Some("test creation error".into()),
    );
    assert_eq!(app.effect_ids(), ids);
    assert_eq!(std::fs::read(crate::status::path(&path)).unwrap(), disk);
    assert!(app.favorites.library.entries.is_empty());
    let expected = crate::status::Status::load(&path).unwrap().effects;
    drop(app);
    let mut restored = App::prepare();
    restored.config_path = Ok(path.clone());
    restored.output = None;
    restored.deferred_scan = false;
    restored.restore = crate::status::Status::load(&path).unwrap().last_played;
    restored.restore_effect = expected.clone().into();
    restored.effect_bypassed = true;
    restored.restore_before_gui();
    settle(&mut restored);
    assert_eq!(
        restored
            .instances
            .iter()
            .filter(|i| i.kind == PluginKind::Effect)
            .map(|i| i.plugin.id.clone())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|key| key.id.clone())
            .collect::<Vec<_>>()
    );
    assert!(restored.effect_bypassed);
}

#[test]
fn prepared_state_is_delivered_without_a_worker_instance() {
    let mut candidate =
        crate::effect_browser::tests::candidate("surge", "Rotary/Fast", "Modulation", "Rotary");
    candidate.state = Some(vec![7, 8].into());
    let mut preparation = Preparation::default();
    // A bundle that cannot load proves no temporary instance is attempted.
    preparation.start(
        candidate,
        Some(3),
        "X:/missing.clap".into(),
        48_000.0,
        &egui::Context::default(),
    );
    let prepared = preparation.poll().unwrap().unwrap();
    assert_eq!(prepared.state, [7, 8]);
    assert_eq!(prepared.target, Some(3));
    assert!(!preparation.busy());
}

#[test]
#[ignore = "requires installed Surge XT and Surge XT Effects CLAP and ../cat-music-patterns; real shim test"]
fn native_surge_fx_browse_loads_published_states_without_a_temporary_instance() {
    let _library = crate::native_library::load().unwrap();
    let directory = std::env::temp_dir().join(format!("cat-surge-fx-{}", std::process::id()));
    let mut app = App::prepare();
    app.config_path = Ok(directory.join("config.toml"));
    app.restore = None;
    app.restore_effect.clear();
    app.output = None;
    app.deferred_scan = false;
    app.start_scan(false);
    settle(&mut app);
    let table = crate::surge_fx_states::Table::parse(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../cat-music-patterns/surge-xt-fx-states.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let catalog = cmrt_core::AudioEffectCatalog::discover();
    let candidates: Vec<Candidate> = catalog
        .presets()
        .iter()
        .filter_map(|preset| Candidate::new(&catalog, preset))
        .filter(|candidate| candidate.plugin_id == crate::surge_fx_states::PLUGIN_ID)
        .map(|mut candidate| {
            let bytes = std::fs::read(&candidate.preset.path).unwrap();
            candidate.state = Some(table.state(&bytes).unwrap().into());
            candidate
        })
        .collect();
    // Every 15th preset spans the effect types without loading all 291.
    let sample: Vec<Candidate> = candidates.iter().step_by(15).cloned().collect();
    app.effect_catalog = Catalog::from_candidates(candidates);
    let source = app
        .plugins
        .iter()
        .position(|plugin| plugin.name == "Surge XT" && plugin.format == "CLAP")
        .unwrap();
    app.load_plugin(source);
    settle(&mut app);
    let ctx = egui::Context::default();
    let mut target = None;
    for candidate in &sample {
        app.start_effect(candidate.clone(), target, &ctx);
        // The prepared state arrives at once: no worker thread, no temporary instance.
        app.poll_effect_preparation();
        assert!(!app.effect_preparation.busy(), "{}", candidate.name);
        settle(&mut app);
        let ids = app.effect_ids();
        assert_eq!(ids.len(), 1, "{}: {}", candidate.name, app.status);
        assert!(!crate::error_log::is_error(&app.status), "{}", app.status);
        assert_eq!(
            app.host.save_state(ids[0]).unwrap(),
            candidate.state.as_deref().unwrap(),
            "{}",
            candidate.name
        );
        target = Some(ids[0]);
    }
    let _ = std::fs::remove_dir_all(directory);
}
