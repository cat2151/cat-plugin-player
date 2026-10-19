//! Native check that opening the editor of a minimal-data-home instance rebuilds it.
use crate::{native_library::process::ROOT, session_load::LoadPurpose, App, SequencePattern};
use std::{
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

const ENV: &str = "SURGE_DATA_HOME";
const CHILD: &str = "CAT_SURGE_EDITOR_REBUILD_CHILD";
const NAME: &str = "native_editor_rebuilds_minimal_surge_with_normal_data_home";

fn settle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.pending.is_some() {
        assert!(Instant::now() < deadline, "load timed out: {}", app.status);
        app.host.pump();
        app.handle_events();
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn peak(app: &App, id: i32) -> f32 {
    let processor = app.host.create_chain(id, &[], false, 48_000, 256).unwrap();
    let mut audio = unsafe { processor.audio_ref() };
    let mut sequencer = crate::seq::Sequencer::new(48_000);
    let mut events = crate::seq::EventBuf::new();
    let mut peak = 0.0f32;
    for _ in 0..100 {
        events.clear();
        sequencer.render(
            SequencePattern::Steps,
            crate::SequenceVelocity::V100,
            256,
            &mut events,
        );
        let mut output = [0.0; 512];
        assert!(audio.process(events.as_slice(), &mut output, 2));
        peak = output.iter().fold(peak, |p, s| p.max(s.abs()));
        app.host.pump_startup();
        std::thread::sleep(Duration::from_secs_f64(256.0 / 48_000.0));
    }
    peak
}

/// One App per process: native host startup state is process-wide.
fn child() {
    let root =
        std::env::temp_dir().join(format!("cat-surge-editor-rebuild-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let test = module_path!().split_once("::").unwrap().1;
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            &format!("{test}::{NAME}"),
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env(ROOT, &root)
        .env_remove("CMRT_SURGE_MINIMAL_DATA_HOME")
        .env_remove(ENV)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stderr.lines().filter(|l| l.starts_with("rebuild:")) {
        eprintln!("{line}");
    }
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        out.status.success() && stdout.contains("1 passed"),
        "child failed\n{stdout}\n{stderr}"
    );
}

#[test]
#[ignore = "requires installed Surge XT CLAP and its factory patches"]
fn native_editor_rebuilds_minimal_surge_with_normal_data_home() {
    if std::env::var_os(CHILD).is_none() {
        child();
        return;
    }
    let _dll = crate::native_library::load().unwrap();
    let before_env = std::env::var_os(ENV);
    let mut app = App::prepare();
    app.output = None;
    let key = crate::selection_host::default_instrument().unwrap();
    // As after `load_plugin_cache`: no deferred scan keeps the actions busy.
    let plugin = app.host.restore_plugin(&key).unwrap();
    app.plugins.push(plugin);
    app.deferred_scan = false;
    app.restore = Some(key.clone());
    app.restore_before_gui();
    settle(&mut app);
    assert_eq!(app.instances.len(), 1, "{}", app.status);
    let old = app.instances[0].id;
    assert!(app.instances[0].minimal_data_home, "{}", app.status);

    // A non-default sound, so that an unrestored new instance would differ.
    let initial = app.host.save_state(old).unwrap();
    let patch = PathBuf::from(std::env::var_os("ProgramData").unwrap())
        .join("Surge XT/patches_factory/Basses/Bass 2.fxp");
    let state = cmrt_patches::prepare_clap_patch_state(&key.id, &patch).unwrap();
    app.host.load_state(old, &state).unwrap();
    // Opening Surge's editor adds editor-only fields (zoom, MSEG view) to its state.
    app.host.show_ui(old).unwrap();
    app.host.hide_ui(old);
    let before = app.host.save_state(old).unwrap();
    assert!(!crate::plugin_specific::same_favorite_state(
        &key, &initial, &before
    ));

    app.favorites.active.push((old, "favorite".into()));
    let started = Instant::now();
    app.open_editor(old);
    assert!(
        matches!(
            app.pending.as_ref().map(|p| &p.purpose),
            Some(LoadPurpose::Editor(id)) if *id == old
        ),
        "{}",
        app.status
    );
    assert!(app.pending.as_ref().unwrap().data_home.is_none());
    assert_eq!(std::env::var_os(ENV), before_env);
    // Pressing again while rebuilding keeps the one pending creation.
    app.open_editor(old);
    assert!(matches!(
        app.pending.as_ref().map(|p| &p.purpose),
        Some(LoadPurpose::Editor(_))
    ));
    settle(&mut app);
    let elapsed = started.elapsed();

    assert_eq!(app.instances.len(), 1, "{}", app.status);
    let new = app.instances[0].id;
    assert_ne!(new, old, "{}", app.status);
    assert!(!app.instances[0].minimal_data_home);
    assert_eq!(std::env::var_os(ENV), before_env);
    assert_eq!(app.favorites.active, [(new, "favorite".to_string())]);
    assert!(app.host.ui_is_visible(new), "{}", app.status);
    app.host.hide_ui(new);
    let after = app.host.save_state(new).unwrap();
    assert!(crate::plugin_specific::same_favorite_state(
        &key, &before, &after
    ));
    let peak = peak(&app, new);
    assert!(peak > 1e-3, "silent chain: {peak}");
    eprintln!(
        "rebuild: {:.1} ms, peak {peak:.3}",
        elapsed.as_secs_f64() * 1000.0
    );
}
