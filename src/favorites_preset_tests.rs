use super::*;

#[test]
fn preset_names_roundtrip_backfill_and_remain_separate_from_renames() {
    let root = std::env::temp_dir().join(format!("cat-favorite-presets-{}", std::process::id()));
    let config = root.join("config.toml");
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "synth".into(),
        name: "Synth".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/synth.clap".into(),
    };
    for effect in [false, true] {
        let mut library = Library::default();
        let legacy = library
            .add(
                &config,
                plugin.clone(),
                effect,
                &[1],
                SequencePattern::Steps,
            )
            .unwrap();
        let mut capture = FavoriteCapture::from(SequencePattern::Steps);
        capture.preset_name = Some("日本語 Warm Pad".into());
        let unchanged = library
            .add(&config, plugin.clone(), effect, &[1], capture.clone())
            .unwrap();
        assert_eq!(unchanged.id, legacy.id);
        assert_eq!(unchanged.name, "Synth 1");
        library.rename(&config, &legacy.id, "My favorite").unwrap();
        let named = library
            .add_automatic(&config, plugin.clone(), effect, &[1], capture.clone())
            .unwrap();
        assert_eq!(named.id, legacy.id);
        assert_eq!(named.name, "My favorite");
        assert_eq!(named.preset_name, capture.preset_name);
        assert!(named.matches_filter("WARM PAD"));
        assert!(named.matches_filter("日本語"));
        assert!(named.matches_filter("my favorite"));
        assert!(named.matches_filter("synth"));
        assert!(!named.matches_filter("missing"));
        let loaded = Library::load(&config).unwrap();
        assert_eq!(loaded.entries[0].preset_name, capture.preset_name);
        // No name in a later capture must not erase the saved origin.
        let again = library
            .add(
                &config,
                plugin.clone(),
                effect,
                &[1],
                SequencePattern::Steps,
            )
            .unwrap();
        assert_eq!(again.preset_name, capture.preset_name);
        assert_eq!(library.entries.len(), 1);
        capture.preset_name = Some("Different".into());
        let different = library
            .add(&config, plugin.clone(), effect, &[2], capture)
            .unwrap();
        assert_ne!(different.id, named.id);
        assert_eq!(different.name, "Different 1");
        // Old metadata without preset_name still deserializes.
        let mut value = toml::Value::try_from(named).unwrap();
        value.as_table_mut().unwrap().remove("preset_name");
        let old: Favorite = value.try_into().unwrap();
        assert_eq!(old.preset_name, None);
        assert!(old.matches_filter("my favorite"));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_snapshots_prefer_preset_names_and_keep_unique_numbers() {
    let root =
        std::env::temp_dir().join(format!("cat-favorite-preset-naming-{}", std::process::id()));
    let config = root.join("config.toml");
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "synth".into(),
        name: "Synth".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/synth.clap".into(),
    };
    let mut library = Library::default();
    let mut number = 0;
    for effect in [false, true] {
        for automatic in [false, true] {
            number += 1;
            let mut capture = FavoriteCapture::from(SequencePattern::Steps);
            capture.preset_name = Some("  日本語 Flute  ".into());
            let favorite = if automatic {
                library.add_automatic(&config, plugin.clone(), effect, &[number], capture)
            } else {
                library.add(&config, plugin.clone(), effect, &[number], capture)
            }
            .unwrap();
            assert_eq!(favorite.name, format!("日本語 Flute {number}"));
            assert_eq!(favorite.preset_name.as_deref(), Some("  日本語 Flute  "));
            assert_eq!(favorite.history, automatic);
            assert_eq!(favorite.favorite, !automatic);
        }
    }
    // A different plugin and a user-assigned name share the same name space.
    let first = library.entries.last().unwrap().id.clone();
    library.rename(&config, &first, "日本語 Flute 5").unwrap();
    let mut capture = FavoriteCapture::from(SequencePattern::Steps);
    capture.preset_name = Some("日本語 Flute".into());
    let other = library
        .add(
            &config,
            PluginKey {
                id: "other".into(),
                ..plugin.clone()
            },
            false,
            &[1],
            capture.clone(),
        )
        .unwrap();
    assert_eq!(other.name, "日本語 Flute 1");
    let next = library
        .add_automatic(&config, other.plugin.clone(), false, &[2], capture)
        .unwrap();
    assert_eq!(next.name, "日本語 Flute 6");
    assert_eq!(Library::load(&config).unwrap().entries[0].name, next.name);
    // Unknown, empty and whitespace-only preset names retain plugin naming.
    for effect in [false, true] {
        for automatic in [false, true] {
            for preset_name in [None, Some(""), Some(" \t\n ")] {
                number += 1;
                let mut capture = FavoriteCapture::from(SequencePattern::Steps);
                capture.preset_name = preset_name.map(str::to_owned);
                let favorite = if automatic {
                    library.add_automatic(&config, plugin.clone(), effect, &[number], capture)
                } else {
                    library.add(&config, plugin.clone(), effect, &[number], capture)
                }
                .unwrap();
                assert_eq!(favorite.name, format!("Synth {}", number - 4));
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
