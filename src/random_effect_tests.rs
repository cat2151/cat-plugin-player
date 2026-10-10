use super::*;
use crate::{
    config::PluginKey,
    plugin_list::PluginKind,
    random_effect_catalog::{Candidate, Catalog},
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn settle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.scanning || app.pending.is_some() || app.random_effect.busy() {
        assert!(Instant::now() < deadline, "{}", app.status);
        app.host.pump();
        app.handle_events();
        app.poll_random_effect();
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn candidate(app: &App, name: &str) -> Candidate {
    app.random_effect_catalog
        .installed(&app.plugins)
        .find(|candidate| candidate.preset.display.starts_with(name))
        .expect("required effect catalog preset missing")
        .clone()
}

fn apply(app: &mut App, candidate: &Candidate, target: Option<i32>, bytes: Option<Vec<u8>>) {
    let plugin = app
        .plugins
        .iter()
        .find(|plugin| crate::random_effect_catalog::matches_plugin(candidate, plugin))
        .unwrap();
    let state = bytes.unwrap_or_else(|| prepare(candidate, &plugin.bundle_path, 48000.0).unwrap());
    app.apply_random_effect(Prepared {
        candidate: candidate.clone(),
        target,
        state,
        browse: false,
    });
    settle(app);
}

fn click_random_effect(app: &mut App, ctx: &egui::Context, width: f32) {
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width, 600.0),
        )),
        ..Default::default()
    };
    let draw = |app: &mut App, ctx: &egui::Context| {
        egui::CentralPanel::default().show(ctx, |ui| app.playback_controls(ui));
    };
    let _ = ctx.run(input(), |ctx| draw(app, ctx));
    let frame = ctx.run(input(), |ctx| draw(app, ctx));
    let position = frame
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Random effect" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .expect("Random effect must render even without Random patch candidates");
    for pressed in [true, false] {
        let mut input = input();
        input.events = vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ];
        let _ = ctx.run(input, |ctx| draw(app, ctx));
    }
}

#[test]
#[ignore = "requires installed Dexed and Dragonfly Hall/Room/Plate CLAP; real shim test"]
fn native_random_effect_add_replace_failure_and_restore() {
    let _library = crate::native_library::load().unwrap();
    let directory = std::env::temp_dir().join(format!("cat-random-effect-{}", std::process::id()));
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
    app.random_effect_catalog = Catalog::from_candidates({
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
    assert!(!app
        .random_effect_candidates(Some(before[0]))
        .any(|c| c.plugin_id == hall.plugin_id));
    assert!(app
        .random_effect_candidates(Some(before[1]))
        .any(|c| c.plugin_id == hall.plugin_id));
    apply(&mut app, &plate, Some(before[0]), None);
    assert_eq!(app.effect_ids().len(), 2);
    assert_eq!(app.effect_ids()[1], before[1]);
    assert_eq!(app.host.save_state(before[1]).unwrap(), neighbor_state);
    assert_eq!(app.instances[1].plugin.id, plate.plugin_id);
    assert!(app.effect_bypassed);
    // Preparation and asynchronous load gates cover both random buttons.
    let ctx = egui::Context::default();
    for iteration in 0..8 {
        let pattern = if iteration % 2 == 0 {
            crate::SequencePattern::Off
        } else {
            crate::SequencePattern::Steps
        };
        app.sequence_pattern = pattern;
        let selected = pattern.selection(app.selected_sequence);
        click_random_effect(
            &mut app,
            &ctx,
            if iteration % 2 == 0 { 900.0 } else { 600.0 },
        );
        assert!(app.actions_busy());
        assert!(app.random_busy());
        let ids = app.effect_ids();
        app.start_random_effect(&ctx);
        app.start_random_patch(&ctx);
        assert!(!app.remove_plugin(ids[0]));
        settle(&mut app);
        assert_eq!(app.effect_ids().len(), 2, "{}", app.status);
        assert_eq!(app.instrument_id(), Some(source_id));
        assert_eq!(app.sequence_pattern, pattern);
        assert_eq!(app.selected_sequence, selected);
        assert!(app.effect_bypassed);
    }
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
        app.status.contains("Random effect failed"),
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
        app.status.contains("Random effect failed"),
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
    app.random_effect_instance_created(
        plugin,
        crate::random_effect_apply::Replacement {
            prepared: Prepared {
                candidate: bad,
                target: Some(ids[0]),
                state: vec![1],
                browse: false,
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
