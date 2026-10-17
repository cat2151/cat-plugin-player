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
    // Applied candidates carry the installed bundle path, not the catalog one.
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
    browser.requests.applied = Some(candidate("a", "Alpha", "P2", None));
    browser.snapshot = three().snapshot;
    browser.resolve_restored_playing();
    assert_eq!(browser.requests.applied.as_ref().unwrap().display, "P2");
}

#[test]
fn narrowing_panes_recovers_the_playing_patch_when_the_cursor_was_lost() {
    let users = cmrt_patch_select::prepare_user_presets(vec![("bass".into(), "bass".into())]);
    let patches =
        ["Bass Bright", "Bass Warm"].map(|name| candidate("a", "Alpha", name, Some("Bass")));
    let mut playing = patches[1].clone();
    playing.bundle_path = "X:/relocated/a.clap".into();
    let mut browser = Browser {
        snapshot: Some(Snapshot::build(patches.to_vec(), &users).unwrap()),
        ..Default::default()
    };
    browser.requests.applied = Some(playing);
    browser.filter(false);
    // Rescanning or opening may clear the cursor while the sound keeps playing.
    browser.role = FilterGroup::ALL
        .iter()
        .position(|group| group.label() == "Bass track")
        .unwrap();
    browser.filter(true);
    assert_eq!(display(&browser), "Bass Warm");
    assert!(
        browser.requests.desired.is_none(),
        "Role must not replace a listed sound"
    );
    browser.selected = None;
    browser.preset = browser.snapshot.as_ref().unwrap().presets[browser.role]
        .iter()
        .position(|preset| preset.is_user)
        .unwrap();
    browser.filter(true);
    assert_eq!(display(&browser), "Bass Warm");
    assert!(
        browser.requests.desired.is_none(),
        "Preset must not replace a listed sound"
    );
}

#[test]
fn recovering_from_empty_or_invalid_search_preserves_a_listed_playing_patch() {
    let mut browser = three();
    browser.requests.applied = Some(candidate("a", "Alpha", "P3", None));
    for query in ["no-such-patch", "[", "P"] {
        browser.query = query.into();
        browser.filter(true);
    }
    assert_eq!(display(&browser), "P3");
    assert!(browser.requests.desired.is_none());
}

#[test]
fn filtering_out_a_cursor_prefers_the_playing_patch_before_first_result() {
    let mut browser = three();
    browser.requests.applied = Some(candidate("a", "Alpha", "P3", None));
    browser.selected = browser
        .results
        .iter()
        .copied()
        .find(|&index| browser.snapshot.as_ref().unwrap().candidates[index].display == "P2");
    browser.query = "P1|P3".into();
    browser.filter(true);
    assert_eq!(display(&browser), "P3");
    assert!(browser.requests.desired.is_none());
    browser.query = "P1".into();
    browser.filter(true);
    assert_eq!(display(&browser), "P1");
    assert_eq!(
        browser.requests.desired.as_ref().unwrap().candidate.display,
        "P1"
    );
}

#[test]
fn playing_identity_waits_until_the_catalog_is_ready() {
    let mut before = three();
    before.requests.applied = Some(candidate("a", "Alpha", "P3", None));
    let mut browser = Browser::restored(before.saved_filter());
    browser.resolve_restored_playing();
    assert!(browser.restore_playing.is_some());
    browser.snapshot = before.snapshot;
    browser.resolve_restored_playing();
    browser.filter(true);
    assert_eq!(display(&browser), "P3");
    assert!(browser.requests.desired.is_none());
}

#[test]
fn cursor_survives_restart_by_patch_name_when_catalog_rows_change() {
    let mut before = three();
    before.selected = Some(2);
    before.requests.applied = Some(candidate("a", "Alpha", "P1", None));
    let saved = serde_json::to_vec(&before.saved_filter()).unwrap();
    let mut after = Browser::restored(serde_json::from_slice(&saved).unwrap());
    after.open_window();
    after.reveal_playing(); // Catalog has not loaded yet.
    assert!(after.restore_selected.is_some());
    assert_eq!(after.saved_cursor().unwrap().display, "P3");
    let mut patches = ["P0", "P1", "P2", "P3"].map(|name| candidate("a", "Alpha", name, None));
    patches[3].bundle_path = "X:/relocated/a.clap".into();
    after.snapshot = Some(Snapshot::build(patches.to_vec(), &[]).unwrap());
    after.resolve_restored_playing();
    after.filter(false);
    after.reveal_playing();
    assert_eq!(display(&after), "P3");
    assert_eq!(after.selected, Some(3), "new row is found by name");
    assert_eq!(after.requests.applied.as_ref().unwrap().display, "P1");
    assert!(
        after.requests.desired.is_none(),
        "cursor restore must not load audio"
    );
    assert!(after.scroll_to_selected);
    after.close();
    after.open_window();
    after.reveal_playing();
    assert_eq!(
        display(&after),
        "P3",
        "opening keeps the named cursor before playing P1"
    );
}

#[test]
fn cursor_identity_distinguishes_duplicate_names_and_missing_patches() {
    let mut before = three();
    before.selected = Some(2);
    let saved = before.saved_filter();
    let mut after = Browser::restored(saved.clone());
    after.snapshot = Some(Snapshot::build(vec![candidate("b", "Beta", "P3", None)], &[]).unwrap());
    after.filter(false);
    after.reveal_playing();
    assert!(
        after.selected.is_none(),
        "another plugin's P3 must not be selected"
    );
    assert!(after.requests.desired.is_none());
    let mut after = Browser::restored(saved);
    after.snapshot = three().snapshot;
    after.query = "P1".into();
    after.filter(false);
    after.reveal_playing();
    assert!(
        after.selected.is_none(),
        "a filtered-out patch must not be applied"
    );
    assert!(after.requests.desired.is_none());
}
