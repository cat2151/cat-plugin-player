//! Real shim/chain acceptance with narrowly injected transaction failures.
use super::*;
use crate::{
    config::PluginKey, plugin_list::PluginKind, random_patch_apply::Operation, state_store,
    SequencePattern,
};
use std::path::{Path, PathBuf};

mod real_surge;

fn key(mode: &str) -> PluginKey {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/shim/Release/clap_state_test_plugin.clap");
    assert!(path.is_file());
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
    app.config_path = Ok(path.into());
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    app.plugins.clear();
    app.initialize_favorites();
    app
}

fn add(app: &mut App, key: &PluginKey) -> usize {
    let mut plugin = app.host.restore_plugin(key).unwrap();
    plugin.kind = PluginKind::Instrument;
    app.plugins.push(plugin);
    app.plugins.len() - 1
}

fn wait(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.pending.is_some() {
        assert!(Instant::now() < deadline);
        app.host.pump_startup();
        app.handle_events();
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn prepared(key: &PluginKey, bytes: &[u8]) -> Prepared {
    let mut candidate = candidate();
    candidate.plugin_id.clone_from(&key.id);
    candidate.plugin_name.clone_from(&key.name);
    Prepared {
        candidate,
        state: bytes.into(),
    }
}

fn files(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn read(root: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        if !root.is_dir() {
            return;
        }
        for item in std::fs::read_dir(root).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                read(&path, out);
            } else {
                out.push((path.clone(), std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut result = Vec::new();
    read(root, &mut result);
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

#[test]
#[ignore = "requires compiled CLAP fixture and S06_NATIVE_WORK_DIR"]
fn native_random_transaction_boundaries_and_preservation() {
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    let normal = key("normal");
    let other = key("context");
    for (case, target, failure, bytes, stopped) in [
        ("backup", &normal, vec![Operation::Backup], vec![5], false),
        ("native-load", &normal, vec![], vec![0], true),
        (
            "current-chain",
            &normal,
            vec![Operation::Chain],
            vec![5],
            false,
        ),
        (
            "rollback",
            &normal,
            vec![Operation::Chain, Operation::Rollback],
            vec![5],
            true,
        ),
        (
            "resume",
            &normal,
            vec![Operation::Apply, Operation::Resume],
            vec![5],
            false,
        ),
        ("creation", &other, vec![Operation::Create], vec![5], true),
        ("replacement-load", &other, vec![], vec![0], false),
        (
            "replacement-chain",
            &other,
            vec![Operation::Chain],
            vec![5],
            true,
        ),
        ("current-success", &normal, vec![], vec![5], true),
        ("replacement-success", &other, vec![], vec![5], false),
        ("replacement-stopped-success", &other, vec![], vec![5], true),
        ("current-playing-success", &normal, vec![], vec![5], false),
    ] {
        let path = root.join(case).join("config.toml");
        let mut app = app(&path);
        let index = add(&mut app, &normal);
        add(&mut app, &other);
        app.load_plugin(index);
        wait(&mut app);
        let id = app.instrument_id().unwrap();
        app.host.load_state(id, &[4]).unwrap();
        let effect = add(&mut app, &key("effect"));
        app.plugins[effect].kind = PluginKind::Effect;
        app.load_plugin(effect);
        wait(&mut app);
        let effects = app.effect_ids();
        assert_eq!(effects.len(), 1);
        let bypass = stopped;
        app.effect_bypassed = bypass;
        app.sequence_pattern = if stopped {
            SequencePattern::Off
        } else {
            SequencePattern::Steps
        };
        app.selected_sequence = SequencePattern::GuitarArpeggio;
        app.sequence_velocity = crate::SequenceVelocity::Range40To100;
        app.sequence_modulation = crate::SequenceModulation::Sweep;
        let velocity = app.sequence_velocity;
        let modulation = app.sequence_modulation;
        app.instances
            .iter_mut()
            .find(|i| i.id == id)
            .unwrap()
            .sweep_cc1 = true;
        app.favorites.active = vec![
            (id, "instrument-selection".into()),
            (effects[0], "effect-selection".into()),
        ];
        app.save_plugin_state(id).unwrap();
        app.save_plugin_state(effects[0]).unwrap();
        app.save_session();
        let before = files(path.parent().unwrap());
        let pattern = app.sequence_pattern;
        let favorites = app.favorites.active.clone();
        app.random_failures = failure;
        // Target lookup must select an instrument identity even when the same
        // CLAP ID is also used as an effect in the connected chain.
        app.apply_random_patch(prepared(target, &bytes));
        wait(&mut app);
        assert!(app.pending.is_none() && !app.random_busy());
        assert_eq!(app.host.save_state(effects[0]).unwrap(), [1]);
        assert_eq!(app.effect_ids(), effects);
        assert_eq!(app.effect_bypassed, bypass);
        assert_eq!(app.sequence_velocity, velocity);
        assert_eq!(app.sequence_modulation, modulation);
        if case.ends_with("success") {
            assert!(
                app.status.contains("Random patch:"),
                "{case}: {}",
                app.status
            );
            let current = app.instrument_id().unwrap();
            assert_eq!(app.host.save_state(current).unwrap(), [5]);
            assert_eq!(
                app.sequence_pattern,
                if stopped {
                    SequencePattern::GuitarArpeggio
                } else {
                    pattern
                }
            );
            assert_eq!(
                app.favorites.active,
                vec![(effects[0], "effect-selection".into())]
            );
            assert_eq!(state_store::load(&path, target).unwrap(), Some(vec![5]));
            if target.id != normal.id {
                assert_eq!(state_store::load(&path, &normal).unwrap(), Some(vec![4]));
            }
        } else {
            assert!(
                app.status.contains("Random patch failed"),
                "{case}: {}",
                app.status
            );
            assert_eq!(app.instrument_id(), Some(id));
            assert_eq!(app.sequence_pattern, pattern);
            assert_eq!(app.favorites.active, favorites);
            assert_eq!(files(path.parent().unwrap()), before);
            assert!(app.instances.iter().find(|i| i.id == id).unwrap().sweep_cc1);
            if case == "rollback" {
                assert!(app.status.contains("rollback failed"));
                assert!(app.save_plugin_state(id).is_err());
                app.save_session();
                assert_eq!(files(path.parent().unwrap()), before);
                drop(app);
                assert_eq!(files(path.parent().unwrap()), before);
                continue;
            }
            assert_eq!(app.host.save_state(id).unwrap(), [4]);
            if case == "resume" {
                assert!(app.status.contains("no audio"));
            }
        }
        // Drop cannot overwrite transaction bytes while no audio device is used.
        drop(app);
    }
    native_preparation_busy_actions_and_applied_save_failure();
    native_rollback_recovery_and_button();
}

fn native_preparation_busy_actions_and_applied_save_failure() {
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    let path = root.join("busy").join("config.toml");
    let mut app = app(&path);
    let normal = key("normal");
    let index = add(&mut app, &normal);
    app.load_plugin(index);
    wait(&mut app);
    let id = app.instrument_id().unwrap();
    app.save_plugin_state(id).unwrap();
    app.save_session();
    let before = files(path.parent().unwrap());
    let (release, wait_worker) = mpsc::channel();
    app.random_patch
        .start_with(candidate(), Default::default(), move |_| {
            wait_worker.recv().unwrap();
            Err("delayed failure".into())
        });
    assert!(app.actions_busy());
    app.load_plugin(index);
    app.start_scan(true);
    app.remove_plugin(id);
    app.add_favorite(id);
    app.load_favorite("missing");
    app.set_effect_bypass(true);
    assert!(!app.reorder_effect(1, 2, true));
    assert!(app.pending.is_none() && !app.scanning);
    assert_eq!(app.instrument_id(), Some(id));
    assert_eq!(files(path.parent().unwrap()), before);
    let ctx = egui::Context::default();
    let old = app.sequence_pattern;
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.sequence_controls(ui));
        },
    );
    assert_eq!(app.sequence_pattern, old);
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.random_busy() {
        app.poll_random_patch();
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(app.status.contains("delayed failure"));
    // Failed persistence after native commit must retain the applied sound.
    app.config_path = Err("injected saving destination failure".into());
    app.apply_random_patch(prepared(&normal, &[5]));
    assert!(
        app.status.contains("applied, saving failed"),
        "{}",
        app.status
    );
    assert_eq!(app.host.save_state(id).unwrap(), [5]);
    assert_eq!(files(path.parent().unwrap()), before);
}

fn native_rollback_recovery_and_button() {
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    let path = root.join("recovery").join("config.toml");
    let mut app = app(&path);
    let normal = key("normal");
    let index = add(&mut app, &normal);
    app.load_plugin(index);
    wait(&mut app);
    let id = app.instrument_id().unwrap();
    app.random_failures = vec![Operation::Chain, Operation::Rollback];
    app.apply_random_patch(prepared(&normal, &[5]));
    assert!(app.unsafe_state.contains(&id));
    app.random_failures.clear();
    app.load_plugin(index);
    wait(&mut app);
    assert_ne!(app.instrument_id(), Some(id));
    assert!(app.unsafe_state.is_empty());
    app.apply_random_patch(prepared(&normal, &[6]));
    assert!(app.status.contains("Random patch:"));
    assert_eq!(state_store::load(&path, &normal).unwrap(), Some(vec![6]));
    // A true Sequence-pane click starts preparation. A second click is disabled
    // while the owned receiver exists, even before the GUI consumes its result.
    app.random_patch_catalog = crate::random_patch_catalog::Catalog::from_candidates(
        vec![prepared(&normal, &[1]).candidate],
        &app.plugins,
    );
    let ctx = egui::Context::default();
    let output = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.sequence_controls(ui));
    });
    let position = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.text() == "Random patch" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .expect("Random patch button missing");
    let click = || egui::RawInput {
        events: vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let _ = ctx.run(click(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.sequence_controls(ui));
    });
    assert!(app.random_busy());
    let status = app.status.clone();
    let _ = ctx.run(click(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.sequence_controls(ui));
    });
    assert_eq!(app.status, status);
    assert!(app.random_busy());
    // A scan-list change after preparation cannot apply a stale CLAP identity.
    app.random_patch = Default::default();
    app.plugins[index].format = "VST3".into();
    let previous = app.instrument_id();
    app.apply_random_patch(prepared(&normal, &[5]));
    assert!(app.status.contains("no longer available"));
    assert_eq!(app.instrument_id(), previous);
}
