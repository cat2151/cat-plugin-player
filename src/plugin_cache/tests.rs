use super::*;

fn plugin(id: &str, index: i32) -> PluginInfo {
    PluginInfo {
        index,
        name: id.into(),
        vendor: "Test".into(),
        format: "CLAP".into(),
        id: id.into(),
        bundle_path: format!("X:/plugins/{id}.clap"),
        kind: PluginKind::Instrument,
    }
}

#[test]
fn cache_creates_on_first_scan_and_writes_only_metadata_changes() {
    let root = crate::selection_tests::directory("plugin-cache");
    let config = root.join("config.toml");
    assert!(load(&config).unwrap().is_none());
    let plugins = vec![plugin("a", 1), plugin("b", 2)];
    assert!(save(&config, &plugins).unwrap());
    assert_eq!(load(&config).unwrap().unwrap(), entries(&plugins));
    // Native scan order and process-local indices must not invalidate a cache.
    assert!(!save(&config, &[plugin("b", 0), plugin("a", 9)]).unwrap());
    let mut changed = plugins.clone();
    changed[0].kind = PluginKind::Effect;
    assert!(save(&config, &changed).unwrap());
    assert!(save(&config, &changed[..1]).unwrap());
    std::fs::write(path(&config), b"invalid cache").unwrap();
    assert!(load(&config).is_err());
    assert!(save(&config, &plugins).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}
