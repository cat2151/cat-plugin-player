//! Retained targets are replacements, never rollback destinations for another synth.
use super::*;

#[test]
#[ignore = "requires compiled CLAP fixture and S06_NATIVE_WORK_DIR; no device"]
fn native_retained_round_trips_and_failure_preservation() {
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    let path = root.join("retention/config.toml");
    let mut app = app(&path);
    let a = key("normal");
    let b = key("context");
    // The fixture's stereo effect can also serve as a synthetic source. Its
    // duplicate scan entry below exercises kind separation with the same ID.
    let c = key("effect2");
    for key in [&a, &b, &c] {
        add(&mut app, key);
    }
    app.load_plugin(0);
    wait(&mut app);
    let first = app.instrument_id().unwrap();
    let fx = add(&mut app, &c);
    app.plugins[fx].kind = PluginKind::Effect;
    app.load_plugin(fx);
    wait(&mut app);
    let effects = app.effect_ids();
    let mut ids = std::collections::HashMap::from([(a.id.clone(), first)]);
    for (target, value) in [
        (&b, 4),
        (&c, 5),
        (&a, 6),
        (&b, 7),
        (&c, 8),
        (&a, 9),
        (&b, 10),
    ] {
        app.apply_patch(prepared(target, &[value]));
        wait(&mut app);
        assert!(app.status.contains("Browse patch:"), "{}", app.status);
        let id = app.instrument_id().unwrap();
        assert_eq!(*ids.entry(target.id.clone()).or_insert(id), id);
        assert_eq!(app.host.save_state(id).unwrap(), [value]);
        assert_eq!(app.instances.len(), 2);
        assert_eq!(app.effect_ids(), effects);
        assert_eq!(app.host.save_state(effects[0]).unwrap(), [1]);
        assert!(app.retained_instruments.len() <= 2);
        assert!(app
            .retained_instruments
            .iter()
            .all(|r| r.instance.voice.is_none() && !app.host.ui_is_visible(r.instance.id)));
        assert_eq!(
            crate::status::Status::load(&path)
                .unwrap()
                .last_played
                .unwrap()
                .id,
            target.id
        );
    }
    assert_eq!(app.retained_instruments.len(), 2);
    // Scan ordering can change without changing an instrument's identity.
    app.plugins.swap(0, 1);
    app.reconcile_retained();
    assert_eq!(app.retained_instruments.len(), 2);
    // An effect request with the same CLAP ID must not evict its dormant source.
    app.load_plugin(fx);
    assert!(app.pending.is_none());
    assert_eq!(app.retained_instruments.len(), 2);
    let current = app.instrument_id().unwrap();
    for failure in [Operation::Apply, Operation::Chain] {
        // Ensure A is retained before each failure trial.
        if !app
            .retained_instruments
            .iter()
            .any(|r| r.instance.plugin.id == a.id)
        {
            app.apply_patch(prepared(&a, &[11]));
            wait(&mut app);
            app.apply_patch(prepared(&b, &[10]));
            wait(&mut app);
        }
        let before = files(path.parent().unwrap());
        app.patch_failures = vec![failure, Operation::Rollback];
        app.apply_patch(prepared(&a, &[12]));
        assert!(app.status.contains("Browse patch failed"));
        assert_eq!(app.instrument_id(), Some(current));
        assert_eq!(app.host.save_state(current).unwrap(), [10]);
        assert!(app.unsafe_state.is_empty());
        assert!(!app
            .retained_instruments
            .iter()
            .any(|r| r.instance.plugin.id == a.id));
        assert_eq!(files(path.parent().unwrap()), before);
        app.patch_failures.clear();
        app.save_plugin_state(current).unwrap();
        app.save_session();
    }
    // Manual load destroys only its conflicting retained identity.
    let c_id = ids[&c.id];
    app.load_plugin(2);
    wait(&mut app);
    assert_ne!(app.instrument_id(), Some(c_id));
    assert!(!app
        .retained_instruments
        .iter()
        .any(|r| r.instance.plugin.id == c.id));
    // Scan removal invalidates an inactive instance, without touching live slots.
    app.apply_patch(prepared(&a, &[13]));
    wait(&mut app);
    let live = app.instrument_id();
    app.plugins.retain(|p| p.id != c.id);
    app.reconcile_retained();
    assert_eq!(app.instrument_id(), live);
    assert!(!app
        .retained_instruments
        .iter()
        .any(|r| r.instance.plugin.id == c.id));
    app.clear_retained();
    assert!(app.retained_instruments.is_empty());
}
