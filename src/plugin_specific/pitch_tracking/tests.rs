use super::*;
use crate::patch_browser_catalog::{tests::candidate, Snapshot};
use cmrt_patch_select::FilterGroup;
use cmrt_patches::PatchRole;
use std::collections::{BTreeMap, BTreeSet};

fn table(entries: &[(&str, &str, bool)]) -> PitchTrackingTable {
    let entries: Vec<_> = entries
        .iter()
        .map(|(plugin_id, display, tracks_keyboard)| {
            serde_json::json!({
                "plugin_id": plugin_id,
                "display": display,
                "tracks_keyboard": tracks_keyboard,
                "observations": [{ "note": 48, "frequency_hz": null }],
            })
        })
        .collect();
    let json = serde_json::json!({ "format_version": 1, "entries": entries });
    PitchTrackingTable::parse(json.to_string().as_bytes()).unwrap()
}

fn role(role: PatchRole) -> usize {
    FilterGroup::ALL
        .iter()
        .position(|g| g.role() == Some(role))
        .unwrap()
}

fn preset(snapshot: &Snapshot, role: usize, label: &str) -> usize {
    snapshot.presets[role]
        .iter()
        .position(|p| p.label == label)
        .unwrap()
}

/// (`plugin_id`, `display`) of the patches listed on a role's preset row.
fn members(snapshot: &Snapshot, role: usize, label: &str) -> BTreeSet<(String, String)> {
    let preset = preset(snapshot, role, label);
    snapshot
        .filter(role, preset, "")
        .unwrap()
        .iter()
        .map(|&index| {
            let candidate = &snapshot.candidates[index];
            (candidate.plugin_id.clone(), candidate.display.clone())
        })
        .collect()
}

fn key(plugin_id: &str, display: &str) -> (String, String) {
    (plugin_id.into(), display.into())
}

#[test]
fn tracking_percussion_moves_for_every_plugin_and_other_rows_stay() {
    let mut candidates = vec![
        candidate("dexed", "Dexed", "bank.syx/01 PERCSYN", None),
        candidate("surge", "Surge XT", "Drum One.fxp", Some("Percussion")),
        candidate("vast", "Vaporizer2", "DR Mystery.vvp", Some("Drum")),
        candidate("dexed", "Dexed", "bank.syx/02 CONGA perc", None),
        candidate("dexed", "Dexed", "bank.syx/03 kick", None),
        candidate("dexed", "Dexed", "bank.syx/04 snare", None),
        candidate("dexed", "Dexed", "bank.syx/05 hat", None),
        candidate("dexed", "Dexed", "bank.syx/06 plain lead", None),
        candidate("other", "Other", "bank.syx/01 PERCSYN", None),
    ];
    let original: Vec<_> = candidates.iter().map(|c| c.path.clone()).collect();
    let table = table(&[
        ("dexed", "bank.syx/01 PERCSYN", true),
        ("surge", "Drum One.fxp", true),
        ("vast", "DR Mystery.vvp", true),
        ("dexed", "bank.syx/02 CONGA perc", false),
        ("dexed", "bank.syx/03 kick", true),
        ("dexed", "bank.syx/04 snare", true),
        ("dexed", "bank.syx/05 hat", true),
        ("dexed", "bank.syx/06 plain lead", true),
    ]);
    apply_table(&mut candidates, &table);
    let moved: Vec<_> = candidates
        .iter()
        .filter(|c| c.entry.selector_category() == Some(PITCHED_PERCUSSION))
        .map(|c| (c.plugin_id.as_str(), c.display.as_str()))
        .collect();
    assert_eq!(
        moved,
        [
            ("dexed", "bank.syx/01 PERCSYN"),
            ("surge", "Drum One.fxp"),
            ("vast", "DR Mystery.vvp"),
        ]
    );
    assert_eq!(
        candidates
            .iter()
            .map(|c| c.path.clone())
            .collect::<Vec<_>>(),
        original
    );

    let snapshot = Snapshot::build(candidates, &[]).unwrap();
    let lead = role(PatchRole::Lead);
    assert_eq!(
        members(&snapshot, lead, PITCHED_PERCUSSION),
        [
            key("dexed", "bank.syx/01 PERCSYN"),
            key("surge", "Drum One.fxp"),
            key("vast", "DR Mystery.vvp"),
        ]
        .into()
    );
    let drum = role(PatchRole::Drum);
    assert_eq!(
        members(&snapshot, drum, "percussion"),
        [
            key("dexed", "bank.syx/02 CONGA perc"),
            key("other", "bank.syx/01 PERCSYN"),
        ]
        .into()
    );
    assert!(members(&snapshot, drum, "kick").contains(&key("dexed", "bank.syx/03 kick")));
    assert!(members(&snapshot, drum, "snare").contains(&key("dexed", "bank.syx/04 snare")));
    assert!(members(&snapshot, drum, "hi-hat").contains(&key("dexed", "bank.syx/05 hat")));
}

#[test]
fn unknown_format_version_is_rejected() {
    let json = r#"{ "format_version": 2, "entries": [] }"#;
    assert!(PitchTrackingTable::parse(json.as_bytes()).is_err());
}

#[test]
#[ignore = "requires the installed cmrt catalog and the sibling cat-music-patterns table; no audio/UI"]
fn installed_tracking_percussion_all_moves_to_lead() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cat-music-patterns")
        .join(PITCH_TRACKING_FILE);
    let table = PitchTrackingTable::parse(&std::fs::read(file).unwrap()).unwrap();
    let candidates =
        crate::patch_catalog::candidates(cmrt_tui_core::patch_catalog_cache::load().unwrap())
            .unwrap();
    let (lead, drum) = (role(PatchRole::Lead), role(PatchRole::Drum));
    let before = Snapshot::build(candidates.clone(), &[]).unwrap();
    let percussion = members(&before, drum, "percussion");
    let tracking: BTreeSet<_> = table
        .tracking()
        .map(|(plugin_id, display)| key(plugin_id, display))
        .collect();
    let mut applied = candidates.clone();
    apply_table(&mut applied, &table);
    // The app path (remote, else the cached copy) must give the same categories.
    let mut loaded = candidates;
    apply(&mut loaded);
    assert!(applied
        .iter()
        .zip(&loaded)
        .all(|(a, l)| a.entry.selector_category() == l.entry.selector_category()));
    let after = Snapshot::build(applied, &[]).unwrap();
    let pitched = members(&after, lead, PITCHED_PERCUSSION);
    let expected: BTreeSet<_> = percussion.intersection(&tracking).cloned().collect();
    // Measured patches named mallet or bell go to that row instead.
    let named: BTreeSet<_> = members(&after, lead, "mallet")
        .union(&members(&after, lead, "bell"))
        .filter(|patch| expected.contains(*patch))
        .cloned()
        .collect();
    let in_lead: BTreeSet<_> = pitched.union(&named).cloned().collect();
    assert_eq!(in_lead, expected);
    assert!(members(&after, drum, "percussion").is_disjoint(&tracking));

    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (plugin_id, _) in &tracking {
        counts.entry(plugin_id).or_default().0 += 1;
    }
    for (plugin_id, _) in &pitched {
        counts.entry(plugin_id).or_default().1 += 1;
    }
    for (plugin_id, (in_table, moved)) in &counts {
        println!("{plugin_id}: table true {in_table}, moved {moved}");
    }
    for (plugin_id, display) in &named {
        println!("mallet or bell by name: {plugin_id} {display}");
    }
    for (plugin_id, display) in tracking.difference(&in_lead) {
        let reason = if before
            .candidates
            .iter()
            .any(|c| c.plugin_id == *plugin_id && c.display == *display)
        {
            "not on the drum percussion row in this app"
        } else {
            "not in this app's catalog"
        };
        println!("not moved: {plugin_id} {display}: {reason}");
    }
}
