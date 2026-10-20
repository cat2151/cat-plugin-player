//! Explicit real-user-data verification. Parent backs up and restores every touched file.
use super::{catalog_probe::CatalogProbe, editor_rebuild_tests::settle};
use crate::{config, native_library::process::ROOT, App};
use std::{path::Path, process::Command};

const PHASE: &str = "CAT_SURGE_USER_SESSION_PHASE";
const WORK: &str = "CAT_SURGE_USER_SESSION_WORK";
const NAME: &str = "native_surge_user_session_survives_editor_and_restart";

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

#[test]
#[ignore = "touches real current Surge session; requires CAT_SURGE_USER_SESSION_WORK, no other player; backs up/restores, memory-only audio"]
fn native_surge_user_session_survives_editor_and_restart() {
    let Ok(phase) = std::env::var(PHASE) else {
        assert!(
            std::env::var_os(ROOT).is_none(),
            "real-data test must not be isolated"
        );
        let work = std::path::PathBuf::from(
            std::env::var_os(WORK).expect("set explicit external work directory"),
        )
        .join(format!("real-session-{}", std::process::id()));
        std::fs::create_dir(&work).unwrap();
        let root = config::path().unwrap().parent().unwrap().to_path_buf();
        let backup = work.join("backup");
        std::fs::create_dir(&backup).unwrap();
        let mut files = Vec::new();
        for name in ["config.toml", "status.json", "states", "favorites"] {
            let source = root.join(name);
            if source.is_dir() {
                copy_tree(&source, &backup.join(name));
            } else if source.is_file() {
                std::fs::copy(&source, backup.join(name)).unwrap();
            } else {
                continue;
            }
            files.push(name);
        }
        let name = format!("{}::{NAME}", module_path!().split_once("::").unwrap().1);
        let mut results = Vec::new();
        for phase in ["editor", "restart"] {
            let out = Command::new(std::env::current_exe().unwrap())
                .args([&name, "--exact", "--ignored", "--nocapture"])
                .env(PHASE, phase)
                .env(WORK, &work)
                .env_remove(ROOT)
                .env_remove("SURGE_DATA_HOME")
                .env_remove("CMRT_SURGE_MINIMAL_DATA_HOME")
                .output()
                .unwrap();
            std::fs::write(work.join(format!("{phase}.log")), &out.stderr).unwrap();
            let passed = out.status.success();
            results.push((
                phase,
                passed,
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ));
            if !passed {
                break;
            }
        }
        // Restore even when a child assertion failed; no user plugin state is discarded.
        for name in files {
            if backup.join(name).is_dir() {
                copy_tree(&backup.join(name), &root.join(name));
            } else {
                std::fs::copy(backup.join(name), root.join(name)).unwrap();
            }
        }
        for (phase, passed, errors) in results {
            assert!(passed, "{phase}: {errors}");
        }
        eprintln!("user-session: normal startup -> Show UI -> graceful save -> fresh process restored; originals restored; evidence {}", work.display());
        return;
    };
    let _dll = crate::native_library::load().unwrap();
    let work = std::path::PathBuf::from(std::env::var_os(WORK).unwrap());
    let mut app = App::prepare();
    app.output = None;
    app.load_plugin_cache();
    app.restore_before_gui();
    settle(&mut app);
    let id = app
        .instrument_id()
        .expect("current user instrument required");
    let key = app.instances[0].plugin.clone();
    assert!(
        super::super::matches_plugin(&key),
        "current instrument must be Surge XT"
    );
    assert!(app.instances[0].minimal_data_home);
    let original = crate::state_store::load(app.config_path.as_ref().unwrap(), &key)
        .unwrap()
        .unwrap();
    let sound = app.host.save_state(id).unwrap();
    assert!(crate::plugin_specific::same_favorite_state(
        &key, &original, &sound
    ));
    if phase == "editor" {
        app.host.show_ui(id).unwrap();
        app.host.hide_ui(id);
        let before = app.host.save_state(id).unwrap();
        let favorites = app.favorites.active.clone();
        let effects = app.effect_ids();
        let playback = (
            app.sequence_pattern,
            app.selected_sequence,
            app.sequence_velocity,
            app.sequence_modulation,
            app.mml_input.confirmed.clone(),
        );
        let probe = CatalogProbe::start();
        app.open_editor(id);
        settle(&mut app);
        probe.assert_normal();
        let new = app.instrument_id().unwrap();
        app.host.hide_ui(new);
        let after = app.host.save_state(new).unwrap();
        assert!(crate::plugin_specific::same_favorite_state(
            &key, &before, &after
        ));
        let expected: Vec<_> = favorites
            .into_iter()
            .map(|(old, name)| (if old == id { new } else { old }, name))
            .collect();
        assert_eq!(app.favorites.active, expected);
        assert_eq!(app.effect_ids(), effects);
        assert_eq!(
            (
                app.sequence_pattern,
                app.selected_sequence,
                app.sequence_velocity,
                app.sequence_modulation,
                app.mml_input.confirmed.clone()
            ),
            playback
        );
        std::fs::write(work.join("expected-state.bin"), &after).unwrap();
    } else {
        // Match the editor-initialization state used for the first comparison.
        app.host.show_ui(id).unwrap();
        app.host.hide_ui(id);
        let expected = std::fs::read(work.join("expected-state.bin")).unwrap();
        assert!(crate::plugin_specific::same_favorite_state(
            &key,
            &expected,
            &app.host.save_state(id).unwrap()
        ));
    }
    app.save_on_shutdown();
}
