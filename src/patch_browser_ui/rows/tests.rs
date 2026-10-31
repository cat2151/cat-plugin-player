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
    let hover = |patch: &Candidate| {
        let snapshot = Snapshot::build(vec![patch.clone()], &[]).unwrap();
        details(&snapshot, 0)
    };
    let mut patch = candidate("a", "Alpha", "Bass/Patch", None);
    assert!(!hover(&patch).contains("category"));
    assert!(!hover(&patch).contains("merged"));
    assert!(!hover(&patch).contains("Load:"));
    assert!(!hover(&patch).contains("Size:"));
    patch.entry = patch.entry.with_merged(3, vec!["alias".into()]);
    assert!(hover(&patch).contains("3 merged"));
    patch.entry = cmrt_patch_select::PatchCatalogEntry::new(
        "Bass/Patch".into(),
        "bass/patch".into(),
        "Alpha".into(),
        Some("Bass".into()),
    );
    assert!(hover(&patch).contains("Category: Bass"));
}

/// In ALL the row alone does not say where the patch belongs, so the hover names it.
#[test]
fn hover_names_the_role_and_presets_holding_the_patch() {
    let users = cmrt_patch_select::prepare_user_presets(vec![("bass".into(), "zebra".into())]);
    let snapshot = Snapshot::build(
        vec![
            candidate("a", "Alpha", "Zebra", Some("Bass")),
            candidate("a", "Alpha", "Other", Some("Bass")),
        ],
        &users,
    )
    .unwrap();
    let zebra = snapshot
        .candidates
        .iter()
        .position(|candidate| candidate.display == "Zebra")
        .unwrap();
    let text = details(&snapshot, zebra);
    assert!(text.contains("\nRole: Bass track"), "{text}");
    assert!(text.contains("zebra"), "{text}");
    assert!(!details(&snapshot, 1 - zebra).contains("zebra"));
    assert!(!text.contains("ALL"), "{text}");
}
