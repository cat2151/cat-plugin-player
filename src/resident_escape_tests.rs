//! What Esc leaves playing on each path: the resident browsers, the heavy-load
//! confirmation, the standalone patch browser, and the MML editor.
use super::*;

fn sent_patch(app: &mut App, editor_in: &mut Receiver, ctx: &egui::Context, display: &str) {
    let candidate = crate::patch_browser_catalog::tests::candidate("synth", "Synth", display, None);
    app.patch_browser.requests.select(candidate, 0);
    let ticket = app.patch_browser.requests.next().unwrap();
    app.remember_preparation_target();
    app.patch_browser
        .preparation
        .start_with(ticket.candidate, ctx.clone(), |_| Ok(vec![1, 2, 3]));
    settle(app, ctx);
    let (sent, _) = selected(editor_in);
    assert_eq!(sent.preset_name.as_deref(), Some(display));
}

fn applied(app: &App) -> Option<&str> {
    app.patch_browser
        .requests
        .applied
        .as_ref()
        .map(|c| c.display.as_str())
}

/// Polls like the GUI for a while; nothing may travel to the editor meanwhile.
fn nothing_sent(app: &mut App, ctx: &egui::Context) {
    let until = Instant::now() + Duration::from_millis(300);
    while Instant::now() < until {
        assert!(!app.poll_resident(), "link closed early");
        app.poll_browser_patch(ctx);
        app.poll_effect_browser(ctx);
        app.poll_effect_preparation();
        assert!(!app.resident_waiting(), "something was sent after Esc");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn resident_esc_after_an_applied_patch_hides_and_keeps_it() {
    use crate::plugin_list::PluginKind;
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut editor_in)) = app("escape keeps patch");
    app.plugins = vec![plugin("synth", PluginKind::Instrument)];
    let ctx = egui::Context::default();
    request(&mut app, &mut editor, Open::Patch { track: 1 });
    sent_patch(&mut app, &mut editor_in, &ctx, "Bass 1");
    answer(&mut app, &mut editor, Ok(()));
    sent_patch(&mut app, &mut editor_in, &ctx, "Bass 2");
    answer(&mut app, &mut editor, Ok(()));
    assert_eq!(applied(&app), Some("Bass 2"));

    frame(&mut app, &ctx, Vec::new(), false);
    frame(&mut app, &ctx, escape(), false);
    assert_eq!(shown(&app), None, "Esc hides the resident browser");
    assert!(!app.patch_browser.open);
    let requests = &app.patch_browser.requests;
    assert!(requests.selected.is_none() && requests.desired.is_none());
    assert_eq!(
        applied(&app),
        Some("Bass 2"),
        "the last applied patch stays"
    );
    // The only player-to-editor message is a selection, so nothing can undo Bass 2.
    nothing_sent(&mut app, &ctx);

    // Showing it again requests nothing either.
    request(&mut app, &mut editor, Open::Patch { track: 1 });
    nothing_sent(&mut app, &ctx);
    assert_eq!(applied(&app), Some("Bass 2"));
}

/// Esc while the editor is applying a patch: its answer still lands, so the
/// patch the editor already plays is the one kept.
#[test]
fn resident_esc_while_the_editor_applies_keeps_that_patch() {
    use crate::plugin_list::PluginKind;
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut editor_in)) = app("escape while applying");
    app.plugins = vec![plugin("synth", PluginKind::Instrument)];
    let ctx = egui::Context::default();
    request(&mut app, &mut editor, Open::Patch { track: 1 });
    sent_patch(&mut app, &mut editor_in, &ctx, "Bass 1");
    answer(&mut app, &mut editor, Ok(()));
    sent_patch(&mut app, &mut editor_in, &ctx, "Bass 2");

    frame(&mut app, &ctx, escape(), false);
    assert_eq!(shown(&app), None);
    assert!(app.resident_waiting(), "Bass 2 is still with the editor");
    answer(&mut app, &mut editor, Ok(()));
    assert_eq!(applied(&app), Some("Bass 2"));
    nothing_sent(&mut app, &ctx);
}

/// Esc while a patch is still being prepared: it never reaches the editor, so
/// the patch playing before stays.
#[test]
fn resident_esc_while_preparing_drops_that_patch() {
    use crate::plugin_list::PluginKind;
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut editor_in)) = app("escape while preparing");
    app.plugins = vec![plugin("synth", PluginKind::Instrument)];
    let ctx = egui::Context::default();
    request(&mut app, &mut editor, Open::Patch { track: 1 });
    sent_patch(&mut app, &mut editor_in, &ctx, "Bass 1");
    answer(&mut app, &mut editor, Ok(()));
    let candidate =
        crate::patch_browser_catalog::tests::candidate("synth", "Synth", "Bass 2", None);
    app.patch_browser.requests.select(candidate, 0);
    let ticket = app.patch_browser.requests.next().unwrap();
    app.remember_preparation_target();
    app.patch_browser
        .preparation
        .start_with(ticket.candidate, ctx.clone(), |_| {
            std::thread::sleep(Duration::from_millis(100));
            Ok(vec![1, 2, 3])
        });

    frame(&mut app, &ctx, escape(), false);
    assert_eq!(shown(&app), None);
    assert!(app.patch_browser.preparation.busy(), "Esc came first");
    nothing_sent(&mut app, &ctx);
    assert!(!app.patch_browser.preparation.busy());
    assert_eq!(applied(&app), Some("Bass 1"));
}

#[test]
fn resident_esc_in_the_heavy_confirmation_returns_to_the_browser() {
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, _editor_in)) = app("escape heavy confirmation");
    let ctx = egui::Context::default();
    request(&mut app, &mut editor, Open::Patch { track: 1 });
    let mut heavy = crate::patch_browser_catalog::tests::candidate("b", "Beta", "Heavy", None);
    heavy.measurement.sfz_sample_bytes = Some(64_000_000);
    app.patch_browser.requests.select(heavy, 0);
    app.patch_browser.requests.space();
    assert!(app.patch_browser.requests.confirmation.is_some());
    frame(&mut app, &ctx, Vec::new(), false);
    assert_eq!(shown(&app), Some(Open::Patch { track: 1 }));

    frame(&mut app, &ctx, escape(), false);
    assert!(app.patch_browser.requests.confirmation.is_none());
    assert!(
        app.patch_browser.requests.desired.is_none(),
        "Heavy is not loaded"
    );
    assert_eq!(shown(&app), Some(Open::Patch { track: 1 }));
    assert!(app.patch_browser.open, "only the dialog closes");

    frame(&mut app, &ctx, escape(), false);
    assert_eq!(shown(&app), None, "the next Esc hides the browser");
}

#[test]
fn resident_esc_after_an_applied_effect_hides_and_sends_nothing() {
    use crate::plugin_list::PluginKind;
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut editor_in)) = app("escape keeps effect");
    app.plugins = vec![plugin("id.Surge", PluginKind::Effect)];
    let ctx = egui::Context::default();
    let open = Open::Effect {
        track: 1,
        slot: None,
    };
    request(&mut app, &mut editor, open);
    let mut effect =
        crate::effect_browser::tests::candidate("Surge", "Big Hall", "Space", "Reverb");
    effect.state = Some(vec![9; 10].into());
    app.effect_browser.desired = Some(effect);
    settle(&mut app, &ctx);
    let (sent, _) = selected(&mut editor_in);
    assert_eq!(sent.target, open);
    answer(&mut app, &mut editor, Ok(()));

    frame(&mut app, &ctx, Vec::new(), false);
    frame(&mut app, &ctx, escape(), false);
    assert_eq!(shown(&app), None);
    assert!(!app.effect_browser.open && app.effect_browser.desired.is_none());
    nothing_sent(&mut app, &ctx);
}

/// Neither resident nor selecting: Esc only closes the browser window.
fn standalone(test: &str) -> App {
    let mut app = App::prepare_mode(true);
    app.config_path = Ok(crate::selection_tests::directory(test).join("config.toml"));
    app.deferred_scan = false;
    app.patch_browser.open_window();
    app
}

#[test]
fn standalone_esc_closes_the_browser_and_keeps_the_applied_patch() {
    let _library = crate::native_library::load().unwrap();
    let mut app = standalone("escape standalone");
    let playing = crate::patch_browser_catalog::tests::candidate("a", "Alpha", "Bass", None);
    app.patch_browser.requests.applied = Some(playing);
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, Vec::new(), false);
    assert!(app.patch_browser.open);
    frame(&mut app, &ctx, escape(), false);
    assert!(!app.patch_browser.open, "Esc closes the browser");
    assert!(app.selection.is_none() && app.resident.is_none());
    assert_eq!(applied(&app), Some("Bass"));
}

#[test]
fn standalone_esc_in_the_heavy_confirmation_closes_only_the_dialog() {
    let _library = crate::native_library::load().unwrap();
    let mut app = standalone("escape standalone heavy");
    let mut heavy = crate::patch_browser_catalog::tests::candidate("b", "Beta", "Heavy", None);
    heavy.measurement.sfz_sample_bytes = Some(64_000_000);
    app.patch_browser.requests.select(heavy, 0);
    app.patch_browser.requests.space();
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, Vec::new(), false);
    frame(&mut app, &ctx, escape(), false);
    assert!(app.patch_browser.requests.confirmation.is_none());
    assert!(app.patch_browser.requests.desired.is_none());
    assert!(app.patch_browser.open, "the browser stays open");
}

#[test]
fn mml_esc_drops_the_typed_text_and_restores_the_pattern() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare_mode(true);
    app.config_path = Ok(crate::selection_tests::directory("escape mml").join("config.toml"));
    app.deferred_scan = false;
    app.sequence_pattern = crate::SequencePattern::Steps;
    app.mml_input.confirmed = "cde".into();
    app.open_mml_editor();
    assert!(app.sequence_pattern == crate::SequencePattern::Off);
    app.mml_input.buffer = "gab".into();
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        events: escape(),
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| app.mml_editor(ctx));
    assert!(!app.mml_input.open);
    assert!(app.sequence_pattern == crate::SequencePattern::Steps);
    assert_eq!(app.mml_input.confirmed, "cde");
}
