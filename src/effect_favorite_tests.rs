use super::*;
use crate::{SequenceModulation, SequencePattern, SequenceVelocity};

#[test]
#[ignore = "requires installed Dexed and Dragonfly Hall Reverb CLAP"]
fn native_effect_recall_preserves_playback_when_reconnecting() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.restore = None;
    app.restore_effect = Default::default();
    app.output = None;
    app.deferred_scan = false;
    let dir = std::env::temp_dir().join(format!("cat-effect-recall-{}", std::process::id()));
    assert!(!dir.exists());
    let path = dir.join("config.toml");
    app.config_path = Ok(path.clone());
    app.initialize_favorites();
    app.start_scan(false);
    settle(&mut app);
    let dexed = app
        .plugins
        .iter()
        .position(|p| p.name == "Dexed" && p.format == "CLAP")
        .unwrap();
    let reverb = app
        .plugins
        .iter()
        .position(|p| p.name == "Dragonfly Hall Reverb" && p.format == "CLAP")
        .unwrap();
    app.load_plugin(dexed);
    settle(&mut app);
    app.load_plugin(reverb);
    settle(&mut app);
    let source = app.instrument_id().unwrap();
    let effect = app.effect_id().unwrap();
    app.sequence_pattern = SequencePattern::Steps;
    app.add_favorite(effect);
    let favorite = app.favorites.library.entries[0].id.clone();
    assert!(app.remove_plugin(effect));
    // Reload metadata too: effect entries omit playback controls on disk.
    app.favorites.library = Library::load(&path).unwrap();
    for pattern in [SequencePattern::GuitarArpeggio, SequencePattern::Off] {
        app.sequence_pattern = pattern;
        app.selected_sequence = SequencePattern::GuitarArpeggio;
        app.sequence_velocity = SequenceVelocity::Range40To100;
        app.sequence_modulation = SequenceModulation::Sweep;
        for reconnect in [true, false] {
            app.load_favorite(&favorite);
            settle(&mut app);
            assert_eq!(app.sequence_pattern, pattern, "reconnect={reconnect}");
            assert_eq!(app.selected_sequence, SequencePattern::GuitarArpeggio);
            assert_eq!(app.sequence_velocity, SequenceVelocity::Range40To100);
            assert_eq!(app.sequence_modulation, SequenceModulation::Sweep);
            assert_eq!(app.instrument_id(), Some(source));
            assert!(app.effect_id().is_some(), "{}", app.status);
            assert_eq!(
                crate::status::Status::load(&path).unwrap().sequence_pattern,
                pattern
            );
        }
        assert!(app.remove_plugin(app.effect_id().unwrap()));
    }
    drop(app);
    std::fs::remove_dir_all(dir).unwrap();
}

fn settle(app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while app.pending.is_some() || app.scanning {
        assert!(
            std::time::Instant::now() < deadline,
            "native operation timed out"
        );
        app.host.pump_startup();
        app.handle_events();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
#[ignore = "requires installed Dexed and Dragonfly Hall/Room/Plate Reverb CLAP"]
fn native_effect_favorites_keep_positions_and_restore_every_selection() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.restore = None;
    app.restore_effect.clear();
    app.output = None;
    app.deferred_scan = false;
    let dir =
        std::env::temp_dir().join(format!("cat-effect-chain-favorites-{}", std::process::id()));
    assert!(!dir.exists());
    let path = dir.join("config.toml");
    app.config_path = Ok(path.clone());
    app.initialize_favorites();
    app.start_scan(false);
    settle(&mut app);
    for name in [
        "Dexed",
        "Dragonfly Hall Reverb",
        "Dragonfly Room Reverb",
        "Dragonfly Plate Reverb",
    ] {
        let index = app
            .plugins
            .iter()
            .position(|p| p.name == name && p.format == "CLAP")
            .expect("required favorite-test plugin missing");
        app.load_plugin(index);
        settle(&mut app);
    }
    let ids = app.effect_ids();
    let mut favorites = Vec::new();
    for id in &ids {
        app.add_favorite(*id);
        favorites.push(app.favorites.library.entries[0].clone());
    }
    for favorite in &favorites {
        app.load_favorite(&favorite.id);
    }
    assert_eq!(
        app.effect_ids(),
        ids,
        "existing favorite keeps its chain position and instance"
    );
    assert!(app.reorder_effect(ids[2], ids[0], false));
    let selected = vec![
        favorites[2].id.clone(),
        favorites[0].id.clone(),
        favorites[1].id.clone(),
    ];
    assert_eq!(
        crate::status::Status::load(&path)
            .unwrap()
            .favorites
            .effects,
        selected
    );
    // UI removes the matching middle favorite, not the first chain effect.
    super::tests::click_favorite(&mut app, &favorites[0].name, 500.0, true);
    assert_eq!(app.effect_ids(), [ids[2], ids[1]]);
    app.load_favorite(&favorites[0].id);
    settle(&mut app);
    assert_eq!(app.effect_ids()[..2], [ids[2], ids[1]]);
    let added = app.effect_ids()[2];
    app.load_favorite(&favorites[0].id);
    assert_eq!(app.effect_ids(), [ids[2], ids[1], added]);
    let expected = app.instances[1..]
        .iter()
        .map(|i| i.plugin.clone())
        .collect::<Vec<_>>();
    for id in app.instances.iter().map(|i| i.id).collect::<Vec<_>>() {
        app.save_plugin_state(id).unwrap();
    }
    let saved = crate::status::Status::load(&path).unwrap();
    let selected = saved.favorites.effects.clone();
    drop(app);
    let mut next = App::prepare();
    next.config_path = Ok(path.clone());
    next.output = None;
    next.restore = saved.last_played;
    next.restore_effect = saved.effects.into();
    next.favorites.restore = saved.favorites;
    next.deferred_scan = false;
    next.restore_before_gui();
    settle(&mut next);
    assert_eq!(
        next.instances[1..]
            .iter()
            .map(|i| i.plugin.clone())
            .collect::<Vec<_>>(),
        expected
    );
    for (id, favorite) in next.effect_ids().iter().zip(selected) {
        assert!(next.favorites.active.contains(&(*id, favorite)));
    }
    drop(next);
    std::fs::remove_dir_all(dir).unwrap();
}
