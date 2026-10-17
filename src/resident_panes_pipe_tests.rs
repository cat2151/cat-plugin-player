use super::*;

#[test]
fn closing_reports_pane_changes_without_selecting_any_patch() {
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
    assert!(app.patch_browser.requests.desired.is_none());
    assert_eq!(shown(&app), None);
    editor
        .send(&ToPlayer::OpenFiltered {
            open,
            context: "song/patch/1".into(),
            panes: ["Bass track".into(), "Warm".into()],
            query: "bass".into(),
            cursor: None,
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
