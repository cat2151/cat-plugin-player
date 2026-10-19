//! Native checks that only the startup restoration sees the minimal data home.
use crate::{native_library::process::ROOT, App};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

const ENV: &str = "SURGE_DATA_HOME";
const CASE: &str = "CAT_SURGE_STARTUP_DATA_HOME_CASE";
const NAME: &str = "native_startup_data_home_is_limited_to_restoration";

fn settle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.pending.is_some() {
        assert!(Instant::now() < deadline, "load timed out: {}", app.status);
        app.host.pump();
        app.handle_events();
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn surge_source() -> PathBuf {
    PathBuf::from(std::env::var_os("ProgramData").unwrap()).join("Surge XT")
}

/// One App per process: native host startup state is process-wide.
fn child(case: &str, original: Option<&Path>) {
    let root =
        std::env::temp_dir().join(format!("cat-surge-startup-{case}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let test = module_path!().split_once("::").unwrap().1;
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            &format!("{test}::{NAME}"),
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(CASE, case)
        .env(ROOT, &root)
        .env_remove("CMRT_SURGE_MINIMAL_DATA_HOME");
    match original {
        Some(value) => command.env(ENV, value),
        None => command.env_remove(ENV),
    };
    let out = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stderr.lines().filter(|l| l.starts_with("case=")) {
        eprintln!("{line}");
    }
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        out.status.success() && stdout.contains("1 passed"),
        "case {case} failed\n{stdout}\n{stderr}"
    );
}

#[test]
#[ignore = "requires installed Surge XT CLAP"]
fn native_startup_data_home_is_limited_to_restoration() {
    let Ok(case) = std::env::var(CASE) else {
        child("unset", None);
        child("cached", None);
        child("set", Some(&surge_source()));
        child("blocked", None);
        return;
    };
    let _dll = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os(ROOT).unwrap());
    if case == "blocked" {
        std::fs::write(root.join("cache"), b"not a directory").unwrap();
    }
    let before = std::env::var_os(ENV);
    let mut app = App::prepare();
    app.output = None;
    let key = crate::selection_host::default_instrument().unwrap();
    if case == "cached" {
        // As after `load_plugin_cache`: startup restores without a deferred scan.
        let plugin = app.host.restore_plugin(&key).unwrap();
        app.plugins.push(plugin);
        app.deferred_scan = false;
    }
    app.restore = Some(key);
    app.restore_before_gui();
    settle(&mut app);
    assert_eq!(app.instances.len(), 1, "{}", app.status);
    assert_eq!(std::env::var_os(ENV), before, "startup must restore {ENV}");
    let log = std::fs::read_to_string(root.join("error.log")).unwrap_or_default();
    if case == "blocked" {
        assert!(!app.instances[0].minimal_data_home);
        assert!(log.contains("Surge XT minimal data home"), "{log}");
    } else {
        assert!(app.instances[0].minimal_data_home, "{}", app.status);
        assert!(log.is_empty(), "{log}");
        let built = std::fs::read_dir(root.join("cache/surge-data-min-v2"))
            .unwrap()
            .count();
        assert_eq!(built, 1, "the minimal tree is prepared under the app cache");
    }

    // A manual load of the same plugin keeps the normal data home throughout.
    app.load_plugin(0);
    assert!(app.pending.as_ref().unwrap().data_home.is_none());
    assert_eq!(std::env::var_os(ENV), before);
    settle(&mut app);
    assert_eq!(app.instances.len(), 1, "{}", app.status);
    assert!(!app.instances[0].minimal_data_home);
    assert_eq!(std::env::var_os(ENV), before);
    eprintln!("case={case} original={before:?} ok");
}
