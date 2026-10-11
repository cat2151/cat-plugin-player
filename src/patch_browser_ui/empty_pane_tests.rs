//! Roles and presets without patches are not painted, so no click lands on an empty list.
use super::*;
use crate::patch_browser_catalog::{tests::candidate, Snapshot};

fn painted(browser: &mut Browser, ctx: &egui::Context) -> Vec<String> {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        },
        |ctx| browser.show(ctx, None, "test"),
    );
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

fn bass_only() -> Browser {
    let snapshot =
        Snapshot::build(vec![candidate("a", "Alpha", "Sub Bass", Some("Bass"))], &[]).unwrap();
    Browser {
        open: true,
        snapshot: Some(snapshot),
        ..Default::default()
    }
}

#[test]
fn empty_roles_and_builtin_presets_are_not_painted() {
    let ctx = egui::Context::default();
    let mut browser = bass_only();
    let bass = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Bass))
        .unwrap();
    browser.role = bass;
    browser.filter(false);
    let texts = painted(&mut browser, &ctx);
    let snapshot = browser.snapshot.as_ref().unwrap();
    let mut hidden_roles = 0;
    for (role, group) in FilterGroup::ALL.iter().enumerate() {
        let listed = browser.listing.role(role);
        hidden_roles += usize::from(!listed);
        assert_eq!(
            texts.iter().any(|text| text == group.label()),
            listed,
            "{}",
            group.label()
        );
    }
    assert!(hidden_roles > 0, "fixture must contain an empty role");
    let presets = &snapshot.presets[bass];
    let mut hidden_presets = 0;
    for (index, preset) in presets.iter().enumerate().skip(1) {
        let listed = !preset.matches.is_empty();
        assert_eq!(browser.listing.preset(bass, index), listed);
        hidden_presets += usize::from(!listed);
        assert_eq!(texts.contains(&preset.label), listed, "{}", preset.label);
    }
    assert!(hidden_presets > 0, "fixture must contain an empty preset");
}

#[test]
fn restored_empty_role_or_preset_falls_back_to_a_listed_one() {
    let mut browser = bass_only();
    let lead = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Lead))
        .unwrap();
    browser.role = lead;
    browser.preset = 1;
    browser.filter(false);
    assert_eq!((browser.role, browser.preset), (0, 0));
    assert_eq!(browser.results.len(), 1);

    let bass = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Bass))
        .unwrap();
    let empty = browser.snapshot.as_ref().unwrap().presets[bass]
        .iter()
        .position(|preset| preset.matches.is_empty())
        .unwrap();
    browser.role = bass;
    browser.preset = empty;
    browser.filter(false);
    assert_eq!((browser.role, browser.preset), (bass, 0));
    assert_eq!(browser.results.len(), 1);
}

#[test]
fn user_preset_without_patches_stays_listed_for_delete() {
    let users = cmrt_patch_select::prepare_user_presets(vec![("bass".into(), "zebra".into())]);
    let snapshot = Snapshot::build(
        vec![candidate("a", "Alpha", "Sub Bass", Some("Bass"))],
        &users,
    )
    .unwrap();
    let bass = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Bass))
        .unwrap();
    let user = snapshot.presets[bass]
        .iter()
        .position(|preset| preset.is_user)
        .unwrap();
    assert!(snapshot.presets[bass][user].matches.is_empty());
    assert!(snapshot.listing("").preset(bass, user));
}

#[test]
fn solo_hides_presets_of_other_plugins_and_leaves_a_hidden_choice() {
    let snapshot = Snapshot::build(
        vec![
            candidate("dexed", "Dexed", "BASS 1", None),
            candidate("surge", "Surge XT", "Sub Bass", None),
        ],
        &[],
    )
    .unwrap();
    let bass = FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(cmrt_patches::PatchRole::Bass))
        .unwrap();
    let fm = snapshot.presets[bass]
        .iter()
        .position(|preset| preset.label == "FM Bass")
        .unwrap();
    assert!(snapshot.listing("").preset(bass, fm));
    assert!(
        snapshot.listing("bass").preset(bass, fm),
        "free text is ignored"
    );
    assert!(!snapshot.listing("plugin:surgext").preset(bass, fm));
    assert!(!snapshot.listing("-plugin:dexed").preset(bass, fm));
    let mut browser = Browser {
        open: true,
        snapshot: Some(snapshot),
        role: bass,
        preset: fm,
        ..Default::default()
    };
    browser.filter(false);
    assert_eq!(browser.preset, fm);
    browser.query = "plugin:surgext".into();
    browser.filter(true);
    assert_eq!((browser.role, browser.preset), (bass, 0));
    assert_eq!(browser.results.len(), 1);
}
