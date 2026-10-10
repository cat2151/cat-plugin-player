//! Editor keys and transient playback through the native fixture.
use super::*;

#[test]
#[ignore = "requires native shim and S06_NATIVE_WORK_DIR; no user-data writes"]
fn native_mml_editor_keys_and_patch_transport() {
    use eframe::egui;
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    let mut app = app(&root.join("mml-editor/config.toml"));
    // App::prepare reads the live session before the fixture redirects its paths.
    app.mml_input = Default::default();
    let index = add(&mut app, &key("normal"));
    app.load_plugin(index);
    wait(&mut app);
    app.sequence_pattern = SequencePattern::Steps;
    let ctx = egui::Context::default();
    let key_event = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let frame = |app: &mut App, events: Vec<egui::Event>| {
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| app.mml_editor(ctx),
        );
    };
    frame(
        &mut app,
        vec![key_event(egui::Key::I), egui::Event::Text("i".into())],
    );
    assert!(app.mml_input.open);
    assert_eq!(app.sequence_pattern, SequencePattern::Off);
    assert_eq!(app.selected_sequence, SequencePattern::Steps);
    // Opening twice must not replace the remembered playing state with Off.
    app.open_mml_editor();
    assert_eq!(
        crate::status::Status::load(&app.config_path.clone().unwrap())
            .unwrap()
            .sequence_pattern,
        SequencePattern::Off
    );
    assert!(app.mml_input.buffer.is_empty());
    // The text editor must receive Space and i as text, without transport actions.
    frame(
        &mut app,
        vec![key_event(egui::Key::Space), egui::Event::Text(" i".into())],
    );
    assert_eq!(app.mml_input.buffer, " i");
    assert_eq!(app.sequence_pattern, SequencePattern::Off);
    app.mml_input.buffer = "t120 crd".into();
    frame(&mut app, vec![key_event(egui::Key::Enter)]);
    assert!(!app.mml_input.open);
    assert_eq!(app.sequence_pattern, SequencePattern::Custom);
    assert_eq!(app.selected_sequence, SequencePattern::Custom);
    assert_eq!(
        crate::status::Status::load(&app.config_path.clone().unwrap())
            .unwrap()
            .sequence_pattern,
        SequencePattern::Custom
    );
    let phrase = app.mml_input.phrase.clone().unwrap();
    app.open_mml_editor();
    app.mml_input.buffer = " ".into();
    frame(&mut app, vec![key_event(egui::Key::Enter)]);
    assert!(app.mml_input.open && app.mml_input.error.is_some());
    assert_eq!(app.sequence_pattern, SequencePattern::Off);
    assert!(std::sync::Arc::ptr_eq(
        app.mml_input.phrase.as_ref().unwrap(),
        &phrase
    ));
    frame(&mut app, vec![key_event(egui::Key::Escape)]);
    assert!(!app.mml_input.open);
    assert_eq!(app.sequence_pattern, SequencePattern::Custom);
    app.sequence_pattern = SequencePattern::Custom;
    app.open_mml_editor();
    assert_eq!(app.sequence_pattern, SequencePattern::Off);
    app.mml_input.buffer = "C F G".into();
    frame(&mut app, vec![key_event(egui::Key::Enter)]);
    assert_eq!(app.sequence_pattern, SequencePattern::Custom);
    let phrase = app.mml_input.phrase.clone().unwrap();
    // Cancel restores a built-in pattern, while stopped edits stay stopped.
    for state in [SequencePattern::Steps, SequencePattern::Off] {
        app.sequence_pattern = state;
        app.selected_sequence = SequencePattern::Steps;
        app.open_mml_editor();
        app.mml_input.buffer = "df a".into();
        frame(&mut app, vec![key_event(egui::Key::Escape)]);
        assert_eq!(app.sequence_pattern, state);
        assert_eq!(app.selected_sequence, SequencePattern::Steps);
        assert!(std::sync::Arc::ptr_eq(
            app.mml_input.phrase.as_ref().unwrap(),
            &phrase
        ));
        assert_eq!(
            crate::status::Status::load(&app.config_path.clone().unwrap())
                .unwrap()
                .sequence_pattern,
            state
        );
    }
    app.open_mml_editor();
    app.mml_input.buffer = "C F G".into();
    frame(&mut app, vec![key_event(egui::Key::Enter)]);
    assert!(!app.mml_input.open);
    assert_eq!(app.sequence_pattern, SequencePattern::Off);
    assert_eq!(app.selected_sequence, SequencePattern::Custom);
    let phrase = app.mml_input.phrase.clone().unwrap();
    for state in [SequencePattern::Off, SequencePattern::Custom] {
        app.sequence_pattern = state;
        let mut patch = prepared(&key("normal"), &[5]);
        app.random_failures = vec![Operation::Apply];
        app.apply_random_patch(patch);
        assert_eq!(app.sequence_pattern, state);
        app.random_failures.clear();
        patch = prepared(&key("normal"), &[5]);
        app.apply_random_patch(patch);
        assert!(app.status.contains("Random patch:"), "{}", app.status);
        assert_eq!(app.sequence_pattern, state);
        assert!(std::sync::Arc::ptr_eq(
            app.mml_input.phrase.as_ref().unwrap(),
            &phrase
        ));
    }
}
