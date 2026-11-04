use super::*;

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn commands() -> Vec<Command> {
    vec![
        Command::new(
            "Random patch",
            "ランダムに音色を選択",
            "音色 ランダム",
            "R",
            &[Key::R],
            true,
        ),
        Command::new(
            "Next velocity",
            "次のベロシティへ",
            "ベロシティ",
            "V",
            &[Key::V],
            true,
        ),
        Command::new(
            "Disabled action",
            "無効な操作",
            "無効",
            "S",
            &[Key::S],
            false,
        ),
    ]
}

fn frame(ctx: &egui::Context, events: Vec<Event>, allowed: bool) -> Vec<Event> {
    let mut background = Vec::new();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            let input = begin(ctx, commands(), allowed);
            background = ctx.input(|i| i.events.clone());
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Background");
            });
            end(ctx, input);
        },
    );
    background
}

fn open(ctx: &egui::Context) {
    frame(
        ctx,
        vec![key(Key::P, Modifiers::CTRL | Modifiers::SHIFT)],
        true,
    );
    frame(ctx, vec![], true);
    assert!(state(ctx).open);
}

#[test]
fn search_accepts_multiple_words_case_and_japanese() {
    let commands = commands();
    let mut query = search::SearchQuery::default();
    query.update("PATCH random");
    assert!(commands[0].matches(&query));
    query.update("音色 ランダム");
    assert!(commands[0].matches(&query));
    query.update("velocity");
    assert!(!commands[0].matches(&query));
    query.update("  ");
    assert!(commands.iter().all(|c| c.matches(&query)));
}

#[test]
fn search_and_escape_are_isolated_and_restore_focus() {
    let ctx = egui::Context::default();
    let original = egui::Id::new("original-filter");
    ctx.memory_mut(|m| m.request_focus(original));
    open(&ctx);
    assert!(frame(
        &ctx,
        vec![
            Event::Text("random".into()),
            key(Key::Space, Modifiers::NONE)
        ],
        true
    )
    .is_empty());
    assert_eq!(state(&ctx).query, "random");
    assert!(frame(&ctx, vec![key(Key::Escape, Modifiers::NONE)], true).is_empty());
    assert!(!state(&ctx).open);
    assert_eq!(ctx.memory(|m| m.focused()), Some(original));
    assert!(frame(&ctx, vec![], true).is_empty());
}

#[test]
fn filtered_enter_dispatches_once_on_next_frame_without_enter_leaking() {
    let ctx = egui::Context::default();
    open(&ctx);
    frame(&ctx, vec![Event::Text("velocity".into())], true);
    assert!(frame(&ctx, vec![key(Key::Enter, Modifiers::NONE)], true).is_empty());
    assert!(!state(&ctx).open);
    assert_eq!(state(&ctx).pending, Some("Next velocity"));
    let events = frame(&ctx, vec![], true);
    assert!(matches!(
        events.as_slice(),
        [Event::Key {
            key: Key::V,
            pressed: true,
            ..
        }]
    ));
    assert!(frame(&ctx, vec![], true).is_empty());
}

#[test]
fn arrows_disabled_and_empty_results_do_not_execute() {
    let ctx = egui::Context::default();
    open(&ctx);
    frame(&ctx, vec![key(Key::ArrowDown, Modifiers::NONE)], true);
    assert_eq!(state(&ctx).selected, 1);
    frame(&ctx, vec![key(Key::ArrowDown, Modifiers::NONE)], true);
    frame(&ctx, vec![key(Key::Enter, Modifiers::NONE)], true);
    assert!(state(&ctx).open);
    assert_eq!(state(&ctx).pending, None);
    frame(&ctx, vec![Event::Text("no match".into())], true);
    frame(&ctx, vec![key(Key::Enter, Modifiers::NONE)], true);
    assert!(state(&ctx).open);
}

#[test]
fn page_and_ctrl_home_end_navigate_without_changing_the_query() {
    let ctx = egui::Context::default();
    open(&ctx);
    for (key_code, modifiers, selected) in [
        (Key::PageDown, Modifiers::NONE, 2),
        (Key::PageUp, Modifiers::NONE, 0),
        (Key::End, Modifiers::CTRL, 2),
        (Key::Home, Modifiers::CTRL, 0),
    ] {
        assert!(frame(&ctx, vec![key(key_code, modifiers)], true).is_empty());
        assert_eq!(state(&ctx).selected, selected);
        assert!(state(&ctx).query.is_empty());
        assert!(state(&ctx).open);
    }
}

#[test]
fn ime_enter_and_same_frame_text_do_not_run_a_command() {
    let ctx = egui::Context::default();
    open(&ctx);
    frame(
        &ctx,
        vec![
            Event::Ime(egui::ImeEvent::Preedit("音".into())),
            key(Key::Enter, Modifiers::NONE),
        ],
        true,
    );
    assert!(state(&ctx).open);
    frame(
        &ctx,
        vec![
            Event::Ime(egui::ImeEvent::Commit("音色".into())),
            key(Key::Enter, Modifiers::NONE),
        ],
        true,
    );
    assert!(state(&ctx).open);
    assert_eq!(state(&ctx).pending, None);
    frame(&ctx, vec![key(Key::Escape, Modifiers::NONE)], true);
    open(&ctx);
    frame(
        &ctx,
        vec![
            Event::Text("velocity".into()),
            key(Key::Enter, Modifiers::NONE),
        ],
        true,
    );
    assert!(state(&ctx).open);
    assert_eq!(state(&ctx).query, "velocity");
}

#[test]
fn modal_owner_blocks_opening_and_invalidates_pending_actions() {
    let ctx = egui::Context::default();
    frame(
        &ctx,
        vec![key(Key::P, Modifiers::CTRL | Modifiers::SHIFT)],
        false,
    );
    assert!(!state(&ctx).open);
    let mut release = key(Key::P, Modifiers::CTRL | Modifiers::SHIFT);
    if let Event::Key { pressed, .. } = &mut release {
        *pressed = false;
    }
    frame(&ctx, vec![release], false);
    open(&ctx);
    frame(&ctx, vec![key(Key::Enter, Modifiers::NONE)], true);
    assert!(state(&ctx).pending.is_some());
    assert!(frame(&ctx, vec![], false).is_empty());
    assert!(state(&ctx).pending.is_none());
}

#[test]
fn dispatch_revalidates_current_command_availability() {
    let ctx = egui::Context::default();
    save(
        &ctx,
        State {
            pending: Some("Disabled action"),
            ..Default::default()
        },
    );
    assert!(frame(&ctx, vec![], true).is_empty());
    assert!(state(&ctx).pending.is_none());
}

#[test]
fn menu_commands_use_the_existing_prefix_dispatcher() {
    for (keys, menu, action) in [
        (
            vec![Key::D, Key::O],
            crate::main_menu::Menu::Settings,
            Key::O,
        ),
        (
            vec![Key::D, Key::D],
            crate::main_menu::Menu::Settings,
            Key::D,
        ),
        (
            vec![Key::A, Key::A],
            crate::main_menu::Menu::Analysis,
            Key::A,
        ),
    ] {
        let ctx = egui::Context::default();
        save(
            &ctx,
            State {
                pending: Some("Menu command"),
                ..Default::default()
            },
        );
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let commands = vec![Command::new(
                "Menu command",
                "メニュー操作",
                "",
                "",
                &keys,
                true,
            )];
            assert!(begin(ctx, commands, true).is_none());
            crate::main_menu::keyboard(ctx, true);
            assert!(menu.take(ctx, action));
            assert!(!menu.take(ctx, action));
            assert!(!crate::selection::exit_key(ctx));
        });
    }
}

#[test]
fn extra_layout_pass_does_not_dispatch_early_or_replay_enter() {
    let ctx = egui::Context::default();
    ctx.options_mut(|options| options.max_passes = 2.try_into().unwrap());
    open(&ctx);
    frame(&ctx, vec![Event::Text("velocity".into())], true);
    let mut passes = 0;
    let _ = ctx.run(
        egui::RawInput {
            events: vec![key(Key::Enter, Modifiers::NONE)],
            ..Default::default()
        },
        |ctx| {
            let input = begin(ctx, commands(), true);
            assert!(ctx.input(|i| i.events.is_empty()));
            egui::CentralPanel::default().show(ctx, |_| {});
            end(ctx, input);
            passes += 1;
            if passes == 1 {
                ctx.request_discard("palette regression");
            }
        },
    );
    assert_eq!(passes, 2);
    assert_eq!(state(&ctx).pending, Some("Next velocity"));
    assert!(matches!(
        frame(&ctx, vec![], true).as_slice(),
        [Event::Key { key: Key::V, .. }]
    ));
    assert!(frame(&ctx, vec![], true).is_empty());
}

#[test]
fn pointer_and_scroll_are_suspended_while_palette_owns_input() {
    let ctx = egui::Context::default();
    open(&ctx);
    let _ = ctx.run(
        egui::RawInput {
            events: vec![Event::PointerButton {
                pos: egui::pos2(10.0, 10.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| {
            let input = begin(ctx, commands(), true);
            assert!(!ctx.input(|i| i.pointer.any_pressed()));
            assert!(ctx.input(|i| i.events.is_empty()));
            end(ctx, input);
        },
    );
}

#[test]
fn native_main_palette_reuses_controls_menus_and_mml_without_touching_user_settings() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let root =
        std::path::PathBuf::from(std::env::var_os(crate::native_library::process::ROOT).unwrap());
    let mut app = crate::App::prepare();
    app.config_path = Ok(root.join("config.toml"));
    std::fs::write(root.join("config.toml"), "# Keep user settings unchanged\n").unwrap();
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    app.restoring = false;
    app.sequence_pattern = crate::SequencePattern::Off;
    app.initialize_favorites();
    let ctx = egui::Context::default();
    let draw = |app: &mut crate::App, events| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                let input = begin(ctx, main_commands(app), !app.mml_input.open);
                crate::main_menu::keyboard(ctx, true);
                app.mml_editor(ctx);
                egui::TopBottomPanel::top("controls").show(ctx, |ui| app.playback_controls(ui));
                app.scope_panel(ctx, false);
                app.library_panel(ctx);
                end(ctx, input);
            },
        );
    };
    let run = |app: &mut crate::App, query: &str| {
        draw(app, vec![key(Key::P, Modifiers::CTRL | Modifiers::SHIFT)]);
        draw(app, vec![]);
        draw(app, vec![Event::Text(query.into())]);
        draw(app, vec![key(Key::Enter, Modifiers::NONE)]);
        draw(app, vec![]);
    };
    draw(&mut app, vec![]);
    let velocity = app.sequence_velocity;
    run(&mut app, "next velocity");
    assert_eq!(app.sequence_velocity, velocity.adjacent(true));
    let on_right = app.scope_ui.on_right;
    run(&mut app, "move analysis");
    assert_ne!(app.scope_ui.on_right, on_right);
    run(&mut app, "filter library");
    assert_eq!(
        ctx.memory(|m| m.focused()),
        Some(egui::Id::new("library_filter"))
    );
    assert!(app.filter.is_empty());
    run(&mut app, "edit mml");
    assert!(app.mml_input.open);
    assert_eq!(app.mml_input.buffer, app.mml_input.confirmed);
    assert_eq!(
        std::fs::read_to_string(root.join("config.toml")).unwrap(),
        "# Keep user settings unchanged\n"
    );
    let saved = crate::status::Status::load(&root.join("config.toml")).unwrap();
    assert_eq!(saved.sequence_velocity, app.sequence_velocity);
    assert_eq!(saved.on_right, app.scope_ui.on_right);
}
