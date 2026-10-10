use super::*;
use crate::patch_browser_catalog::{tests::candidate, Snapshot};

fn names(tints: &[RowTint]) -> Vec<Option<usize>> {
    tints.iter().map(|tint| tint.name).collect()
}

#[test]
fn shown_plugins_tint_only_the_plugin_prefix() {
    let candidates = vec![
        candidate("a", "Alpha", "One", Some("Bass")),
        candidate("b", "Beta", "Two", Some("Lead")),
        candidate("a", "Alpha", "Three", Some("Lead")),
    ];
    let tints = tints(&candidates, &[0, 1, 2], None, true);
    let plugins: Vec<_> = tints.iter().map(|tint| tint.plugin).collect();
    assert_eq!(plugins, [Some(0), Some(1), Some(0)]);
    assert_eq!(names(&tints), [None, None, None]);
}

#[test]
fn presets_under_role_all_outrank_categories() {
    let users = cmrt_patch_select::prepare_user_presets(vec![
        ("bass".into(), "zebra".into()),
        ("bass".into(), "yak".into()),
    ]);
    let snapshot = Snapshot::build(
        vec![
            candidate("a", "Alpha", "Zebra", Some("Same")),
            candidate("a", "Alpha", "Yak", Some("Same")),
        ],
        &users,
    )
    .unwrap();
    let bass = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Bass))
        .unwrap();
    let results = snapshot.filter(bass, 0, "").unwrap();
    let presets = Some(snapshot.presets[bass].as_slice());
    let tints = tints(&snapshot.candidates, &results, presets, false);
    assert_eq!(names(&tints), [Some(0), Some(1)]);
    assert!(tints.iter().all(|tint| tint.plugin.is_none()));
    // A chosen preset passes no presets, and equal categories leave nothing to tell apart.
    let tints = super::tints(&snapshot.candidates, &results, None, false);
    assert_eq!(names(&tints), [None, None]);
}

#[test]
fn categories_tint_names_and_uncategorized_rows_stay_plain() {
    let candidates = vec![
        candidate("a", "Alpha", "One", Some("Bass")),
        candidate("a", "Alpha", "Two", Some("Lead")),
        candidate("a", "Alpha", "Three", None),
        candidate("a", "Alpha", "Four", Some("Bass")),
    ];
    let tints = super::tints(&candidates, &[0, 1, 2, 3], None, false);
    assert_eq!(names(&tints), [Some(0), Some(1), None, Some(0)]);
    assert_eq!(
        names(&super::tints(&candidates, &[0, 3], None, false)),
        [None, None]
    );
}
