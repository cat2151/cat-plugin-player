use super::*;

#[test]
#[ignore = "requires compiled CLAP fixture and isolated S06_NATIVE_WORK_DIR"]
fn native_browser_preset_names_capture_recall_restart_and_failure() {
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    let path = root.join("preset-names/config.toml");
    let normal = key("normal");
    let mut app = app(&path);
    let index = add(&mut app, &normal);
    app.load_plugin(index);
    wait(&mut app);
    let mut patch = prepared(&normal, &[5]);
    patch.candidate.display = "Warm Pad".into();
    app.apply_patch(patch);
    let source = app.instrument_id().unwrap();
    app.add_favorite(source);
    let favorite = app.favorites.library.entries[0].clone();
    assert_eq!(favorite.preset_name.as_deref(), Some("Warm Pad"));
    let mut failed = prepared(&normal, &[0]);
    failed.candidate.display = "Failed".into();
    app.apply_patch(failed);
    assert!(app.status.contains("Browse patch failed"));
    assert_eq!(app.instances[0].preset_name.as_deref(), Some("Warm Pad"));
    let mut next = prepared(&normal, &[6]);
    next.candidate.display = "Other".into();
    app.apply_patch(next);
    app.load_favorite(&favorite.id);
    assert_eq!(app.instances[0].preset_name, favorite.preset_name);

    let effect = key("effect");
    let effect_index = add(&mut app, &effect);
    app.plugins[effect_index].kind = PluginKind::Effect;
    let candidate = crate::effect_catalog::Candidate {
        plugin_id: effect.id,
        plugin_name: effect.name,
        preset: cmrt_core::audio_effect::PresetLocation {
            path: "X:/presets/Hall".into(),
            value: "Hall".into(),
            display: "Large Hall".into(),
        },
        name: "Large Hall".into(),
        category: "Reverb".into(),
        kind: "Hall".into(),
        state: None,
    };
    app.apply_effect(crate::effect_preparation::Prepared {
        candidate: candidate.clone(),
        target: None,
        state: vec![7],
    });
    wait(&mut app);
    let effect_id = app.effect_id().unwrap();
    app.add_favorite(effect_id);
    let saved_effect = app.favorites.library.entries[0].clone();
    assert_eq!(saved_effect.preset_name.as_deref(), Some("Large Hall"));
    app.apply_effect(crate::effect_preparation::Prepared {
        candidate,
        target: Some(effect_id),
        state: vec![0],
    });
    wait(&mut app);
    assert!(app.status.contains("Browse effect failed"));
    assert_eq!(app.effect_id(), Some(effect_id));
    assert_eq!(app.instances[1].preset_name, saved_effect.preset_name);
    // A regular plugin reload restores the name alongside its saved state.
    app.save_plugin_state(source).unwrap();
    app.save_plugin_state(effect_id).unwrap();
    app.load_plugin(index);
    wait(&mut app);
    assert_eq!(app.instances[0].preset_name, favorite.preset_name);
    let status = crate::status::Status::load(&path).unwrap();
    drop(app);
    let mut restored = super::app(&path);
    restored.restore = status.last_played;
    restored.restore_effect = status.effects.into();
    restored.favorites.restore = status.favorites;
    restored.restore_before_gui();
    wait(&mut restored);
    assert_eq!(restored.instances[0].preset_name, favorite.preset_name);
    assert_eq!(restored.instances[1].preset_name, saved_effect.preset_name);
    assert!(restored
        .favorites
        .library
        .entries
        .iter()
        .any(|f| f.matches_filter("large hall")));
}
