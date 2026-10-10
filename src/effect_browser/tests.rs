use super::*;
use cmrt_core::audio_effect::PresetLocation;

pub(crate) fn candidate(plugin: &str, name: &str, category: &str, kind: &str) -> Candidate {
    Candidate {
        plugin_id: format!("id.{plugin}"),
        plugin_name: plugin.into(),
        preset: PresetLocation {
            path: format!("X:/presets/{plugin}/{name}").into(),
            value: name.into(),
            display: format!("{plugin}: {name}"),
        },
        name: name.into(),
        category: category.into(),
        kind: kind.into(),
    }
}

pub(crate) fn browser() -> EffectBrowser {
    let mut browser = EffectBrowser::default();
    browser.open_window(&[]);
    browser.refresh(
        1,
        vec![
            candidate("Surge", "Big Hall", "Space", "Reverb"),
            candidate("Surge", "Tape Echo", "Time", "Delay"),
            candidate("Dragonfly", "Small Room", "Space", "Reverb"),
            candidate("Surge", "Plate", "Space", "Plate"),
        ],
    );
    browser
}

fn names(browser: &EffectBrowser) -> Vec<&str> {
    browser
        .results
        .iter()
        .map(|&index| browser.candidates[index].name.as_str())
        .collect()
}

#[test]
fn category_narrows_kinds_and_list_without_requesting_on_open() {
    let mut browser = browser();
    assert_eq!(browser.categories, ["ALL", "Space", "Time"]);
    assert_eq!(browser.kinds, ["ALL", "Delay", "Plate", "Reverb"]);
    assert_eq!(names(&browser).len(), 4);
    assert!(browser.desired.is_none(), "opening alone changes no sound");
    browser.choose_category(1);
    assert_eq!(browser.kinds, ["ALL", "Plate", "Reverb"]);
    assert_eq!(names(&browser), ["Big Hall", "Small Room", "Plate"]);
    assert_eq!(browser.desired.as_ref().unwrap().name, "Big Hall");
    browser.choose_kind(2);
    assert_eq!(names(&browser), ["Big Hall", "Small Room"]);
    // A new category resets kind to ALL.
    browser.choose_category(2);
    assert_eq!(browser.kind, 0);
    assert_eq!(names(&browser), ["Tape Echo"]);
}

#[test]
fn search_matches_name_category_kind_or_plugin_and_keeps_list_on_bad_regex() {
    let mut browser = browser();
    browser.query = "dragonfly".into();
    browser.filter(true);
    assert_eq!(names(&browser), ["Small Room"]);
    browser.query = "space -reverb".into();
    browser.filter(true);
    assert_eq!(names(&browser), ["Plate"]);
    browser.query = "(".into();
    browser.filter(true);
    assert!(browser.error.is_some());
    assert_eq!(
        names(&browser),
        ["Plate"],
        "half-typed pattern keeps the list"
    );
}

#[test]
fn target_prefers_browsed_slot_then_last_effect_then_add() {
    let mut browser = browser();
    assert_eq!(browser.target, None, "no effect connected: add one");
    browser.close();
    browser.open_window(&[10, 11]);
    assert_eq!(browser.target, Some(11));
    let hall = browser.candidates[0].clone();
    browser.applied(10, hall.clone());
    browser.close();
    browser.open_window(&[10, 11]);
    assert_eq!(browser.target, Some(10));
    // The slot holding the selected preset is not reloaded.
    browser.select(0);
    assert!(browser.desired.is_none());
    browser.select(1);
    assert_eq!(browser.desired.as_ref().unwrap().name, "Tape Echo");
    browser.sync_target(&[11]);
    assert_eq!(browser.target, Some(11));
    browser.sync_target(&[]);
    assert_eq!(browser.target, None);
    // An explicit Add choice survives while slots exist.
    browser.sync_target(&[11]);
    assert_eq!(browser.target, None);
}

#[test]
fn refresh_keeps_conditions_and_cursor_by_name() {
    let mut browser = browser();
    browser.choose_category(1);
    browser.choose_kind(2);
    browser.select(2);
    let mut next = browser.candidates.clone();
    next.insert(0, candidate("Shu", "Air", "Space", "Reverb"));
    browser.refresh(2, next);
    assert_eq!(browser.categories[browser.category], "Space");
    assert_eq!(browser.kinds[browser.kind], "Reverb");
    assert_eq!(names(&browser), ["Air", "Big Hall", "Small Room"]);
    assert_eq!(
        browser.candidates[browser.selected.unwrap()].name,
        "Small Room"
    );
}

#[test]
fn conditions_cursor_and_target_position_survive_status_json_round_trip() {
    let mut browser = browser();
    browser.choose_category(1);
    browser.choose_kind(2);
    browser.select(2);
    browser.open_window(&[10, 11, 12]);
    browser.target = Some(11);
    let root = std::env::temp_dir().join(format!("cat-effect-filter-{}", std::process::id()));
    let config = root.join("config.toml");
    let status = crate::status::Status {
        effect_browser: browser.saved_filter(&[10, 11, 12]),
        ..Default::default()
    };
    status.save(&config).unwrap();
    let saved = crate::status::Status::load(&config).unwrap().effect_browser;
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(saved.slot, Some(1));
    let mut restored = EffectBrowser::restored(saved.clone());
    // A session that never opens the browser keeps the conditions for the next one.
    assert_eq!(restored.saved_filter(&[]), saved);
    // New instance ids after restart; the same chain position is targeted.
    restored.open_window(&[20, 21, 22]);
    assert_eq!(restored.target, Some(21));
    restored.refresh(1, browser.candidates.clone());
    assert_eq!(restored.categories[restored.category], "Space");
    assert_eq!(restored.kinds[restored.kind], "Reverb");
    assert_eq!(restored.results, browser.results);
    assert_eq!(
        restored.candidates[restored.selected.unwrap()].name,
        "Small Room",
        "arrow keys continue from the saved cursor, not the first row"
    );
    assert!(restored.desired.is_none(), "restoring changes no sound");
    // A name the catalog no longer has falls back to ALL; the other is kept if it still exists.
    let mut missing = EffectBrowser::restored(crate::status::EffectBrowserFilter {
        category: "Gone".into(),
        ..saved
    });
    missing.open_window(&[]);
    missing.refresh(1, browser.candidates.clone());
    assert_eq!(missing.category, 0);
    assert_eq!(missing.kinds[missing.kind], "Reverb");
}

#[test]
fn saved_query_survives_round_trip() {
    let mut browser = browser();
    browser.query = "hall".into();
    browser.filter(true);
    let saved = browser.saved_filter(&[]);
    let mut restored = EffectBrowser::restored(saved);
    restored.open_window(&[]);
    restored.refresh(1, browser.candidates.clone());
    assert_eq!(restored.query, "hall");
    assert_eq!(restored.results, browser.results);
    assert_eq!(restored.selected, browser.selected);
}

mod native;
