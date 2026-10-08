use super::*;

#[test]
fn automatic_saves_ignore_stop_in_both_directions_but_keep_other_differences() {
    let dir = std::env::temp_dir().join(format!(
        "cat-auto-favorite-stop-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = dir.join("config.toml");
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "synth".into(),
        name: "Synth".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/synth.clap".into(),
    };
    for (i, &pattern) in SequencePattern::TYPES.iter().enumerate() {
        for (first_pattern, second_pattern) in [
            (pattern, SequencePattern::Off),
            (SequencePattern::Off, pattern),
        ] {
            let config = config.with_file_name(format!("case-{i}-{first_pattern:?}/config.toml"));
            let mut library = Library::default();
            // Automatic saves must also reuse manually saved and renamed entries.
            let first = library
                .add(&config, plugin.clone(), false, &[1], first_pattern)
                .unwrap();
            library.rename(&config, &first.id, "My sound").unwrap();
            let mut library = Library::load(&config).unwrap();
            let saved = library
                .add_automatic(&config, plugin.clone(), false, &[1], second_pattern)
                .unwrap();
            assert_eq!(saved.id, first.id);
            assert_eq!(saved.name, "My sound");
            assert_eq!(saved.sequence_pattern, first_pattern);
            assert_eq!(library.entries.len(), 1);
            assert_eq!(library.state(&config, &first.id).unwrap(), [1]);
            assert_eq!(
                std::fs::read_dir(directory(&config).unwrap())
                    .unwrap()
                    .count(),
                2
            );
            // A sound change still creates an automatic snapshot while stopped.
            library
                .add_automatic(&config, plugin.clone(), false, &[2], SequencePattern::Off)
                .unwrap();
            assert_eq!(library.entries.len(), 2);
            // Manual saves still distinguish stopped and playing snapshots.
            let manual = library
                .add(&config, plugin.clone(), false, &[1], second_pattern)
                .unwrap();
            assert_ne!(manual.id, first.id);
            assert_eq!(library.entries.len(), 3);
        }
    }
    let mut library = Library::default();
    let playing = library
        .add_automatic(&config, plugin.clone(), false, &[1], SequencePattern::Steps)
        .unwrap();
    let other_pattern = library
        .add_automatic(
            &config,
            plugin.clone(),
            false,
            &[1],
            SequencePattern::GuitarArpeggio,
        )
        .unwrap();
    assert_ne!(playing.id, other_pattern.id);
    assert_eq!(library.entries.len(), 2);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn repeated_automatic_saves_preserve_numbers_and_renamed_favorites() {
    let dir = std::env::temp_dir().join(format!(
        "cat-auto-favorite-name-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = dir.join("config.toml");
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "dexed".into(),
        name: "Dexed".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/Dexed.clap".into(),
    };
    let mut library = Library::default();
    let first = library
        .add_automatic(&config, plugin.clone(), false, &[1], SequencePattern::Steps)
        .unwrap();
    let second = library
        .add_automatic(&config, plugin.clone(), false, &[2], SequencePattern::Steps)
        .unwrap();
    assert_eq!(first.name, "auto Dexed 1");
    assert_eq!(second.name, "auto Dexed 2");
    for _ in 0..3 {
        let saved = library
            .add_automatic(&config, plugin.clone(), false, &[1], SequencePattern::Steps)
            .unwrap();
        assert_eq!(saved.id, first.id);
        assert_eq!(saved.name, first.name);
        assert_eq!(library.entries.len(), 2);
    }
    library
        .rename(&config, &first.id, "My Dexed sound")
        .unwrap();
    let mut library = Library::load(&config).unwrap();
    let index = directory(&config).unwrap().join("index.toml");
    let before = std::fs::read(&index).unwrap();
    let saved = library
        .add_automatic(&config, plugin, false, &[1], SequencePattern::Steps)
        .unwrap();
    assert_eq!(saved.id, first.id);
    assert_eq!(saved.name, "My Dexed sound");
    assert_eq!(std::fs::read(index).unwrap(), before);
    assert_eq!(library.state(&config, &first.id).unwrap(), [1]);
    assert_eq!(library.entries[0].id, first.id);
    assert_eq!(library.entries[1].id, second.id);
    assert_eq!(
        std::fs::read_dir(directory(&config).unwrap())
            .unwrap()
            .count(),
        3
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn old_favorites_use_the_original_phrase() {
    let library: Library = toml::from_str(
            "[[entries]]\nid = 'abc'\nname = 'Old sound'\neffect = false\n[entries.plugin]\nformat = 'CLAP'\nid = 'synth'\n"
        ).unwrap();
    assert_eq!(library.entries[0].sequence_pattern, SequencePattern::Steps);
}

#[test]
fn duplicate_snapshots_keep_the_existing_name_and_rollback_on_index_failure() {
    let dir = std::env::temp_dir().join(format!(
        "cat-favorite-dedup-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = dir.join("config.toml");
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "synth".into(),
        name: "Synth".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/synth.clap".into(),
    };
    let mut library = Library::default();
    let old = library
        .add(&config, plugin.clone(), false, &[1], SequencePattern::Steps)
        .unwrap();
    library.rename(&config, &old.id, "My sound").unwrap();
    let newest = library
        .add_automatic(&config, plugin.clone(), false, &[1], SequencePattern::Steps)
        .unwrap();
    assert_eq!(newest.name, "My sound");
    assert_eq!(newest.id, old.id);
    assert_eq!(library.entries.len(), 1);
    assert!(state_path(&config, &old.id).unwrap().exists());
    assert_eq!(Library::load(&config).unwrap().entries[0].id, newest.id);
    assert_eq!(library.state(&config, &newest.id).unwrap(), [1]);
    // Different state, pattern, role or plugin identity must survive.
    for (key, effect, state, pattern) in [
        (plugin.clone(), false, vec![2], SequencePattern::Steps),
        (
            plugin.clone(),
            false,
            vec![1],
            SequencePattern::GuitarArpeggio,
        ),
        (plugin.clone(), true, vec![1], SequencePattern::Steps),
        (
            PluginKey {
                id: "other".into(),
                ..plugin.clone()
            },
            false,
            vec![1],
            SequencePattern::Steps,
        ),
        (
            PluginKey {
                format: "VST3".into(),
                ..plugin.clone()
            },
            false,
            vec![1],
            SequencePattern::Steps,
        ),
    ] {
        library
            .add_automatic(&config, key, effect, &state, pattern)
            .unwrap();
    }
    assert_eq!(library.entries.len(), 6);
    let index = directory(&config).unwrap().join("index.toml");
    std::fs::remove_file(&index).unwrap();
    std::fs::create_dir(&index).unwrap();
    assert!(library
        .add_automatic(&config, plugin.clone(), false, &[3], SequencePattern::Steps)
        .is_err());
    assert_eq!(library.entries.len(), 6);
    assert_eq!(library.state(&config, &newest.id).unwrap(), [1]);
    assert_eq!(
        std::fs::read_dir(directory(&config).unwrap())
            .unwrap()
            .count(),
        7
    );
    std::fs::remove_dir(&index).unwrap();
    let manual = library
        .add(&config, plugin, false, &[1], SequencePattern::Steps)
        .unwrap();
    assert!(!manual.name.starts_with("auto"));
    assert_eq!(manual.id, old.id);
    assert_eq!(manual.name, "My sound");
    assert_eq!(library.entries.len(), 6);
    assert!(state_path(&config, &newest.id).unwrap().exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn favorites_survive_autosave_rename_delete_and_reload() {
    let dir = std::env::temp_dir().join(format!(
        "cat-favorites-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = dir.join("config.toml");
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "test".into(),
        name: "Synth".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/synth.clap".into(),
    };
    let mut library = Library::load(&config).unwrap();
    let first = library
        .add(
            &config,
            plugin.clone(),
            false,
            &[0, 255, 1],
            SequencePattern::GuitarArpeggio,
        )
        .unwrap();
    let second = library
        .add(&config, plugin.clone(), false, &[42], SequencePattern::Off)
        .unwrap();
    crate::state_store::save(&config, &plugin, &[99]).unwrap();
    assert_ne!(first.name, second.name);
    library.rename(&config, &first.id, " Bell ").unwrap();
    assert!(library.rename(&config, &first.id, " ").is_err());
    let mut library = Library::load(&config).unwrap();
    assert_eq!(library.entries[1].name, "Bell");
    assert_eq!(
        library.entries[1].sequence_pattern,
        SequencePattern::GuitarArpeggio
    );
    assert_eq!(library.entries[0].sequence_pattern, SequencePattern::Off);
    assert_eq!(library.state(&config, &first.id).unwrap(), [0, 255, 1]);
    assert!(library.state(&config, "../outside").is_err());
    library.record_use(&config, &first.id).unwrap();
    let mut library = Library::load(&config).unwrap();
    assert_eq!(library.entries[0].id, first.id);
    assert_eq!(library.entries[1].id, second.id);
    library.record_use(&config, &second.id).unwrap();
    assert_eq!(Library::load(&config).unwrap().entries[0].id, second.id);
    // Failed persistence must roll back the order and leave snapshots intact.
    let index = directory(&config).unwrap().join("index.toml");
    let metadata = std::fs::read(&index).unwrap();
    std::fs::remove_file(&index).unwrap();
    std::fs::create_dir(&index).unwrap();
    assert!(library.record_use(&config, &first.id).is_err());
    assert_eq!(library.entries[0].id, second.id);
    assert_eq!(library.entries[1].id, first.id);
    assert_eq!(library.state(&config, &first.id).unwrap(), [0, 255, 1]);
    std::fs::remove_dir(&index).unwrap();
    std::fs::write(&index, metadata).unwrap();
    library.delete(&config, &second.id).unwrap();
    assert_eq!(Library::load(&config).unwrap().entries.len(), 1);
    assert!(!state_path(&config, &second.id).unwrap().exists());
    // A failed index update must keep the in-memory record and its snapshot.
    let index = directory(&config).unwrap().join("index.toml");
    std::fs::remove_file(&index).unwrap();
    std::fs::create_dir(&index).unwrap();
    assert!(library.rename(&config, &first.id, "Lost").is_err());
    assert_eq!(library.entries[0].name, "Bell");
    assert!(library.delete(&config, &first.id).is_err());
    assert_eq!(library.state(&config, &first.id).unwrap(), [0, 255, 1]);
    assert!(library
        .add(&config, plugin, false, &[7], SequencePattern::Steps)
        .is_err());
    assert_eq!(library.entries.len(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn effects_omit_playback_deduplicate_across_patterns_and_remove_only_legacy_effects() {
    let dir = std::env::temp_dir().join(format!(
        "cat-effect-migration-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = dir.join("config.toml");
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "fx".into(),
        name: "Effect".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/fx.clap".into(),
    };
    let mut library = Library::default();
    let effect = library
        .add(&config, plugin.clone(), true, &[1], SequencePattern::Steps)
        .unwrap();
    let same = library
        .add_automatic(&config, plugin.clone(), true, &[1], SequencePattern::Off)
        .unwrap();
    assert_eq!(effect.id, same.id);
    for pattern in [
        SequencePattern::Off,
        SequencePattern::Steps,
        SequencePattern::GuitarArpeggio,
        SequencePattern::Csus4CArpeggio,
        SequencePattern::Fmaj7G6Arpeggio,
    ] {
        assert_eq!(effect.playback_pattern(pattern), pattern);
        let reloaded = Library::load(&config).unwrap();
        assert_eq!(reloaded.entries[0].playback_pattern(pattern), pattern);
    }
    let metadata = toml::to_string(&effect).unwrap();
    for control in [
        "sequence_pattern",
        "sequence_velocity",
        "sequence_modulation",
    ] {
        assert!(!metadata.contains(control));
    }
    assert_eq!(Library::load(&config).unwrap().entries.len(), 1);
    let instrument = library
        .add(
            &config,
            plugin.clone(),
            false,
            &[2],
            SequencePattern::GuitarArpeggio,
        )
        .unwrap();
    let legacy = library
        .add(&config, plugin, true, &[3], SequencePattern::Off)
        .unwrap();
    let index = directory(&config).unwrap().join("index.toml");
    let text = std::fs::read_to_string(&index).unwrap();
    // Simulate the old serializer, which stored a sequence for effect entries.
    let text = text.replace(
        &format!(r#"id = "{}""#, legacy.id),
        &format!(
            r#"sequence_pattern = "steps"
id = "{}""#,
            legacy.id
        ),
    );
    std::fs::write(&index, text).unwrap();
    let migrated = Library::load(&config).unwrap();
    assert_eq!(migrated.entries.len(), 2);
    assert!(!state_path(&config, &legacy.id).unwrap().exists());
    assert_eq!(migrated.state(&config, &effect.id).unwrap(), [1]);
    assert_eq!(migrated.state(&config, &instrument.id).unwrap(), [2]);
    assert_eq!(
        migrated
            .entries
            .iter()
            .find(|f| f.id == instrument.id)
            .unwrap()
            .sequence_pattern,
        SequencePattern::GuitarArpeggio
    );
    assert_eq!(Library::load(&config).unwrap().entries.len(), 2);
    std::fs::remove_dir_all(dir).unwrap();
}
