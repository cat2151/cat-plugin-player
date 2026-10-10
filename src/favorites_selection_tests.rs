use super::*;

#[test]
fn restored_selection_requires_the_saved_slot_and_plugin_identity() {
    let plugin = crate::config::PluginKey {
        format: "CLAP".into(),
        id: "synth".into(),
        name: "New name".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/synth.clap".into(),
    };
    let instrument = Favorite {
        id: "instrument".into(),
        name: "Renamed favorite".into(),
        plugin: plugin.clone(),
        effect: false,
        history: false,
        favorite: true,
        registered_at: 0,
        sequence_pattern: Default::default(),
        selected_sequence: None,
        mml: None,
    };
    let effect = Favorite {
        id: "effect".into(),
        effect: true,
        ..instrument.clone()
    };
    let mut favorites = Favorites::default();
    favorites.library.entries = vec![instrument, effect];
    favorites.restore = crate::status::FavoriteSelection {
        instrument: Some("instrument".into()),
        effects: vec!["effect".into()],
        ..Default::default()
    };
    assert_eq!(
        favorites
            .take_restored(PluginKind::Instrument, &plugin)
            .unwrap()
            .name,
        "Renamed favorite"
    );
    assert_eq!(
        favorites
            .take_restored(PluginKind::Effect, &plugin)
            .unwrap()
            .id,
        "effect"
    );
    assert!(favorites
        .take_restored(PluginKind::Effect, &plugin)
        .is_none());
    for invalid in ["deleted", "effect"] {
        favorites.restore.instrument = Some(invalid.into());
        assert!(favorites
            .take_restored(PluginKind::Instrument, &plugin)
            .is_none());
    }
    for other in [
        crate::config::PluginKey {
            format: "VST3".into(),
            ..plugin.clone()
        },
        crate::config::PluginKey {
            id: "other".into(),
            ..plugin.clone()
        },
    ] {
        favorites.restore.instrument = Some("instrument".into());
        assert!(favorites
            .take_restored(PluginKind::Instrument, &other)
            .is_none());
    }
}
