use super::*;

#[test]
#[ignore = "read-only installed Mind Game and CR-909 catalog acceptance"]
fn installed_default_and_lightweight_fallback() {
    let mut candidates = crate::patch_catalog::load().unwrap();
    let exists = |c: &Candidate| c.path.is_file() && c.bundle_path.exists();
    let preferred = choose(&candidates, exists).unwrap();
    assert_eq!(preferred.display, DEFAULT);
    assert!(!crate::drum_browser::kit_metadata(preferred, 1)
        .unwrap()
        .notes
        .is_empty());
    candidates.retain(|c| !(c.plugin_id == PLUGIN && c.display == DEFAULT));
    let fallback = choose(&candidates, exists).unwrap();
    assert_eq!(
        fallback.display,
        "Free Sounds/Programs/Analog Industries/CR-909/main.sfz"
    );
    assert!(!fallback.measurement.is_heavy_offline_load());
}

fn kit(name: &str, bytes: Option<u64>) -> Candidate {
    let mut c = crate::drum_browser::tests::candidate(PLUGIN);
    c.display = name.into();
    c.measurement.sfz_sample_bytes = bytes;
    c
}

#[test]
fn mind_game_wins_over_the_first_kit_and_its_load_gate() {
    let kits = [kit("CR-909", Some(0)), kit(DEFAULT, Some(64_000_000))];
    assert_eq!(choose(&kits, |_| true).unwrap().display, DEFAULT);
}

#[test]
fn fallback_keeps_catalog_order_and_uses_the_exact_space_load_gate() {
    let kits = [
        kit("heavy", Some(64_000_000)),
        kit("first", Some(63_999_999)),
        kit("unknown", None),
    ];
    assert_eq!(choose(&kits, |_| true).unwrap().display, "first");
    assert_eq!(choose(&kits[2..], |_| true).unwrap().display, "unknown");
    assert!(choose(&kits[..1], |_| true).is_none());
}

#[test]
fn missing_default_and_invalid_metadata_do_not_block_the_fallback() {
    let mut invalid = kit("invalid", Some(0));
    invalid.measurement.drum_kit_notes = Some(vec![]);
    let kits = [kit(DEFAULT, Some(0)), invalid, kit("CR-909", Some(0))];
    assert_eq!(
        choose(&kits, |c| c.display != DEFAULT).unwrap().display,
        "CR-909"
    );
    assert!(choose(&kits, |_| false).is_none());
}
