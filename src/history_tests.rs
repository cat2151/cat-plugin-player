use super::*;
use crate::{config::PluginKey, favorites_store::Library, sequence_pattern::SequencePattern};

#[test]
fn compact_age_boundaries() {
    for (seconds, label) in [
        (0, "1s"),
        (1, "1s"),
        (59, "59s"),
        (60, "1m"),
        (3600, "1h"),
        (86400, "1d"),
        (604800, "1w"),
        (2592000, "1mon"),
        (31536000, "1y"),
    ] {
        assert_eq!(age(100, 100 + seconds), label);
    }
    assert_eq!(age(200, 100), "1s");
    assert_eq!(age(0, 100), "?");
}

#[test]
fn history_registration_refreshes_timestamp_without_replacing_snapshot() {
    let dir = std::env::temp_dir().join(format!(
        "cat-history-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = dir.join("config.toml");
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "synth".into(),
        name: "Synth".into(),
        vendor: String::new(),
        bundle_path: "X:/synth.clap".into(),
    };
    let mut library = Library::default();
    let first = library
        .add_automatic(&config, plugin.clone(), false, &[1], SequencePattern::Steps)
        .unwrap();
    assert!(first.history && !first.favorite);
    library.entries[0].registered_at = 1;
    library
        .add_automatic(&config, plugin.clone(), false, &[2], SequencePattern::Steps)
        .unwrap();
    let saved = library
        .add_automatic(&config, plugin.clone(), false, &[1], SequencePattern::Steps)
        .unwrap();
    assert_eq!(saved.id, first.id);
    assert_eq!(library.entries.len(), 2);
    assert!(saved.registered_at > 1);
    assert_eq!(library.entries[1].id, first.id);
    assert_eq!(library.state(&config, &first.id).unwrap(), [1]);
    let saved = library
        .add(&config, plugin.clone(), false, &[1], SequencePattern::Steps)
        .unwrap();
    assert!(saved.history && saved.favorite);
    assert_eq!(saved.id, first.id);
    let index = dir.join("favorites/index.toml");
    let before = library
        .entries
        .iter()
        .map(|entry| (entry.id.clone(), entry.registered_at))
        .collect::<Vec<_>>();
    let text = std::fs::read_to_string(&index).unwrap();
    std::fs::remove_file(&index).unwrap();
    std::fs::create_dir(&index).unwrap();
    assert!(library
        .add_automatic(&config, plugin.clone(), false, &[2], SequencePattern::Steps)
        .is_err());
    assert_eq!(
        library
            .entries
            .iter()
            .map(|entry| (entry.id.clone(), entry.registered_at))
            .collect::<Vec<_>>(),
        before
    );
    std::fs::remove_dir(&index).unwrap();
    std::fs::write(&index, &text).unwrap();
    let mut metadata: toml::Value = toml::from_str(&text).unwrap();
    let entries = metadata.get_mut("entries").unwrap().as_array_mut().unwrap();
    for entry in entries {
        let table = entry.as_table_mut().unwrap();
        table.remove("history");
        table.remove("favorite");
        table.remove("registered_at");
        let renamed = table.get("id").unwrap().as_str().unwrap() == first.id;
        table.insert(
            "name".into(),
            toml::Value::String(if renamed { "My sound" } else { "auto Synth 1" }.into()),
        );
        if !renamed {
            table.insert("effect".into(), toml::Value::Boolean(true));
            table.insert(
                "sequence_pattern".into(),
                toml::Value::String("steps".into()),
            );
        }
    }
    std::fs::write(&index, toml::to_string(&metadata).unwrap()).unwrap();
    let migrated = Library::load(&config).unwrap();
    assert!(migrated
        .entries
        .iter()
        .all(|entry| entry.history && entry.registered_at > 0));
    assert_eq!(migrated.entries.len(), 2);
    assert!(
        migrated
            .entries
            .iter()
            .find(|entry| entry.id == first.id)
            .unwrap()
            .favorite
    );
    assert!(
        !migrated
            .entries
            .iter()
            .find(|entry| entry.id != first.id)
            .unwrap()
            .favorite
    );
    assert_eq!(
        migrated
            .state(
                &config,
                &migrated
                    .entries
                    .iter()
                    .find(|entry| entry.id != first.id)
                    .unwrap()
                    .id
            )
            .unwrap(),
        [2]
    );
    let once = std::fs::read(&index).unwrap();
    Library::load(&config).unwrap();
    assert_eq!(std::fs::read(&index).unwrap(), once);
    std::fs::remove_dir_all(dir).unwrap();
}
