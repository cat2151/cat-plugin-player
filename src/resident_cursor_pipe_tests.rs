use super::*;
use crate::{
    patch_browser_catalog::{tests::candidate, Snapshot},
    status::PatchIdentity,
};

#[test]
fn named_cursor_waits_for_catalog_then_cursor_only_changes_are_reported() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut input)) = app("named cursor");
    let patch = PatchIdentity {
        format: "CLAP".into(),
        plugin_id: "a".into(),
        path: "X:/patches/a/P3".into(),
        display: "P3".into(),
    };
    let open = Open::Patch { track: 1 };
    editor
        .send(&ToPlayer::OpenFiltered {
            open,
            context: "song/patch/1".into(),
            panes: ["ALL".into(), "ALL".into()],
            query: String::new(),
            cursor: Some(patch.clone()),
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while shown(&app) != Some(open) {
        assert!(!app.poll_resident());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(app.patch_browser.saved_cursor(), Some(patch));
    app.poll_resident(); // Waiting must not erase the saved name.
    app.patch_browser.snapshot = Some(
        Snapshot::build(
            ["P0", "P1", "P2", "P3"]
                .map(|name| candidate("a", "Alpha", name, None))
                .to_vec(),
            &[],
        )
        .unwrap(),
    );
    app.patch_browser.resolve_restored_preset();
    app.patch_browser.filter(false);
    app.patch_browser.reveal_playing();
    assert_eq!(app.patch_browser.selected, Some(3));
    assert!(app.patch_browser.requests.desired.is_none());
    // Only the cursor changes; there is no filter change or sound transaction.
    app.patch_browser.selected = Some(2);
    app.hide_resident();
    let Some(ToEditor::Cursor { context, patch }) = input.receive().unwrap() else {
        panic!("cursor event")
    };
    assert_eq!(context, "song/patch/1");
    assert_eq!(patch.unwrap().display, "P2");
    assert!(app.patch_browser.requests.desired.is_none());
}
