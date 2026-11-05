use super::*;

#[test]
fn native_cross_screen_candidates_navigation_and_selection_do_not_exit_or_duplicate() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.restoring = false;
    app.deferred_scan = false;
    let ctx = egui::Context::default();
    app.patch_browser.open = true;
    let list = commands(&app, Screen::Patch);
    assert!(list
        .iter()
        .any(|c| c.id == "next-velocity" && c.screen == Screen::Main));
    assert_eq!(list.iter().filter(|c| c.id == "keyboard-help").count(), 1);
    assert!(
        list.iter()
            .find(|c| c.id == "keyboard-help")
            .unwrap()
            .screen
            == Screen::Patch
    );
    assert!(!list.iter().any(|c| c.id == "close-effect-browser"));
    assert!(!list.iter().any(|c| !c.enabled));

    // A child request is moved to the root, then uses the existing control handler.
    let source_id = viewport_id(Screen::Patch.viewport());
    ctx.data_mut(|d| {
        d.insert_temp(
            source_id,
            State {
                pending: Some("Next velocity"),
                pending_screen: Screen::Main,
                pending_query: "velocity".into(),
                pending_target: Some(Target::of(&app, Screen::Main)),
                ..Default::default()
            },
        )
    });
    prepare(&mut app, &ctx);
    assert!(!app.patch_browser.open);
    let before = app.sequence_velocity;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        let input = super::super::begin(ctx, commands(&app, Screen::Main), true);
        egui::CentralPanel::default().show(ctx, |ui| app.sequence_controls(ui));
        super::super::end(ctx, input);
    });
    assert_eq!(app.sequence_velocity, before.adjacent(true));
    prepare(&mut app, &ctx);
    assert!(ctx
        .data(|d| d.get_temp::<State>(viewport_id(Screen::Main.viewport())))
        .unwrap()
        .pending
        .is_none());
    let h = history::load(&ctx);
    assert_eq!(h.rank("velocity", "Main/next-velocity"), (0, 0));

    // A main input in progress blocks navigation from the child as well.
    app.patch_browser.open = true;
    app.mml_input.open = true;
    assert!(commands(&app, Screen::Patch).is_empty());
    app.mml_input.open = false;
    app.patch_browser.solo_draft = Some("unconfirmed conditions".into());
    for screen in [Screen::Main, Screen::Patch, Screen::Effect] {
        assert!(commands(&app, screen).is_empty());
    }
    assert_eq!(
        app.patch_browser.solo_draft.as_deref(),
        Some("unconfirmed conditions")
    );
    app.patch_browser.solo_draft = None;

    // The existing selection lifecycle must not mistake a palette transition for exit.
    let root =
        std::path::PathBuf::from(std::env::var_os(crate::native_library::process::ROOT).unwrap());
    let music = root.join("input.mid");
    crate::selection_tests::input(&music, &Default::default());
    let previous_target = Target::of(&app, Screen::Main);
    app.configure_selection(music, crate::cli::Browser::Patch)
        .unwrap();
    app.configure_browser_only(crate::cli::Browser::Patch);
    navigate(&mut app, &ctx, Screen::Main);
    let selection = app.selection.as_ref().unwrap();
    assert!(selection.main_visible);
    assert!(selection.browser_only.is_none());
    assert!(!selection.exit_requested);
    assert!(!app.patch_browser.open);
    navigate(&mut app, &ctx, Screen::Effect);
    assert!(app.effect_browser.open);
    assert!(!app.selection.as_ref().unwrap().exit_requested);
    app.selection_browser_closed(crate::cli::Browser::Patch);
    assert!(!app.selection.as_ref().unwrap().exit_requested);

    // The operation still exists, but now belongs to a different SMF target.
    ctx.data_mut(|d| {
        d.insert_temp(
            viewport_id(Screen::Main.viewport()),
            State {
                pending: Some("Next library tab"),
                pending_target: Some(previous_target),
                ..Default::default()
            },
        )
    });
    prepare(&mut app, &ctx);
    let current = ctx
        .data(|d| d.get_temp::<State>(viewport_id(Screen::Main.viewport())))
        .unwrap();
    assert!(current.open);
    assert!(current.pending.is_none());
    assert_eq!(
        history::load(&ctx).rank("", "Main/next-library-tab"),
        (2, 0)
    );
}

#[test]
fn native_drum_palette_targets_and_entry_follow_existing_navigation() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    let ctx = egui::Context::default();
    assert!(commands(&app, Screen::Main)
        .iter()
        .any(|c| c.id == "browse-drums"));
    navigate(&mut app, &ctx, Screen::Drum);
    assert!(app.drum_browser.open);
    assert!(!app.patch_browser.open && !app.effect_browser.open);
    assert_eq!(app.sequence_pattern, crate::SequencePattern::Off);
    let list = commands(&app, Screen::Drum);
    assert_eq!(list.iter().filter(|c| c.id == "keyboard-help").count(), 1);
    assert!(!list
        .iter()
        .any(|c| c.id == "close-patch-browser" || c.id == "close-effect-browser"));
    app.drum_browser.editing = Some(String::new());
    assert!(commands(&app, Screen::Drum).is_empty());
    app.drum_browser.editing = None;
    navigate(&mut app, &ctx, Screen::Patch);
    assert!(app.patch_browser.open && !app.drum_browser.open);
}

#[test]
fn native_drum_main_palette_hides_melodic_controls_and_keeps_random_patch() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    app.patch_browser.requests.applied = Some(crate::drum_browser::tests::candidate("kit"));
    let commands = commands(&app, Screen::Main);
    assert!(!commands
        .iter()
        .any(|c| c.id == "play-stop-sequence" || c.id == "next-sequence"));
    assert!(commands.iter().any(|c| c.id == "random-patch"));
    app.patch_browser
        .requests
        .applied
        .as_mut()
        .unwrap()
        .measurement
        .drum_kit = false;
    assert!(super::commands(&app, Screen::Main)
        .iter()
        .any(|c| c.id == "next-sequence"));
}
