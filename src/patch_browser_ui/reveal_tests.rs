use super::*;

fn three() -> Browser {
    let patches = ["P1", "P2", "P3"].map(|name| candidate("a", "Alpha", name, None));
    let mut browser = Browser {
        snapshot: Some(Snapshot::build(patches.to_vec(), &[]).unwrap()),
        ..Default::default()
    };
    browser.open_window();
    browser.filter(false);
    browser
}

fn display(browser: &Browser) -> &str {
    let index = browser.selected.unwrap();
    &browser.snapshot.as_ref().unwrap().candidates[index].display
}

#[test]
fn open_moves_cursor_to_playing_patch_without_requesting_it() {
    let ctx = egui::Context::default();
    let mut browser = three();
    let mut playing = candidate("a", "Alpha", "P3", None);
    // Random patch candidates carry the installed bundle path, not the catalog one.
    playing.bundle_path = "X:/elsewhere/a.clap".into();
    browser.requests.applied = Some(playing);
    browser.reveal_playing();
    frame(&mut browser, &ctx, vec![]);
    assert_eq!(display(&browser), "P3");
    assert!(
        browser.requests.desired.is_none(),
        "open alone does not apply"
    );
    frame(&mut browser, &ctx, vec![key(Key::H)]);
    assert_eq!(display(&browser), "P2", "h starts from the playing patch");
}

#[test]
fn unknown_playing_patch_keeps_the_cursor_from_before_close() {
    let ctx = egui::Context::default();
    let mut browser = three();
    browser.reveal_playing();
    frame(&mut browser, &ctx, vec![key(Key::L)]);
    frame(&mut browser, &ctx, vec![key(Key::L)]);
    assert_eq!(display(&browser), "P2");
    browser.close();
    browser.open_window();
    browser.reveal_playing();
    frame(&mut browser, &ctx, vec![key(Key::L)]);
    assert_eq!(display(&browser), "P3", "l continues from the kept cursor");
}

#[test]
fn open_scrolls_a_distant_playing_patch_into_view() {
    let ctx = egui::Context::default();
    let patches = (0..600).map(|n| candidate("a", "Alpha", &format!("P{n:03}"), None));
    let mut browser = Browser {
        snapshot: Some(Snapshot::build(patches.collect(), &[]).unwrap()),
        ..Default::default()
    };
    browser.open_window();
    browser.filter(false);
    browser.requests.applied = Some(candidate("a", "Alpha", "P590", None));
    browser.reveal_playing();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    let cell = browser
        .results
        .iter()
        .position(|&index| Some(index) == browser.selected)
        .unwrap();
    let painted = ctx.data(|data| {
        data.get_temp::<std::ops::Range<usize>>(egui::Id::new("painted_patch_rows"))
            .unwrap()
    });
    assert!(painted.contains(&cell), "{cell} not in {painted:?}");
}

#[test]
fn playing_patch_survives_restart_and_receives_the_cursor() {
    let ctx = egui::Context::default();
    let mut before = three();
    before.requests.applied = Some(candidate("a", "Alpha", "P3", None));
    let root = std::env::temp_dir().join(format!("cat-browser-playing-{}", std::process::id()));
    let config = root.join("config.toml");
    let status = crate::status::Status {
        patch_browser: before.saved_filter(),
        ..Default::default()
    };
    status.save(&config).unwrap();
    let saved = crate::status::Status::load(&config).unwrap().patch_browser;
    std::fs::remove_dir_all(&root).unwrap();
    let mut after = Browser::restored(saved.clone());
    // A session that never opens the browser keeps the playing patch for the next one.
    assert_eq!(after.saved_filter(), saved);
    after.snapshot = three().snapshot;
    after.resolve_restored_playing();
    after.open_window();
    after.filter(false);
    after.reveal_playing();
    frame(&mut after, &ctx, vec![]);
    assert_eq!(display(&after), "P3");
    assert!(
        after.requests.desired.is_none(),
        "restoring does not reapply"
    );
    frame(&mut after, &ctx, vec![key(Key::H)]);
    assert_eq!(display(&after), "P2");
}

#[test]
fn patch_applied_after_startup_wins_over_the_saved_one() {
    let mut old = three();
    old.requests.applied = Some(candidate("a", "Alpha", "P1", None));
    let mut browser = Browser::restored(old.saved_filter());
    // e.g. Random patch before the browser was ever opened.
    browser.requests.applied = Some(candidate("a", "Alpha", "P2", None));
    browser.snapshot = three().snapshot;
    browser.resolve_restored_playing();
    assert_eq!(browser.requests.applied.as_ref().unwrap().display, "P2");
}
