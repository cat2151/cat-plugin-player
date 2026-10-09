use super::*;
use crate::{
    favorites_store::{FavoriteCapture, Library},
    sequence_pattern::SequencePattern,
};

fn plugin() -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "com.floe-audio.floe".into(),
        name: "Floe".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/Floe.clap".into(),
    }
}

fn fixture(macro_value: f32, volume: f32, dirty: u8, mapping: u32) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&0x2a491f93u32.to_le_bytes());
    b.extend_from_slice(&30u16.to_le_bytes());
    b.extend_from_slice(&0x00020002u32.to_le_bytes());
    for _ in 0..3 {
        b.extend_from_slice(&[0, 0]); // No instrument, no velocity-curve points.
        b.extend_from_slice(&[0; 16 + 64 * 8 + 2]);
    }
    b.push(0); // Tags.
    b.extend_from_slice(&[0; 6]); // Author, description, instance ID strings.
    b.extend_from_slice(&2u16.to_le_bytes());
    for (id, value) in [(0u32, volume), (101, macro_value)] {
        b.extend_from_slice(&id.to_le_bytes());
        b.extend_from_slice(&value.to_le_bytes());
    }
    b.push(4);
    b.extend_from_slice(&[0; 20]); // Four empty macro names/destinations.
    b.push(0); // No IR.
    b.extend_from_slice(&[0; 8]); // Integrity check, FX order/visibility counts.
    b.extend_from_slice(&1u32.to_le_bytes());
    b.push(1);
    b.extend_from_slice(&mapping.to_le_bytes());
    b.extend_from_slice(&[0; 20]); // Config, display name/category, UUID.
    b.push(dirty);
    b
}

#[test]
fn only_mapped_sweep_macro_and_dirty_flag_are_excluded() {
    let a = fixture(1.0, 0.8, 0, 101);
    let b = fixture(0.4, 0.8, 1, 101);
    assert!(same_sweep_sound(&plugin(), &a, &b));
    assert!(!crate::plugin_specific::same_favorite_state(
        &plugin(),
        &a,
        &b
    ));
    for state in [
        fixture(0.4, 0.7, 1, 101),
        fixture(f32::NAN, 0.8, 1, 101),
        fixture(1.1, 0.8, 1, 101),
        fixture(0.4, 0.8, 2, 101),
    ] {
        assert!(!same_sweep_sound(&plugin(), &a, &state));
    }
    for mapping in [0, 102] {
        assert!(!same_sweep_sound(
            &plugin(),
            &fixture(1.0, 0.8, 0, mapping),
            &fixture(0.4, 0.8, 1, mapping)
        ));
    }
    for offset in [0, 4, 6] {
        let mut other = b.clone();
        other[offset] ^= 1;
        assert!(!same_sweep_sound(&plugin(), &a, &other));
    }
    let mut trailing = b.clone();
    trailing.push(0);
    assert!(!same_sweep_sound(&plugin(), &a, &trailing));
    assert!(!same_sweep_sound(&plugin(), &a, &b[..b.len() - 1]));
    let mut other = plugin();
    other.format = "VST3".into();
    assert!(!same_sweep_sound(&other, &a, &b));
    other = plugin();
    other.id = "other".into();
    assert!(!same_sweep_sound(&other, &a, &b));
}

#[test]
fn sweep_deduplicates_without_changing_state_name_or_pattern_identity() {
    let directory = std::env::temp_dir().join(format!(
        "cat-floe-sweep-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = directory.join("config.toml");
    let original = fixture(1.0, 0.8, 0, 101);
    let played = fixture(0.4, 0.8, 1, 101);
    let mut library = Library::default();
    let first = library
        .add(&config, plugin(), false, &original, SequencePattern::Steps)
        .unwrap();
    library.rename(&config, &first.id, "My harp").unwrap();
    let capture = FavoriteCapture {
        sequence_pattern: SequencePattern::Steps,
        sweep_cc1: true,
    };
    let same = library
        .add_automatic(&config, plugin(), false, &played, capture)
        .unwrap();
    assert_eq!(same.id, first.id);
    assert_eq!(same.name, "My harp");
    assert_eq!(library.state(&config, &first.id).unwrap(), original);
    let other = library
        .add_automatic(
            &config,
            plugin(),
            false,
            &played,
            FavoriteCapture {
                sequence_pattern: SequencePattern::GuitarArpeggio,
                ..capture
            },
        )
        .unwrap();
    assert_ne!(other.id, first.id);
    let manual = library
        .add(
            &config,
            plugin(),
            false,
            &fixture(0.6, 0.8, 1, 101),
            SequencePattern::Steps,
        )
        .unwrap();
    assert_ne!(manual.id, first.id);
    assert_eq!(library.entries.len(), 3);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires saved Floe CLAP snapshots; reads user files only"]
fn saved_floe_sweep_snapshots_keep_two_patterns() {
    let config = crate::config::path().unwrap();
    let library: Library = toml::from_str(
        &std::fs::read_to_string(config.parent().unwrap().join("favorites/index.toml")).unwrap(),
    )
    .unwrap();
    let entries: Vec<_> = library
        .entries
        .iter()
        .filter(|e| e.plugin.id == plugin().id)
        .collect();
    assert!(!entries.is_empty());
    let first = library.state(&config, &entries[0].id).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "cat-saved-floe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let temporary_config = directory.join("config.toml");
    let mut deduplicated = Library::default();
    let mut patterns = std::collections::BTreeSet::new();
    for entry in entries {
        let state = library.state(&config, &entry.id).unwrap();
        assert!(same_sweep_sound(&entry.plugin, &first, &state));
        patterns.insert(entry.sequence_pattern as u8);
        deduplicated
            .add_automatic(
                &temporary_config,
                entry.plugin.clone(),
                false,
                &state,
                FavoriteCapture {
                    sequence_pattern: entry.sequence_pattern,
                    sweep_cc1: true,
                },
            )
            .unwrap();
    }
    assert_eq!(deduplicated.entries.len(), patterns.len());
    assert_eq!(
        patterns.len(),
        2,
        "the two saved performance patterns must stay separate"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn startup_delay_uses_exact_clap_id_not_display_metadata() {
    let mut key = plugin();
    key.name = "Renamed".into();
    assert_eq!(
        crate::plugin_specific::automatic_note_start_delay(&key).as_millis(),
        100
    );
    key.format = "VST3".into();
    assert!(crate::plugin_specific::automatic_note_start_delay(&key).is_zero());
    key.format = "CLAP".into();
    key.id = "other".into();
    key.name = "Floe".into();
    assert!(crate::plugin_specific::automatic_note_start_delay(&key).is_zero());
}
