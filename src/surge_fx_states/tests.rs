use super::table::{encode_hex, sha256_hex, Entry, FORMAT_VERSION};
use super::*;
use std::path::Path;

fn table_json(format_version: u32, preset: &[u8], state: &str) -> Vec<u8> {
    serde_json::json!({
        "format_version": format_version,
        "plugin_id": PLUGIN_ID,
        "states": {
            sha256_hex(preset): {"path": "Rotary/Fast.srgfx", "state": state}
        }
    })
    .to_string()
    .into_bytes()
}

#[test]
fn state_is_found_by_preset_content_not_by_path() {
    let state = [0x56, 0x43, 0x32, 0x21, 0x00, 0xff];
    let table = Table::parse(&table_json(
        FORMAT_VERSION,
        b"<srgfx/>",
        &encode_hex(&state),
    ))
    .unwrap();
    assert_eq!(table.state(b"<srgfx/>").unwrap(), state);
    assert!(table.state(b"<srgfx edited/>").is_none());
}

#[test]
fn unknown_format_and_broken_hex_are_rejected() {
    let error = Table::parse(&table_json(FORMAT_VERSION + 1, b"x", "00")).unwrap_err();
    assert!(error.contains("format_version"), "{error}");
    for broken in ["0", "zz"] {
        let error = Table::parse(&table_json(FORMAT_VERSION, b"x", broken)).unwrap_err();
        assert!(error.contains("invalid state hex"), "{error}");
    }
    let mut other = Table {
        format_version: FORMAT_VERSION,
        plugin_id: "other".into(),
        ..Default::default()
    };
    other.states.insert(
        sha256_hex(b"x"),
        Entry {
            path: "x".into(),
            state: "00".into(),
        },
    );
    let error = Table::parse(&serde_json::to_vec(&other).unwrap()).unwrap_err();
    assert!(error.contains("plugin_id"), "{error}");
}

/// Needs the sibling `cat-music-patterns` checkout and Surge XT installed.
#[test]
#[ignore]
fn published_table_covers_every_installed_preset() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../cat-music-patterns/surge-xt-fx-states.json");
    let table = Table::parse(&std::fs::read(path).unwrap()).unwrap();
    let catalog = cmrt_core::AudioEffectCatalog::scan(
        cmrt_core::audio_effect::builtin_effect_plugins()
            .into_iter()
            .filter(|plugin| plugin.plugin_id == PLUGIN_ID)
            .collect(),
    );
    assert!(!catalog.presets().is_empty());
    for preset in catalog.presets() {
        let bytes = std::fs::read(&preset.path).unwrap();
        assert!(table.state(&bytes).is_some(), "{}", preset.value);
    }
}

/// Needs the network and the table pushed to cat-music-patterns.
#[test]
#[ignore]
fn published_url_serves_a_valid_table() {
    let bytes = crate::surge_patch_catalog::fetch(URL).unwrap();
    Table::parse(&bytes).unwrap();
}
