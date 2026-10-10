use super::*;

#[test]
fn different_mml_snapshots_survive_stop_dedup_and_effects_omit_playback() {
    let dir = std::env::temp_dir().join(format!("cat-mml-favorites-{}", std::process::id()));
    let config = dir.join("config.toml");
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "mml-test".into(),
        name: "MML".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/test.clap".into(),
    };
    let mut library = Library::default();
    let capture = |text: &str, pattern| FavoriteCapture {
        sequence_pattern: pattern,
        selected_sequence: Some(SequencePattern::Custom),
        mml: Some(text.into()),
        sweep_cc1: false,
    };
    let first = library
        .add(
            &config,
            plugin.clone(),
            false,
            &[1],
            capture("C F G", SequencePattern::Custom),
        )
        .unwrap();
    let stopped = library
        .add_automatic(
            &config,
            plugin.clone(),
            false,
            &[1],
            capture("C F G", SequencePattern::Off),
        )
        .unwrap();
    assert_eq!(first.id, stopped.id);
    let second = library
        .add_automatic(
            &config,
            plugin.clone(),
            false,
            &[1],
            capture("o5 l8 c", SequencePattern::Off),
        )
        .unwrap();
    assert_ne!(first.id, second.id);
    let effect = library
        .add(
            &config,
            plugin,
            true,
            &[1],
            capture("C F G", SequencePattern::Custom),
        )
        .unwrap();
    assert!(effect.mml.is_none());
    assert!(effect.selected_sequence.is_none());
    let loaded = Library::load(&config).unwrap();
    assert_eq!(
        loaded
            .entries
            .iter()
            .find(|f| f.id == first.id)
            .unwrap()
            .mml
            .as_deref(),
        Some("C F G")
    );
    assert_eq!(
        loaded
            .entries
            .iter()
            .find(|f| f.id == second.id)
            .unwrap()
            .selected_sequence,
        Some(SequencePattern::Custom)
    );
    let text = std::fs::read_to_string(directory(&config).unwrap().join("index.toml")).unwrap();
    let metadata: toml::Value = toml::from_str(&text).unwrap();
    let fx = metadata["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["effect"].as_bool() == Some(true))
        .unwrap();
    assert!(fx.get("mml").is_none() && fx.get("selected_sequence").is_none());
    std::fs::remove_dir_all(dir).unwrap();
}
