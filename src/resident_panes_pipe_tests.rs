use super::*;

#[test]
fn random_role_open_waits_for_catalog_and_offers_the_selected_patch_once() {
    if crate::native_library::process::isolated() {
        return;
    }
    use crate::patch_browser_catalog::{tests::candidate, Snapshot};
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut input)) = app("random role open");
    app.plugins = vec![plugin("synth", crate::plugin_list::PluginKind::Instrument)];
    let ctx = egui::Context::default();
    let open = Open::Layer {
        track: 2,
        layer: 1,
        song: 1,
    };
    let context = "song/patch/2";
    app.resident.as_mut().unwrap().pane_cache.insert(
        context.into(),
        (["Chord track".into(), "piano".into()], "piano".into()),
    );
    let (publish, build) = mpsc::channel();
    app.patch_browser.build = Some(build);
    editor
        .send(&ToPlayer::OpenFiltered {
            open,
            context: context.into(),
            panes: ["Chord track".into(), "piano".into()],
            query: "piano".into(),
            cursor: None,
            random_patch_role: Some("bass".into()),
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while shown(&app) != Some(open) {
        assert!(!app.poll_resident());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    frame(&mut app, &ctx, vec![], false);
    assert_eq!(app.patch_browser.saved_filter().role, "Bass track");
    assert!(app.patch_browser.query.is_empty());
    assert!(app.patch_browser.requests.selected.is_none());
    let mut heavy = candidate("synth", "Synth", "Heavy Bass", Some("Bass"));
    heavy.measurement.sfz_sample_bytes = Some(64_000_000);
    let snapshot = Snapshot::build(
        vec![
            heavy,
            candidate("synth", "Synth", "Bass 1", Some("Bass")),
            candidate("synth", "Synth", "Piano", Some("Keys")),
        ],
        &[],
    )
    .unwrap();
    publish.send(Ok((snapshot, None))).unwrap();
    frame(&mut app, &ctx, vec![], false);
    assert_eq!(app.patch_browser.saved_filter().role, "Bass track");
    assert_eq!(app.patch_browser.saved_filter().preset, "ALL");
    assert_eq!(app.patch_browser.results.len(), 2);
    assert_eq!(app.patch_browser.saved_cursor().unwrap().display, "Bass 1");
    let ticket = app.patch_browser.requests.next().unwrap();
    assert_eq!(ticket.candidate.display, "Bass 1");
    app.remember_preparation_target();
    app.patch_browser
        .preparation
        .start_with(ticket.candidate, ctx.clone(), |_| Ok(vec![1, 2, 3]));
    settle(&mut app, &ctx);
    let (sent, state) = selected(&mut input);
    assert_eq!(sent.target, open);
    assert_eq!(sent.preset_name.as_deref(), Some("Bass 1"));
    assert_eq!(state, [1, 2, 3]);
    frame(
        &mut app,
        &ctx,
        super::kit::drum_keys(egui::Key::Enter),
        false,
    );
    assert_eq!(
        shown(&app),
        Some(open),
        "the editor has not applied the patch"
    );
    answer(&mut app, &mut editor, Ok(()));
    frame(&mut app, &ctx, vec![], false);
    assert_eq!(
        app.patch_browser.requests.applied.as_ref().unwrap().display,
        "Bass 1"
    );
    assert!(app.patch_browser.requests.desired.is_none());

    frame(
        &mut app,
        &ctx,
        super::kit::drum_keys(egui::Key::Enter),
        false,
    );
    assert_eq!(shown(&app), None);
    drop(app);
    let mut confirmed = Vec::new();
    while let Some(event) = input.receive::<ToEditor>().unwrap() {
        match event {
            ToEditor::BrowserConfirmed { target } => confirmed.push(target),
            ToEditor::BrowserClosed { .. } => panic!("Enter was reported as a cancellation"),
            _ => {}
        }
    }
    assert_eq!(confirmed, [open]);
}

#[test]
fn closing_reports_pane_changes_without_selecting_any_patch() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut input)) = app("resident pane reporting");
    let open = Open::Patch { track: 1 };
    editor
        .send(&ToPlayer::OpenFiltered {
            open,
            context: "song/patch/1".into(),
            panes: ["Bass track".into(), "Warm".into()],
            query: "bass".into(),
            cursor: None,
            random_patch_role: None,
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while shown(&app) != Some(open) {
        assert!(!app.poll_resident());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    // A played patch belongs to this target even when opening clears its cursor.
    use crate::patch_browser_catalog::{tests::candidate, Snapshot};
    let patches = ["Bass 1", "Bass 2"].map(|name| candidate("a", "Alpha", name, Some("Bass")));
    app.patch_browser.requests.applied = Some(patches[1].clone());
    // No catalog or patch transaction is needed to remember these labels.
    app.patch_browser.restore_preset = Some("Bright".into());
    app.patch_browser.query = "bass -bright".into();
    app.hide_resident();
    assert_eq!(
        input.receive::<ToEditor>().unwrap(),
        Some(ToEditor::Panes {
            context: "song/patch/1".into(),
            panes: ["Bass track".into(), "Bright".into()],
            query: "bass -bright".into()
        })
    );
    assert_eq!(
        input.receive::<ToEditor>().unwrap(),
        Some(ToEditor::BrowserClosed { target: open })
    );
    assert!(app.patch_browser.requests.desired.is_none());
    assert_eq!(shown(&app), None);
    editor
        .send(&ToPlayer::OpenFiltered {
            open,
            context: "song/patch/1".into(),
            panes: ["Bass track".into(), "Warm".into()],
            query: "bass".into(),
            cursor: None,
            random_patch_role: None,
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while shown(&app) != Some(open) {
        assert!(!app.poll_resident());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(app.patch_browser.saved_filter().preset, "Bright");
    assert_eq!(app.patch_browser.query, "bass -bright");
    app.patch_browser.snapshot = Some(Snapshot::build(patches.to_vec(), &[]).unwrap());
    app.patch_browser.resolve_restored_preset();
    app.patch_browser.resolve_restored_playing();
    app.patch_browser.filter(true);
    assert_eq!(
        app.patch_browser.requests.applied.as_ref().unwrap().display,
        "Bass 2"
    );
    assert_eq!(
        app.patch_browser.snapshot.as_ref().unwrap().candidates
            [app.patch_browser.selected.unwrap()]
        .display,
        "Bass 2"
    );
    assert!(app.patch_browser.requests.desired.is_none());
    app.patch_browser.query = "bass plugin:alpha".into();
    app.poll_resident();
    let Some(ToEditor::Panes { query, .. }) = input.receive().unwrap() else {
        panic!("query-only change")
    };
    assert_eq!(query, "bass plugin:alpha");
}
