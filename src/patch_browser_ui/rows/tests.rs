use super::*;
use crate::patch_browser_catalog::tests::candidate;

#[test]
fn contextual_names_keep_shortest_unique_suffix_and_dexed_program_identity() {
    let candidates = vec![
        candidate("a", "Alpha", "Bass/Bank A/Shared", None),
        candidate("a", "Alpha", "Lead/Bank B/Shared", None),
        candidate("a", "Alpha", "Bass/Unique", None),
        candidate("d", "Dexed", "Bank.syx/01 LAURIE", None),
        candidate("d", "Dexed", "Other.syx/01 LAURIE", None),
    ];
    assert_eq!(
        labels(&candidates, &[0, 1, 2]),
        ["Bank A/Shared", "Bank B/Shared", "Unique"]
    );
    assert_eq!(labels(&candidates, &[0]), ["Shared"]);
    assert_eq!(
        labels(&candidates, &[0, 3, 4]),
        [
            "Alpha — Shared",
            "Dexed — Bank.syx/01 LAURIE",
            "Dexed — Other.syx/01 LAURIE"
        ]
    );
    assert_eq!(
        candidates[3].path.to_string_lossy(),
        "X:/patches/d/Bank.syx/01 LAURIE"
    );
    let same_name = vec![
        candidate("a", "Synth", "Same", None),
        candidate("b", "Synth", "Same", None),
    ];
    assert_ne!(
        labels(&same_name, &[0, 1])[0],
        labels(&same_name, &[0, 1])[1]
    );
}

#[test]
fn hover_only_has_valid_category_and_multiple_merge_details() {
    let mut patch = candidate("a", "Alpha", "Bass/Patch", None);
    assert!(!details(&patch).contains("category"));
    assert!(!details(&patch).contains("merged"));
    assert!(!details(&patch).contains("Load:"));
    assert!(!details(&patch).contains("Size:"));
    patch.entry = patch.entry.with_merged(3, vec!["alias".into()]);
    assert!(details(&patch).contains("3 merged"));
    patch.entry = cmrt_patch_select::PatchCatalogEntry::new(
        "Bass/Patch".into(),
        "bass/patch".into(),
        "Alpha".into(),
        Some("Bass".into()),
    );
    assert!(details(&patch).contains("Category: Bass"));
}
