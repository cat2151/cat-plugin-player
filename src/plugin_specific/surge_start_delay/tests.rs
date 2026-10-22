use super::*;
use crate::{config::PluginKey, patch_catalog::Candidate, patch_preparation::Prepared};
use std::time::Duration;
mod native;

fn fixture() -> (Table, Prepared, PluginKey) {
    let name = "patches_factory/Basses/One.fxp";
    let state = vec![1, 2, 3];
    let key = PluginKey {
        format: "CLAP".into(),
        id: ID.into(),
        name: "Surge XT".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/Surge.clap".into(),
    };
    let prepared = Prepared {
        candidate: Candidate {
            format: "CLAP".into(),
            plugin_id: ID.into(),
            plugin_name: "Surge XT".into(),
            display: name.into(),
            path: "X:/patches/One.fxp".into(),
            bundle_path: key.bundle_path.clone().into(),
            entry: cmrt_patch_select::PatchCatalogEntry::from_display(name.into()),
            measurement: Default::default(),
        },
        state: state.clone(),
    };
    let table = Table {
        format_version: 1,
        plugin_format: "CLAP".into(),
        plugin_id: ID.into(),
        max_onset_delta_ms: 1.0,
        measurement: Measurement {
            candidate_wait_ms: 10,
            reference_wait_ms: 100,
            capture_ms: 200,
            sample_rate: 48000,
            note: 48,
            velocity: 100,
            cc1: 0,
            onset_ratio: 0.01,
            onset_window_frames: 48,
            trials_per_wait: 1,
            plugin_sha256: hash(b"plugin"),
            renderer_sha256: hash(b"renderer"),
        },
        entries: BTreeMap::from([(
            name.into(),
            Entry {
                fxp_sha256: hash(b"fxp"),
                state_sha256: Some(hash(&state)),
                candidate_onset_ms: Some(1.0),
                reference_onset_ms: Some(0.0),
                outcome: Outcome::Measured,
                zero_wait_allowed: true,
                error: None,
            },
        )]),
    };
    (table, prepared, key)
}

fn directory() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "cat-delay-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn reclassification_keeps_measurements_and_errors_out_of_allowlist() {
    let (mut table, _, _) = fixture();
    let m = serde_json::to_value(&table.measurement).unwrap();
    table.select(0.5).unwrap();
    assert!(!table.entries.values().next().unwrap().zero_wait_allowed);
    table.select(1.0).unwrap();
    assert!(table.entries.values().next().unwrap().zero_wait_allowed);
    assert_eq!(serde_json::to_value(&table.measurement).unwrap(), m);
    table.entries.values_mut().next().unwrap().outcome = Outcome::Error;
    table.select(200.0).unwrap();
    assert!(!table.entries.values().next().unwrap().zero_wait_allowed);
    table.plugin_format = "VST3".into();
    assert!(table.validate().is_err());
}

#[test]
fn current_state_permission_survives_restart_and_rejects_changes() {
    let root = directory();
    let config = root.join("config.toml");
    let (table, mut prepared, mut key) = fixture();
    let path = local_path(&config);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_vec(&table).unwrap()).unwrap();
    let mut rt = Runtime::load(&config, None);
    let permit = rt.authorize(1, &prepared).unwrap();
    rt.replace(1, Some(permit.clone()));
    let fallback = Duration::from_millis(100);
    assert_eq!(rt.delay(1, &key, fallback), Duration::ZERO);
    // Plugin serialization changes bytes; the permission follows the saved snapshot.
    rt.saved_state(1, b"serialized state");
    let saved = rt.snapshot(1).unwrap();
    let mut next = Runtime::load(&config, Some(saved.clone()));
    next.restore(9, &key, b"serialized state");
    assert_eq!(next.delay(9, &key, fallback), Duration::ZERO);
    next.restore(9, &key, b"edited");
    assert_eq!(next.delay(9, &key, fallback), fallback);
    let mut changed_table = table.clone();
    changed_table.select(0.5).unwrap();
    std::fs::write(&path, serde_json::to_vec(&changed_table).unwrap()).unwrap();
    let mut stricter = Runtime::load(&config, Some(saved.clone()));
    stricter.restore(9, &key, b"serialized state");
    assert_eq!(stricter.delay(9, &key, fallback), fallback);
    changed_table.select(2.0).unwrap();
    std::fs::write(&path, serde_json::to_vec(&changed_table).unwrap()).unwrap();
    let mut revised = Runtime::load(&config, Some(saved.clone()));
    revised.restore(9, &key, b"serialized state");
    assert_eq!(revised.delay(9, &key, fallback), Duration::ZERO);
    key.format = "VST3".into();
    next.restore(9, &key, b"serialized state");
    assert_eq!(next.delay(9, &key, fallback), fallback);
    prepared.state.push(4);
    assert!(rt.authorize(1, &prepared).is_none());
    prepared.state.pop();
    rt.expose_editor(1);
    assert!(rt.authorize(1, &prepared).is_none());
    assert!(rt.snapshot(1).is_none());
    rt.editor_closed(1);
    assert!(rt.authorize(1, &prepared).is_some());
    std::fs::write(&path, b"invalid downloaded table").unwrap();
    let mut broken = Runtime::load(&config, Some(saved));
    broken.restore(1, &key, b"serialized state");
    assert!(broken.snapshot(1).is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn old_status_and_missing_table_keep_the_existing_delay() {
    let status: crate::status::Status = serde_json::from_str("{}").unwrap();
    assert!(status.surge_start_delay.is_none());
    let root = directory();
    let (_, prepared, key) = fixture();
    let rt = Runtime::load(&root.join("config.toml"), None);
    assert!(rt.authorize(1, &prepared).is_none());
    assert_eq!(
        rt.delay(1, &key, Duration::from_millis(100)),
        Duration::from_millis(100)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_or_invalid_download_does_not_replace_local_table() {
    let root = directory();
    let target = root.join("table.json");
    let (table, _, _) = fixture();
    let bytes = serde_json::to_vec(&table).unwrap();
    runtime::refresh_with(|| Ok(bytes.clone()), &target).unwrap();
    assert!(runtime::refresh_with(|| Err("offline".into()), &target).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    assert!(runtime::refresh_with(|| Ok(b"corrupt".to_vec()), &target).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_options_reject_conflicting_modes_and_invalid_thresholds() {
    use clap::Parser;
    assert!(crate::cli::Cli::try_parse_from([
        "app",
        "measure-surge-start-delay",
        "--output",
        "X:/table.json",
        "--candidate-wait-ms",
        "0"
    ])
    .is_ok());
    for args in [
        vec![
            "app",
            "--restore-state",
            "measure-surge-start-delay",
            "--output",
            "X:/table.json",
        ],
        vec![
            "app",
            "measure-surge-start-delay",
            "--output",
            "X:/table.json",
            "--jobs",
            "0",
        ],
        vec![
            "app",
            "measure-surge-start-delay",
            "--output",
            "X:/table.json",
            "--max-onset-delta-ms",
            "NaN",
        ],
        vec![
            "app",
            "measure-surge-start-delay",
            "--output",
            "X:/table.json",
            "--reclassify",
            "X:/old.json",
            "--candidate-wait-ms",
            "0",
        ],
    ] {
        assert!(crate::cli::Cli::try_parse_from(args).is_err());
    }
}
