use super::*;
use std::fs;

/// A scratch directory removed on drop; junctions inside are removed, not followed.
struct Scratch(PathBuf);
impl Scratch {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("surge-data-home-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn source(root: &Path) -> PathBuf {
    let source = root.join("source & %PATH%! 日本語");
    for name in [
        "patches_factory/Templates",
        "patches_factory/Keys",
        "patches_3rdparty",
        "wavetables",
        "wavetables_3rdparty",
        "fx_presets",
        "skins",
    ] {
        fs::create_dir_all(source.join(name)).unwrap();
    }
    fs::write(
        source.join("patches_factory/Templates/Init Saw.fxp"),
        b"initial patch",
    )
    .unwrap();
    fs::write(
        source.join("patches_factory/Keys/Organ.fxp"),
        b"organ patch",
    )
    .unwrap();
    fs::write(
        source.join("patches_3rdparty/Other.fxp"),
        b"third party patch",
    )
    .unwrap();
    fs::write(source.join("wavetables/wave.wt"), b"sound resource").unwrap();
    fs::write(source.join("fx_presets/preset.srgfx"), b"preset list").unwrap();
    fs::write(source.join("skins/shared.xml"), b"live resource").unwrap();
    fs::write(source.join("resource.txt"), b"shared file").unwrap();
    source
}

#[test]
fn minimal_tree_keeps_initial_sound_and_live_resources_without_scanning_other_patches() {
    let root = Scratch::new("keeps");
    let source = source(root.path());
    let cache = root.path().join("cache");
    let minimal = prepare(&source, &cache).unwrap();
    assert_eq!(
        fs::read(minimal.join("patches_factory/Templates/Init Saw.fxp")).unwrap(),
        b"initial patch"
    );
    assert!(!minimal.join("patches_factory/Keys").exists());
    assert_eq!(
        fs::read_dir(minimal.join("patches_3rdparty"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        fs::read(source.join("patches_factory/Keys/Organ.fxp")).unwrap(),
        b"organ patch"
    );
    for name in OMITTED {
        assert!(fs::read_dir(minimal.join(name)).unwrap().next().is_none());
    }
    assert_eq!(
        fs::read(source.join("wavetables/wave.wt")).unwrap(),
        b"sound resource"
    );
    fs::write(source.join("skins/shared.xml"), b"updated skin").unwrap();
    assert_eq!(
        fs::read(minimal.join("skins/shared.xml")).unwrap(),
        b"updated skin"
    );
    assert_eq!(prepare(&source, &cache).unwrap(), minimal);
    fs::write(source.join("resource.txt"), b"new resource").unwrap();
    let refreshed = prepare(&source, &cache).unwrap();
    assert_ne!(minimal, refreshed);
    assert_eq!(
        fs::read(refreshed.join("resource.txt")).unwrap(),
        b"new resource"
    );
}

#[test]
fn empty_templates_and_corrupt_cached_links_preserve_source_and_fail_safely() {
    let root = Scratch::new("corrupt");
    let source = source(root.path());
    let cache = root.path().join("cache");
    let minimal = prepare(&source, &cache).unwrap();
    #[cfg(windows)]
    fs::remove_dir(minimal.join("skins")).unwrap();
    #[cfg(not(windows))]
    fs::remove_file(minimal.join("skins")).unwrap();
    assert!(prepare(&source, &cache).is_err());
    assert_eq!(
        fs::read(source.join("wavetables/wave.wt")).unwrap(),
        b"sound resource"
    );
    fs::remove_file(source.join("patches_factory/Templates/Init Saw.fxp")).unwrap();
    assert!(prepare(&source, &root.path().join("empty-cache")).is_err());
    assert!(!root.path().join("empty-cache").exists());
}

#[test]
fn child_player_gets_original_data_home_or_removes_the_editors_override() {
    let mut command = Command::new("player");
    child_environment(
        &mut command,
        Some(std::ffi::OsStr::new("X:/custom/Surge XT")),
    );
    assert_eq!(
        command
            .get_envs()
            .find(|(key, _)| *key == ENV)
            .unwrap()
            .1
            .unwrap(),
        "X:/custom/Surge XT"
    );
    child_environment(&mut command, None);
    assert!(command
        .get_envs()
        .find(|(key, _)| *key == ENV)
        .unwrap()
        .1
        .is_none());
}

#[test]
fn unexpected_resource_catalog_in_cache_fails_without_changing_source() {
    let root = Scratch::new("unexpected");
    let source = source(root.path());
    let cache = root.path().join("cache");
    let minimal = prepare(&source, &cache).unwrap();
    fs::write(
        minimal.join("fx_presets/unexpected.srgfx"),
        b"extra catalog",
    )
    .unwrap();
    assert!(prepare(&source, &cache).is_err());
    assert_eq!(
        fs::read(source.join("fx_presets/preset.srgfx")).unwrap(),
        b"preset list"
    );
}
