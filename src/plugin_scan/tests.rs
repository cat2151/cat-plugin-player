use super::*;
use crate::{config::PluginKey, ffi::PluginInfo};

#[test]
fn cached_catalog_is_ready_without_loading_plugins_and_unchanged_refresh_keeps_generation() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let root = crate::selection_tests::directory("cached-startup");
    let config = root.join("config.toml");
    // An empty file proves cache registration does not load the plugin DLL.
    let bundle = root.join("test.clap");
    std::fs::write(&bundle, []).unwrap();
    let key = PluginKey {
        format: "CLAP".into(),
        id: "test".into(),
        name: "Test".into(),
        vendor: "Test".into(),
        bundle_path: bundle.to_string_lossy().into_owned(),
    };
    let cached = PluginInfo {
        index: 77,
        format: key.format.clone(),
        id: key.id.clone(),
        name: key.name.clone(),
        vendor: key.vendor.clone(),
        bundle_path: key.bundle_path.clone(),
        kind: PluginKind::Unknown,
    };
    plugin_cache::save(&config, &[cached]).unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.config_path = Ok(config.clone());
    app.load_plugin_cache();
    assert!(!app.deferred_scan && !app.scanning);
    assert!(app.pending.is_none());
    assert_eq!(app.plugins.len(), 1);
    assert_ne!(app.plugins[0].index, 77);
    assert!(app.catalog_startup.refresh_pending);
    let generation = app.patch_catalog.generation;
    app.catalog_startup.running = true;
    assert!(app.finish_background_scan(None));
    assert_eq!(app.patch_catalog.generation, generation);
    app.catalog_startup.running = true;
    assert!(app.finish_background_scan(Some("test failure")));
    assert_eq!(app.plugins.len(), 1);
    assert_eq!(plugin_cache::load(&config).unwrap().unwrap().len(), 1);
    let mut another = key;
    another.id = "second".into();
    app.host.restore_plugin(&another).unwrap();
    app.catalog_startup.running = true;
    assert!(app.finish_background_scan(None));
    assert_eq!(app.plugins.len(), 2);
    assert_eq!(app.patch_catalog.generation, generation + 1);
    assert_eq!(plugin_cache::load(&config).unwrap().unwrap().len(), 2);
    app.restore = Some(PluginKey {
        format: "CLAP".into(),
        id: "missing-saved-target".into(),
        name: "Missing".into(),
        vendor: "Test".into(),
        bundle_path: "X:/missing/saved-target.clap".into(),
    });
    app.restore_before_gui();
    assert!(app.deferred_scan);
    assert!(
        app.restore.is_some(),
        "scan must retain the restoration target"
    );
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}
