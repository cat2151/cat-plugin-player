use crate::{
    ffi::PluginInfo, patch_browser_catalog::tests::candidate, patch_catalog::Catalog,
    plugin_list::PluginKind, App,
};
use eframe::egui;
use std::time::{Duration, Instant};

#[test]
fn startup_publishes_only_the_complete_scanned_catalog() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    let root = crate::selection_tests::directory("complete-catalog");
    let path = root.join("input.mid");
    crate::selection_tests::input(&path, &Default::default());
    app.selection = Some(crate::selection::Selection::load(path, 48000).unwrap());
    app.config_path = Err("isolated snapshot test".into());
    app.patch_browser = crate::patch_browser::Browser {
        open: true,
        restore_preset: Some("ALL".into()),
        ..Default::default()
    };
    let plugin = |id: &str| PluginInfo {
        index: 0,
        name: id.into(),
        vendor: "Test".into(),
        format: "CLAP".into(),
        id: id.into(),
        bundle_path: format!("X:/plugins/{id}.clap"),
        kind: PluginKind::Instrument,
    };
    app.plugins = vec![plugin("a")];
    app.patch_catalog = Catalog::from_candidates(
        vec![
            candidate("a", "Alpha", "First", None),
            candidate("b", "Beta", "Second", None),
        ],
        &app.plugins,
    );
    let ctx = egui::Context::default();
    // The audio-first restore knows only the initial instrument. Neither the
    // deferred nor the running scan may publish that partial candidate set.
    app.update_browser_snapshot(&ctx);
    assert!(app.patch_browser.build.is_none());
    assert!(app.patch_browser.snapshot.is_none());
    assert_eq!(app.patch_browser.restore_preset.as_deref(), Some("ALL"));
    app.deferred_scan = false;
    app.scanning = true;
    app.update_browser_snapshot(&ctx);
    assert!(app.patch_browser.build.is_none());
    app.plugins.push(plugin("b"));
    app.patch_catalog.reconcile(&app.plugins);
    app.scanning = false;
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.patch_browser.snapshot.is_none() {
        assert!(Instant::now() < deadline, "snapshot worker timed out");
        app.update_browser_snapshot(&ctx);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.patch_browser
            .snapshot
            .as_ref()
            .unwrap()
            .candidates
            .len(),
        2
    );
    assert_eq!(app.patch_browser.results.len(), 2);
    assert!(app.patch_browser.requests.desired.is_none());
    // Standalone player keeps its early partial-list publication behavior.
    app.selection = None;
    app.patch_browser.snapshot = None;
    app.scanning = true;
    app.deferred_scan = true;
    app.update_browser_snapshot(&ctx);
    assert!(app.patch_browser.build.is_some());
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}
