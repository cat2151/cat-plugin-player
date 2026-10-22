use super::*;

fn key(key: egui::Key, repeat: bool, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers,
    }
}

#[test]
fn favorite_ignores_repeats_modifiers_and_disabled_actions() {
    for (repeat, modifiers, enabled, expected) in [
        (false, egui::Modifiers::NONE, true, true),
        (true, egui::Modifiers::NONE, true, false),
        (false, egui::Modifiers::CTRL, true, false),
        (false, egui::Modifiers::SHIFT, true, false),
        (false, egui::Modifiers::NONE, false, false),
    ] {
        let ctx = egui::Context::default();
        if repeat {
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![key(egui::Key::F, false, modifiers)],
                    ..Default::default()
                },
                |_| {},
            );
        }
        let _ = ctx.run(
            egui::RawInput {
                events: vec![key(egui::Key::F, repeat, modifiers)],
                ..Default::default()
            },
            |ctx| {
                assert_eq!(take_favorite_shortcut(ctx, enabled), expected);
                assert!(!take_favorite_shortcut(ctx, enabled));
            },
        );
    }
}

#[test]
#[ignore = "creates native host/audio device; no installed plugins required"]
fn native_main_slash_focuses_filter_without_inserting_slash() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect = Default::default();
    app.deferred_scan = false;
    let ctx = egui::Context::default();
    let input = |events| egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(900.0, 700.0),
        )),
        events,
        ..Default::default()
    };
    let _ = ctx.run(input(vec![]), |ctx| app.library_panel(ctx));
    let _ = ctx.run(
        input(vec![
            key(egui::Key::Slash, false, egui::Modifiers::NONE),
            egui::Event::Text("/".into()),
        ]),
        |ctx| app.library_panel(ctx),
    );
    assert_eq!(
        ctx.memory(|memory| memory.focused()),
        Some(egui::Id::new("library_filter"))
    );
    assert!(app.filter.is_empty());
    let _ = ctx.run(
        input(vec![
            key(egui::Key::F, false, egui::Modifiers::NONE),
            egui::Event::Text("f".into()),
        ]),
        |ctx| app.library_panel(ctx),
    );
    assert_eq!(app.filter, "f");
    assert!(app.favorites.library.entries.is_empty());
}

#[test]
#[ignore = "creates native host/audio device; no installed plugins required"]
fn native_main_letters_cycle_controls_and_toggle_library_and_analysis() {
    let _library = crate::native_library::load().unwrap();
    let root = std::env::temp_dir().join(format!("cat-main-letters-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let config = root.join("config.toml");
    let mut app = App::prepare();
    app.config_path = Ok(config.clone());
    app.output = None;
    app.restore = None;
    app.restore_effect = Default::default();
    app.deferred_scan = false;
    app.initialize_favorites();
    let ctx = egui::Context::default();
    let press = |app: &mut App, letter| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events: vec![
                key(letter, false, egui::Modifiers::NONE),
                egui::Event::Key {
                    key: letter,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::TopBottomPanel::top("sequence_controls").show(ctx, |ui| {
                app.playback_controls(ui);
            });
            let on_right = app.scope_ui.on_right;
            app.scope_panel(ctx, on_right);
            app.routing_panel(ctx, on_right);
            app.library_panel(ctx);
        });
    };
    let sequence = app.selected_sequence;
    let velocity = app.sequence_velocity;
    let modulation = app.sequence_modulation;
    press(&mut app, egui::Key::S);
    press(&mut app, egui::Key::V);
    press(&mut app, egui::Key::W);
    assert_eq!(app.selected_sequence, sequence.adjacent(true));
    assert_eq!(app.sequence_velocity, velocity.adjacent(true));
    assert_eq!(
        app.sequence_modulation,
        crate::SequenceModulation::adjacent(modulation, true)
    );

    let tab = |app: &App| (app.favorites.show, app.favorites.show_history);
    app.favorites.show = true;
    app.favorites.show_history = false;
    for expected in [(true, true), (false, false), (true, false)] {
        press(&mut app, egui::Key::L);
        assert_eq!(tab(&app), expected);
    }
    let saved = crate::status::Status::load(&config).unwrap();
    assert_eq!(saved.show_favorites, Some(true));
    press(&mut app, egui::Key::O);
    assert!(app.favorites.reorder_mode);
    let prefer_clap = app.prefer_clap;
    press(&mut app, egui::Key::D);
    assert_eq!(app.prefer_clap, !prefer_clap);

    let on_right = app.scope_ui.on_right;
    press(&mut app, egui::Key::P);
    assert_eq!(app.scope_ui.on_right, !on_right);

    press(&mut app, egui::Key::R);
    assert_ne!(
        app.patch_browser.main_random,
        crate::patch_browser::MainRandom::Idle
    );
    assert!(!app.patch_browser.open);
    let _ = std::fs::remove_dir_all(&root);
}
