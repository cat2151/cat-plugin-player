use super::*;

#[test]
fn roundtrip_corruption_and_write_failure_preserve_original() {
    let root = std::env::temp_dir().join(format!(
        "cat-filter-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = Ok(root.join("config.toml"));
    let mut store = Store::load(&config);
    assert!(store.replace(vec![("bass".into(), "pad plugin:floe".into())]));
    assert_eq!(Store::load(&config).presets, store.presets);
    let path = root.join("patch-filter-presets.json");
    std::fs::write(&path, b"broken").unwrap();
    let mut corrupt = Store::load(&config);
    assert!(corrupt.error.is_some());
    assert!(!corrupt.replace(Vec::new()));
    assert_eq!(std::fs::read(&path).unwrap(), b"broken");
    // A directory at the destination forces atomic replacement to fail.
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let before = store.presets.clone();
    assert!(!store.replace(Vec::new()));
    assert_eq!(store.presets, before);
    assert!(path.is_dir());
    assert_eq!(
        std::fs::read_dir(&root).unwrap().count(),
        1,
        "no failed-write residue"
    );
    std::fs::remove_dir_all(root).unwrap();
}
