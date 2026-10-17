use super::*;
use crate::patch_catalog::{candidates, SURGE_ID};
use cmrt_tui_core::patch_catalog_cache::load_from;

fn catalog(format_version: u32, display: &str) -> Vec<u8> {
    serde_json::json!({
        "format_version": format_version,
        "patches": [{
            "audio": {
                "reference": {"plugin": "id:org.surge-synth-team.surge-xt", "display": display},
                "normalized_display": display.to_lowercase(),
                "sort": {
                    "category": "", "source_rank": 0, "vendor": "",
                    "category_rest": display, "path_rest": display
                },
                "voicing": {"kind": "external_lookup", "key": display}
            },
            "second_load_ms": 13,
            "first_load_error": null,
            "second_load_error": null
        }],
        "plugins": [{
            "name": "Surge XT",
            "plugin_path": "D:\\CLAP\\Surge XT.clap",
            "plugin_id": SURGE_ID,
            "base": {"Shared": "D:\\Surge XT"},
            "dirs": ["D:\\Surge XT\\patches_factory"],
            "source_notices": []
        }],
        "patch_voicings": {},
        "catalog_notes": []
    })
    .to_string()
    .into_bytes()
}

fn destination(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "cat-plugin-player-surge-catalog-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    directory.join("patch-catalog").join("surge-xt.json")
}

fn cleanup(destination: &Path) {
    let _ = std::fs::remove_dir_all(destination.parent().unwrap().parent().unwrap());
}

fn displays(path: &Path) -> Vec<String> {
    let (snapshot, _) = load_from(path).unwrap().into_parts();
    snapshot
        .audio_patches()
        .iter()
        .map(|patch| patch.reference.display.clone())
        .collect()
}

#[test]
fn fetched_catalog_is_saved_and_its_path_returned() {
    let destination = destination("saved");
    let bytes = catalog(5, "patches_factory/Basses/Attacky.fxp");
    let path = path_with(|| Ok(bytes.clone()), &destination).unwrap();
    assert_eq!(path, destination);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(displays(&path), ["patches_factory/Basses/Attacky.fxp"]);
    assert!(!destination.with_extension("download.tmp").exists());
    cleanup(&destination);
}

#[test]
fn fetch_failure_keeps_the_previous_file() {
    let destination = destination("previous");
    let previous = catalog(5, "patches_factory/Leads/Old.fxp");
    path_with(|| Ok(previous.clone()), &destination).unwrap();
    let path = path_with(|| Err("offline".into()), &destination).unwrap();
    assert_eq!(path, destination);
    assert_eq!(std::fs::read(&path).unwrap(), previous);
    cleanup(&destination);
}

#[test]
fn fetch_failure_without_a_previous_file_is_an_error() {
    let destination = destination("none");
    let error = path_with(|| Err("offline".into()), &destination).unwrap_err();
    assert!(error.contains("offline"), "{error}");
    assert!(!destination.exists());
    cleanup(&destination);
}

#[test]
fn invalid_download_keeps_the_previous_file() {
    let destination = destination("invalid");
    let previous = catalog(5, "patches_factory/Leads/Old.fxp");
    path_with(|| Ok(previous.clone()), &destination).unwrap();
    let path = path_with(
        || Ok(catalog(4, "patches_factory/Leads/New.fxp")),
        &destination,
    )
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), previous);
    assert!(!destination.with_extension("download.tmp").exists());
    cleanup(&destination);
}

#[test]
fn invalid_download_without_a_previous_file_reports_the_check() {
    let destination = destination("invalid-none");
    let error = path_with(|| Ok(catalog(4, "x.fxp")), &destination).unwrap_err();
    assert!(error.contains("format version"), "{error}");
    assert!(!destination.exists());
    cleanup(&destination);
}

/// Needs the sibling `cat-music-patterns` checkout.
#[test]
#[ignore]
fn published_catalog_builds_every_surge_candidate() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cat-music-patterns/surge-xt-patch-catalog.json");
    let candidates = candidates(load_from(&path).unwrap()).unwrap();
    assert_eq!(candidates.len(), 3008);
    for candidate in &candidates {
        assert_eq!(candidate.plugin_id, SURGE_ID);
        assert!(
            candidate
                .path
                .to_string_lossy()
                .starts_with("C:\\ProgramData\\Surge XT\\"),
            "{}",
            candidate.path.display()
        );
    }
}

/// Needs the network and the catalog pushed to cat-music-patterns.
#[test]
#[ignore]
fn published_url_serves_a_valid_catalog() {
    let destination = destination("network");
    let fetched = fetch(URL).and_then(|bytes| replace_if_valid(&bytes, &destination));
    cleanup(&destination);
    fetched.unwrap();
}
