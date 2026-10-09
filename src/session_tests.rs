use super::*;

// Opt-in: needs the native DLL and an installed plugin from existing history.
// All writes go to a temporary session directory; user history is read only.
#[test]
#[ignore = "requires an installed instrument and the native shim"]
fn native_session_saves_on_replace_remove_and_exit_then_restores() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    let key = app.restore.take().expect("no plugin in existing history");
    let directory = std::env::temp_dir().join(format!(
        "cat-session-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = directory.join("config.toml");
    app.config_path = Ok(path.clone());
    app.restore_effect = Default::default();
    app.output = None; // No sound or audio-device dependency in this check.
    app.deferred_scan = false;
    app.plugins = vec![app.host.restore_plugin(&key).expect("plugin missing")];
    let load = |app: &mut App| {
        app.load_plugin(0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while app.pending.is_some() {
            assert!(std::time::Instant::now() < deadline, "creation timed out");
            app.host.pump_startup();
            app.handle_events();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(app.instances.len(), 1, "{}", app.status);
    };
    load(&mut app);
    let first = app.instances[0].id;
    assert!(app.host.save_state(i32::MAX).is_err());
    let vaporizer = key.format == "CLAP" && key.id == "com.vastdynamics.VAST2";
    // Vaporizer2's JUCE state is XML wrapped in a binary header. Change the
    // tuning without changing the payload length, then check the actual
    // parameter after reinstantiation (serialization can normalize UI data).
    let changed_tuning = b"id=\"m_fMasterTune\" text=\"432\"";
    if vaporizer {
        let mut state = app.host.save_state(first).unwrap();
        let original = b"id=\"m_fMasterTune\" text=\"440\"";
        let offset = state
            .windows(original.len())
            .position(|bytes| bytes == original)
            .expect("Vaporizer2 state layout changed");
        state[offset..offset + original.len()].copy_from_slice(changed_tuning);
        app.host.load_state(first, &state).unwrap();
    }
    let baseline = app.host.save_state(first).unwrap();
    assert!(!baseline.is_empty(), "instrument has no meaningful state");
    load(&mut app); // Auto-remove saves the previous instance first.
    assert_ne!(app.instances[0].id, first);
    assert!(state_store::load(&path, &key).unwrap() == Some(baseline));
    let second = app.instances[0].id;
    let restored = app.host.save_state(second).unwrap();
    if vaporizer {
        assert!(
            restored
                .windows(changed_tuning.len())
                .any(|bytes| bytes == changed_tuning),
            "changed tuning was not restored"
        );
    }
    let blocker = directory.join("blocked");
    std::fs::write(&blocker, b"not a directory").unwrap();
    app.config_path = Ok(blocker.join("config.toml"));
    assert!(
        !app.remove_plugin(second),
        "failed save must keep the instance"
    );
    assert_eq!(app.instances[0].id, second);
    app.config_path = Ok(path.clone());
    assert!(app.remove_plugin(second));
    assert!(app.instances.is_empty());
    assert!(state_store::load(&path, &key).unwrap() == Some(restored));
    load(&mut app); // Restore the saved snapshot before starting audio.
    let live = app.host.save_state(app.instances[0].id).unwrap();
    if vaporizer {
        assert!(live
            .windows(changed_tuning.len())
            .any(|bytes| bytes == changed_tuning));
    }
    state_store::save(&path, &key, b"stale snapshot").unwrap();
    app.save_on_shutdown(); // Save while the main window would still be visible.
    assert_eq!(state_store::load(&path, &key).unwrap(), Some(live.clone()));
    // A second close notification and Drop must not recapture state.
    state_store::save(&path, &key, b"already saved").unwrap();
    app.save_on_shutdown();
    drop(app);
    assert_eq!(
        state_store::load(&path, &key).unwrap(),
        Some(b"already saved".to_vec())
    );
    state_store::save(&path, &key, &live).unwrap();
    assert_eq!(
        crate::status::Status::load(&path).unwrap().last_played,
        Some(key.clone())
    );
    let mut next = App::prepare();
    next.config_path = Ok(path.clone());
    next.restore = crate::status::Status::load(&path).unwrap().last_played;
    next.restore_effect = Default::default();
    next.output = None;
    next.deferred_scan = false;
    next.restore_before_gui();
    assert_eq!(next.instances.len(), 1, "{}", next.status);
    if vaporizer {
        let state = next.host.save_state(next.instances[0].id).unwrap();
        assert!(
            state
                .windows(changed_tuning.len())
                .any(|bytes| bytes == changed_tuning),
            "changed tuning was not restored on next startup"
        );
    }
    drop(next);
    std::fs::remove_dir_all(directory).unwrap();
}
