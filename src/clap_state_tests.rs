//! Opt-in compiled-backend and application preservation regression.
use crate::{config::PluginKey, plugin_list::PluginKind, state_store, App};
use std::path::Path;
use std::time::{Duration, Instant};

fn fixture_key(mode: &str) -> PluginKey {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/shim/Release/clap_state_test_plugin.clap");
    assert!(path.is_file(), "build clap_state_test_plugin first");
    PluginKey {
        format: "CLAP".into(),
        id: format!("cat.test.state.{mode}"),
        name: format!("State {mode}"),
        vendor: "cat test".into(),
        bundle_path: path.to_string_lossy().into_owned(),
    }
}

fn app(path: &Path) -> App {
    let mut app = App::prepare();
    app.config_path = Ok(path.to_owned());
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    app.plugins.clear();
    app.initialize_favorites();
    app
}

fn add(app: &mut App, key: &PluginKey) -> usize {
    let mut plugin = app.host.restore_plugin(key).expect("fixture missing");
    plugin.kind = PluginKind::Instrument;
    app.plugins.push(plugin);
    app.plugins.len() - 1
}

fn wait(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while app.pending.is_some() {
        assert!(Instant::now() < deadline, "creation timeout");
        app.host.pump_startup();
        app.handle_events();
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
#[ignore = "requires compiled CLAP fixture; S03_NATIVE_WORK_DIR points to global evidence directory"]
fn native_clap_state_error_and_favorite_session_preservation() {
    let _library = crate::native_library::load().unwrap();
    let work = std::env::var_os("S03_NATIVE_WORK_DIR").expect("set evidence directory");
    let work = Path::new(&work);
    for mode in ["normal", "context", "none"] {
        let path = work.join(mode).join("config.toml");
        let mut app = app(&path);
        let key = fixture_key(mode);
        let index = add(&mut app, &key);
        app.load_plugin(index);
        wait(&mut app);
        assert_eq!(app.instances.len(), 1, "{}", app.status);
        let id = app.instrument_id().unwrap();
        for value in [1, 0, 2, 3, 1] {
            let result = app.host.load_state(id, &[value]);
            assert_eq!(result.is_ok(), value == 1 && mode != "none");
            if let Err(error) = result {
                assert!(error.contains("CLAP state load"), "{error}");
            }
        }
        if mode == "none" {
            continue;
        }
        // Existing favorite path succeeds, then rejection rolls back the sound
        // without touching the immutable favorite bytes or use order.
        let good = app
            .favorites
            .library
            .add(&path, key.clone(), false, &[4], app.sequence_pattern)
            .unwrap();
        app.load_favorite(&good.id);
        assert!(app.status.contains("Loaded favorite"), "{}", app.status);
        assert_eq!(app.host.save_state(id).unwrap(), [4]);
        let bad = app
            .favorites
            .library
            .add(&path, key.clone(), false, &[0], app.sequence_pattern)
            .unwrap();
        let index_path = path.parent().unwrap().join("favorites/index.toml");
        let before = std::fs::read(&index_path).unwrap();
        app.load_favorite(&bad.id);
        assert!(
            app.status.contains("Could not load favorite"),
            "{}",
            app.status
        );
        assert_eq!(app.instrument_id(), Some(id));
        assert_eq!(app.host.save_state(id).unwrap(), [4]);
        assert_eq!(std::fs::read(&index_path).unwrap(), before);
        assert_eq!(app.favorites.library.state(&path, &bad.id).unwrap(), [0]);
        // A successful session restoration uses the same native load path.
        app.save_session();
        drop(app);
        let mut restored = app_from_saved(&path, &key);
        assert_eq!(
            restored
                .host
                .save_state(restored.instrument_id().unwrap())
                .unwrap(),
            [4]
        );
        // Failed new-slot restoration retains the old live instance and the
        // rejected saved state, and consumes the pending purpose and payload.
        let other = fixture_key(if mode == "normal" {
            "context"
        } else {
            "normal"
        });
        let index = add(&mut restored, &other);
        state_store::save(&path, &other, &[0]).unwrap();
        let previous = restored.instrument_id();
        restored.load_plugin(index);
        wait(&mut restored);
        assert!(
            restored.status.contains("Could not restore"),
            "{}",
            restored.status
        );
        assert_eq!(restored.instrument_id(), previous);
        assert_eq!(restored.host.save_state(previous.unwrap()).unwrap(), [4]);
        assert_eq!(state_store::load(&path, &other).unwrap(), Some(vec![0]));
        assert!(restored.pending.is_none());
    }
}

fn app_from_saved(path: &Path, key: &PluginKey) -> App {
    let mut app = app(path);
    add(&mut app, key);
    app.restore = Some(key.clone());
    app.restore_session();
    wait(&mut app);
    assert_eq!(app.instances.len(), 1, "{}", app.status);
    app
}
