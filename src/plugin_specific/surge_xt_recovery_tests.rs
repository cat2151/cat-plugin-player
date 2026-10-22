//! Failure contracts for the destructive minimal-to-normal DLL transition.
use super::editor_rebuild_tests::{edited_state, peak, settle};
use crate::{native_library::process::ROOT, plugin_list::PluginKind, App};
use std::{path::PathBuf, process::Command};

const CASE: &str = "CAT_SURGE_RECOVERY_CASE";
const NAME: &str = "native_surge_editor_rebuild_recovers_failures";

#[test]
#[ignore = "requires installed Surge XT CLAP and Effects; briefly opens editors, memory-only audio"]
fn native_surge_editor_rebuild_recovers_failures() {
    let Ok(case) = std::env::var(CASE) else {
        for case in [
            "save",
            "create",
            "state",
            "chain",
            "double-create",
            "double-state",
            "double-chain",
        ] {
            let root = std::env::temp_dir()
                .join(format!("cat-surge-recovery-{}-{case}", std::process::id()));
            std::fs::create_dir_all(&root).unwrap();
            let name = format!("{}::{NAME}", module_path!().split_once("::").unwrap().1);
            let out = Command::new(std::env::current_exe().unwrap())
                .args([&name, "--exact", "--ignored", "--nocapture"])
                .env(CASE, case)
                .env(ROOT, &root)
                .env_remove("SURGE_DATA_HOME")
                .env_remove("CMRT_SURGE_MINIMAL_DATA_HOME")
                .output()
                .unwrap();
            let _ = std::fs::remove_dir_all(root);
            assert!(
                out.status.success(),
                "{case}: {}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            eprintln!("recovery: {case} passed");
        }
        return;
    };
    let _dll = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    let key = crate::selection_host::default_instrument().unwrap();
    app.plugins.push(app.host.restore_plugin(&key).unwrap());
    app.deferred_scan = false;
    app.restore = Some(key.clone());
    app.restore_before_gui();
    settle(&mut app);
    let old = app.instrument_id().unwrap();
    let patch = PathBuf::from(std::env::var_os("ProgramData").unwrap())
        .join("Surge XT/patches_factory/Basses/Bass 2.fxp");
    app.host
        .load_state(
            old,
            &cmrt_patches::prepare_clap_patch_state(&key.id, &patch).unwrap(),
        )
        .unwrap();
    app.host.show_ui(old).unwrap();
    app.host.hide_ui(old);
    app.host
        .load_state(
            old,
            &edited_state(&app.host.save_state(old).unwrap(), false),
        )
        .unwrap();
    let before = app.host.save_state(old).unwrap();
    app.favorites.active.push((old, "kept-favorite".into()));
    app.sequence_velocity = crate::SequenceVelocity::V127;
    app.sequence_modulation = Some(crate::SequenceModulation::Fixed(127));

    // A different DLL must already use the normal catalogs while the source is minimal.
    let mut effect = key.clone();
    effect.id = "org.surge-synth-team.surge-xt-fx".into();
    effect.bundle_path = effect
        .bundle_path
        .replace("Surge XT.clap", "Surge XT Effects.clap");
    let mut info = app.host.restore_plugin(&effect).unwrap();
    info.kind = PluginKind::Effect;
    app.plugins.push(info);
    {
        let catalog = super::catalog_probe::CatalogProbe::start();
        app.load_plugin(1);
        settle(&mut app);
        let paths = catalog.paths();
        assert!(paths.iter().any(|p| p.contains("/fx_presets/Airwindows/")));
        assert!(!paths.iter().any(|p| p.contains("surge-data-min")));
    }
    let fx = app.effect_ids();
    assert_eq!(fx.len(), 1, "{}", app.status);
    let effect_state = app.host.save_state(fx[0]).unwrap();
    app.effect_bypassed = true;
    let point = case.strip_prefix("double-").unwrap_or(&case);
    crate::session_editor::FAILURES.with(|f| {
        let mut f = f.borrow_mut();
        let point = match point {
            "save" => "save",
            "create" => "create",
            "state" => "state",
            "chain" => "chain",
            _ => unreachable!(),
        };
        f.push(point);
        if case.starts_with("double-") {
            f.push(point);
        }
    });
    app.open_editor(old);
    settle(&mut app);
    assert_eq!(std::env::var_os("SURGE_DATA_HOME"), None);
    assert_eq!(app.effect_ids(), fx);
    assert!(app.effect_bypassed);
    if case.starts_with("double-") {
        assert!(app.instrument_id().is_none());
        assert!(
            app.status.contains("recovery also failed"),
            "{}",
            app.status
        );
        let path = app.config_path.as_ref().unwrap().clone();
        let session = std::fs::read(path.with_file_name("status.json")).unwrap();
        let saved = crate::state_store::load(&path, &key).unwrap().unwrap();
        assert!(crate::plugin_specific::same_favorite_state(
            &key, &before, &saved
        ));
        app.save_session();
        assert_eq!(
            std::fs::read(path.with_file_name("status.json")).unwrap(),
            session
        );
        assert!(app.editor_rebuild.is_some());
        app.load_plugin(0);
        settle(&mut app);
    } else if case == "save" {
        assert_eq!(app.instrument_id(), Some(old));
        assert!(app.instances[0].minimal_data_home);
        app.open_editor(old);
        settle(&mut app);
    } else {
        assert!(
            app.status.contains("saved sound recovered"),
            "{}",
            app.status
        );
    }
    let new = app.instrument_id().unwrap();
    assert_eq!(app.instances.len(), 2);
    assert!(app.editor_rebuild.is_none());
    assert_eq!(app.favorites.active, [(new, "kept-favorite".to_string())]);
    assert_eq!(app.sequence_velocity, crate::SequenceVelocity::V127);
    assert_eq!(
        app.sequence_modulation,
        Some(crate::SequenceModulation::Fixed(127))
    );
    assert_eq!(app.host.save_state(fx[0]).unwrap(), effect_state);
    app.open_editor(new);
    app.host.hide_ui(new);
    assert!(crate::plugin_specific::same_favorite_state(
        &key,
        &before,
        &app.host.save_state(new).unwrap()
    ));
    assert!(peak(&app, new) > 1e-3);
    crate::session_editor::FAILURES.with(|f| assert!(f.borrow().is_empty()));
}
