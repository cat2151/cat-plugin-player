use super::*;

#[test]
#[ignore = "requires installed Dexed, Dragonfly Hall Reverb and Surge XT VST3"]
fn native_favorites_restore_both_slots_without_overwriting_snapshots() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.restore = None;
    app.restore_effect = None;
    app.output = None;
    app.deferred_scan = false;
    let dir = std::env::temp_dir().join(format!("cat-favorite-native-{}", std::process::id()));
    assert!(!dir.exists());
    let path = dir.join("config.toml");
    app.config_path = Ok(path.clone());
    app.initialize_favorites();
    app.start_scan(false);
    settle(&mut app);
    let find = |name: &str, format: &str| {
        app.plugins
            .iter()
            .position(|p| p.name == name && p.format == format)
            .unwrap()
    };
    let dexed = find("Dexed", "CLAP");
    let surge = find("Surge XT", "VST3");
    let reverb = find("Dragonfly Hall Reverb", "CLAP");
    app.load_plugin(dexed);
    settle(&mut app);
    let source = app.instrument_id().unwrap();
    app.sequence_pattern = crate::SequencePattern::GuitarArpeggio;
    app.add_favorite(source);
    let first = app.favorites.library.entries[0].clone();
    let snapshot = app.favorites.library.state(&path, &first.id).unwrap();
    assert!(!snapshot.is_empty());
    assert_eq!(
        first.sequence_pattern,
        crate::SequencePattern::GuitarArpeggio
    );
    app.sequence_pattern = crate::SequencePattern::Off;
    app.add_favorite(source);
    let stopped = app.favorites.library.entries[0].clone();
    assert_eq!(stopped.sequence_pattern, crate::SequencePattern::Off);
    assert_eq!(app.favorites.library.entries.len(), 2);
    assert_ne!(app.favorites.library.entries[0].id, first.id);
    app.sequence_pattern = crate::SequencePattern::Steps;
    app.load_plugin(reverb);
    settle(&mut app);
    let fx = app.effect_id().unwrap();
    app.add_favorite(fx);
    let effect_favorite = app.favorites.library.entries[0].clone();
    let fx_state = app.host.save_state(fx).unwrap();
    app.effect_bypassed = true;
    app.sequence_pattern = crate::SequencePattern::Off;
    app.load_favorite(&first.id);
    assert_eq!(
        app.instrument_id(),
        Some(source),
        "same-plugin recall must keep the editor instance"
    );
    assert_eq!(app.sequence_pattern, first.sequence_pattern);
    assert_eq!(app.effect_id(), Some(fx));
    assert!(app.effect_bypassed);
    assert_eq!(app.host.save_state(fx).unwrap(), fx_state);
    assert_eq!(app.host.save_state(source).unwrap(), snapshot);
    app.load_favorite(&stopped.id);
    assert_eq!(app.sequence_pattern, crate::SequencePattern::Off);
    assert_eq!(
        crate::config::Config::load(&path).unwrap().sequence_pattern,
        crate::SequencePattern::Off
    );
    app.sequence_velocity = crate::SequenceVelocity::Range40To100;
    app.sequence_modulation = crate::SequenceModulation::Sweep;
    app.load_favorite(&effect_favorite.id);
    assert_eq!(app.sequence_pattern, crate::SequencePattern::Off);
    assert_eq!(app.sequence_velocity, crate::SequenceVelocity::Range40To100);
    assert_eq!(app.sequence_modulation, crate::SequenceModulation::Sweep);
    assert_eq!(app.instrument_id(), Some(source));
    assert_eq!(app.effect_id(), Some(fx));
    assert_eq!(app.host.save_state(source).unwrap(), snapshot);

    // Restart restores the last associations, even when the live sound has
    // changed since recall. It must not reload the immutable favorite snapshot.
    app.save_plugin_state(source).unwrap();
    app.save_plugin_state(fx).unwrap();
    let saved = crate::config::Config::load(&path).unwrap();
    assert_eq!(
        saved.favorites.instrument.as_deref(),
        Some(stopped.id.as_str())
    );
    assert_eq!(
        saved.favorites.effect.as_deref(),
        Some(effect_favorite.id.as_str())
    );
    let mut next = App::prepare();
    next.config_path = Ok(path.clone());
    next.restore = saved.last_played;
    next.restore_effect = saved.effect;
    next.effect_bypassed = saved.effect_bypassed;
    next.sequence_pattern = saved.sequence_pattern;
    next.favorites.restore = saved.favorites;
    next.output = None;
    next.deferred_scan = false;
    next.restore_before_gui();
    settle(&mut next);
    assert!(next.favorites.show);
    assert!(next
        .favorites
        .active
        .contains(&(next.instrument_id().unwrap(), stopped.id.clone())));
    assert!(next
        .favorites
        .active
        .contains(&(next.effect_id().unwrap(), effect_favorite.id.clone())));
    assert!(next.effect_bypassed);
    assert_eq!(
        next.host.save_state(next.instrument_id().unwrap()).unwrap(),
        snapshot
    );
    // The restored effect's lit row still disconnects it on the next click.
    click_favorite(&mut next, &effect_favorite.name, 260.0, true);
    assert!(next.effect_id().is_none());
    drop(next);
    app.save_session();

    let count = app.favorites.library.entries.len();
    let outgoing_state = app.host.save_state(source).unwrap();
    let outgoing_pattern = app.sequence_pattern;
    app.load_plugin(surge);
    settle(&mut app);
    assert_eq!(app.favorites.library.entries.len(), count + 1);
    let automatic = &app.favorites.library.entries[0];
    assert!(automatic.name.starts_with("auto Dexed "));
    assert_eq!(automatic.sequence_pattern, outgoing_pattern);
    assert_eq!(
        app.favorites.library.state(&path, &automatic.id).unwrap(),
        outgoing_state
    );
    let other_source = app.instrument_id().unwrap();
    assert_ne!(other_source, source);
    // Recall must use the favorite, even if the ordinary autosave is unusable.
    crate::state_store::save(&path, &first.plugin, b"invalid autosave").unwrap();
    app.load_favorite(&first.id);
    settle(&mut app);
    assert_ne!(app.instrument_id(), Some(other_source), "{}", app.status);
    assert_eq!(app.sequence_pattern, first.sequence_pattern);
    assert_eq!(
        crate::config::Config::load(&path).unwrap().sequence_pattern,
        first.sequence_pattern
    );
    assert_eq!(app.effect_id(), Some(fx));
    assert!(app.effect_bypassed);
    assert_eq!(
        app.host.save_state(app.instrument_id().unwrap()).unwrap(),
        snapshot
    );
    assert_eq!(app.host.save_state(fx).unwrap(), fx_state);
    assert!(app.favorites.pending.is_none());

    // Recreate the effect slot from its favorite while retaining the instrument.
    let source = app.instrument_id().unwrap();
    assert!(app.remove_plugin(fx));
    let playback = (
        app.sequence_pattern,
        app.selected_sequence,
        app.sequence_velocity,
        app.sequence_modulation,
    );
    app.load_favorite(&effect_favorite.id);
    settle(&mut app);
    assert_eq!(
        (
            app.sequence_pattern,
            app.selected_sequence,
            app.sequence_velocity,
            app.sequence_modulation
        ),
        playback
    );
    assert_eq!(
        crate::config::Config::load(&path).unwrap().sequence_pattern,
        playback.0
    );
    assert_eq!(app.instrument_id(), Some(source));
    assert_eq!(
        app.host.save_state(app.effect_id().unwrap()).unwrap(),
        fx_state
    );
    assert!(app.rename_favorite(&first.id, "My sound"));
    assert_eq!(
        app.favorites.library.state(&path, &first.id).unwrap(),
        snapshot
    );

    // Click the actual rendered favorite label, including after editing its name.
    let ctx = eframe::egui::Context::default();
    app.favorites.show = true;
    let input = || eframe::egui::RawInput {
        screen_rect: Some(eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(900.0, 600.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| app.library_panel(ctx));
    let frame = ctx.run(input(), |ctx| app.library_panel(ctx));
    let position = frame
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            eframe::egui::epaint::Shape::Text(text) if text.galley.job.text == "My sound" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .expect("favorite name was not rendered");
    app.sequence_pattern = crate::SequencePattern::Off;
    app.favorites.active.clear();
    for pressed in [true, false] {
        let mut input = input();
        input.events = vec![
            eframe::egui::Event::PointerMoved(position),
            eframe::egui::Event::PointerButton {
                pos: position,
                button: eframe::egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ];
        let _ = ctx.run(input, |ctx| app.library_panel(ctx));
    }
    assert_eq!(
        app.sequence_pattern, first.sequence_pattern,
        "one click must recall playback pattern"
    );
    assert!(app.favorites.active.iter().any(|(_, id)| id == &first.id));

    // Favorites toggle only the connected effect, even after editing playback
    // and bypassing it. Snapshot recall above remains unchanged for instruments.
    // The instrument click test cleared all last-favorite associations.
    app.load_favorite(&effect_favorite.id);
    app.sequence_pattern = crate::SequencePattern::Off;
    app.effect_bypassed = true;
    let effect = app.effect_id().unwrap();
    let count = app.favorites.library.entries.len();
    click_favorite(&mut app, &effect_favorite.name, 260.0, true);
    assert_eq!(app.effect_id(), None, "{}", app.status);
    assert_eq!(app.instrument_id(), Some(source));
    assert_eq!(app.sequence_pattern, crate::SequencePattern::Off);
    assert!(!app.effect_bypassed);
    assert_eq!(app.favorites.library.entries.len(), count);
    assert_eq!(app.host.save_state(source).unwrap(), snapshot);
    assert_eq!(
        app.favorites
            .library
            .state(&path, &effect_favorite.id)
            .unwrap(),
        fx_state
    );
    assert!(crate::config::Config::load(&path).unwrap().effect.is_none());
    assert!(!app.instances.iter().any(|i| i.id == effect));

    // Once removed, the same row recalls the effect rather than removing the
    // instrument or acting on the stale last-favorite association.
    click_favorite(&mut app, &effect_favorite.name, 260.0, false);
    settle(&mut app);
    assert!(app.effect_id().is_some(), "{}", app.status);
    assert_eq!(app.instrument_id(), Some(source));
    assert_eq!(
        app.host.save_state(app.effect_id().unwrap()).unwrap(),
        fx_state
    );

    // Missing snapshot is reported before replacing either slot.
    let missing = app
        .favorites
        .library
        .entries
        .iter()
        .find(|f| f.id != first.id && !f.effect)
        .unwrap()
        .id
        .clone();
    std::fs::remove_file(dir.join("favorites").join(format!("{missing}.bin"))).unwrap();
    let previous_effect = app.effect_id();
    app.load_favorite(&missing);
    assert!(app.status.contains("Could not load favorite"));
    assert_eq!(app.instrument_id(), Some(source));
    assert_eq!(app.effect_id(), previous_effect);
    let count = app.favorites.library.entries.len();
    app.delete_favorite(&missing);
    assert_eq!(app.favorites.library.entries.len(), count - 1);
    // An effect can remain connected after removing the source in Routing.
    // Its favorite still offers removal without requiring another instrument.
    assert!(app.remove_plugin(source));
    assert!(app.effect_id().is_some());
    click_favorite(&mut app, &effect_favorite.name, 260.0, true);
    assert_eq!(app.effect_id(), None, "{}", app.status);
    assert_eq!(app.instrument_id(), None);
    drop(app);
    let library = Library::load(&path).unwrap();
    assert_eq!(library.entries.len(), count - 1);
    assert_eq!(
        library
            .entries
            .iter()
            .find(|f| f.id == first.id)
            .unwrap()
            .name,
        "My sound"
    );
    assert_eq!(library.state(&path, &first.id).unwrap(), snapshot);
    std::fs::remove_dir_all(dir).unwrap();
}

fn click_favorite(app: &mut App, name: &str, width: f32, connected: bool) {
    let ctx = eframe::egui::Context::default();
    app.filter = name.to_owned();
    let input = || eframe::egui::RawInput {
        screen_rect: Some(eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(width, 600.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| app.library_panel(ctx));
    let frame = ctx.run(input(), |ctx| app.library_panel(ctx));
    let mut position = None;
    let mut kind_visible = false;
    let mut hint_visible = false;
    for shape in &frame.shapes {
        if let eframe::egui::epaint::Shape::Text(text) = &shape.shape {
            let label = text.galley.job.text.as_str();
            if label == name {
                position = Some(text.pos + text.galley.size() / 2.0);
            }
            if label == "Effect" {
                kind_visible = true;
                assert!(shape
                    .clip_rect
                    .contains(text.pos + text.galley.size() / 2.0));
            }
            hint_visible |= label.starts_with("Connected");
        }
    }
    assert!(kind_visible, "type must remain visible in a narrow list");
    assert_eq!(hint_visible, connected);
    let position = position.expect("favorite name was not rendered");
    for pressed in [true, false] {
        let mut input = input();
        input.events = vec![
            eframe::egui::Event::PointerMoved(position),
            eframe::egui::Event::PointerButton {
                pos: position,
                button: eframe::egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ];
        let _ = ctx.run(input, |ctx| app.library_panel(ctx));
    }
    app.filter.clear();
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
fn restored_selection_requires_the_saved_slot_and_plugin_identity() {
    let plugin = crate::config::PluginKey {
        format: "CLAP".into(),
        id: "synth".into(),
        name: "New name".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/synth.clap".into(),
    };
    let instrument = Favorite {
        id: "instrument".into(),
        name: "Renamed favorite".into(),
        plugin: plugin.clone(),
        effect: false,
        sequence_pattern: Default::default(),
    };
    let effect = Favorite {
        id: "effect".into(),
        effect: true,
        ..instrument.clone()
    };
    let mut favorites = Favorites::default();
    favorites.library.entries = vec![instrument, effect];
    favorites.restore = crate::config::FavoriteSelection {
        instrument: Some("instrument".into()),
        effect: Some("effect".into()),
    };
    assert_eq!(
        favorites
            .take_restored(PluginKind::Instrument, &plugin)
            .unwrap()
            .name,
        "Renamed favorite"
    );
    assert_eq!(
        favorites
            .take_restored(PluginKind::Effect, &plugin)
            .unwrap()
            .id,
        "effect"
    );
    assert!(favorites
        .take_restored(PluginKind::Effect, &plugin)
        .is_none());
    for invalid in ["deleted", "effect"] {
        favorites.restore.instrument = Some(invalid.into());
        assert!(favorites
            .take_restored(PluginKind::Instrument, &plugin)
            .is_none());
    }
    for other in [
        crate::config::PluginKey {
            format: "VST3".into(),
            ..plugin.clone()
        },
        crate::config::PluginKey {
            id: "other".into(),
            ..plugin.clone()
        },
    ] {
        favorites.restore.instrument = Some("instrument".into());
        assert!(favorites
            .take_restored(PluginKind::Instrument, &other)
            .is_none());
    }
}
