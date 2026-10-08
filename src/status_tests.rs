use super::*;

#[test]
fn library_tab_restores_both_choices_and_keeps_legacy_default() {
    assert_eq!(
        serde_json::from_str::<Status>("{}").unwrap().show_favorites,
        None
    );
    let directory = directory();
    std::fs::create_dir_all(&directory).unwrap();
    let config = directory.join("config.toml");
    for show_favorites in [false, true] {
        Status {
            show_favorites: Some(show_favorites),
            ..Default::default()
        }
        .save(&config)
        .unwrap();
        assert_eq!(
            Status::load(&config).unwrap().show_favorites,
            Some(show_favorites)
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}

fn directory() -> PathBuf {
    std::env::temp_dir().join(format!(
        "cat-status-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn migrates_legacy_session_preserves_settings_comments_and_backups() {
    let directory = directory();
    std::fs::create_dir_all(&directory).unwrap();
    let config = directory.join("config.toml");
    let original = "# user notes\nsequence_pattern = 'off'\nselected_sequence = 'guitar_arpeggio'\nsequence_velocity = 'random'\nsequence_modulation = 'sweep'\neffect_bypassed = true\n\n[effect]\nformat = 'CLAP'\nid = 'reverb'\n[favorites]\ninstrument = 'favo1'\neffect = 'favo2'\n\n# main window\n[window.main]\nx = -120.0 # offset\ny = 80.0\n\n[build] # build settings\nUAPMD_DIR = 'X:/dependencies/uapmd'\n\n[custom]\nvalue = 'user data' # preserve unknown\n";
    std::fs::write(&config, original).unwrap();
    let status = Status::load(&config).unwrap();
    assert_eq!(status.sequence_pattern, SequencePattern::Off);
    assert_eq!(status.selected_sequence, SequencePattern::GuitarArpeggio);
    assert_eq!(status.sequence_velocity, SequenceVelocity::Range80To127);
    assert_eq!(status.sequence_modulation, SequenceModulation::Sweep);
    assert!(status.effect_bypassed);
    assert_eq!(status.effects[0].id, "reverb");
    assert_eq!(status.favorites.instrument.as_deref(), Some("favo1"));
    assert_eq!(status.favorites.effects, ["favo2"]);
    let cleaned = std::fs::read_to_string(&config).unwrap();
    assert!(cleaned.contains("# user notes"));
    assert!(cleaned.contains("# main window\n[window.main]\nx = -120.0 # offset\ny = 80.0"));
    assert!(cleaned.contains("[build] # build settings\nUAPMD_DIR = 'X:/dependencies/uapmd'"));
    assert!(cleaned.contains("[custom]\nvalue = 'user data' # preserve unknown"));
    assert!(!cleaned.contains("sequence_pattern") && !cleaned.contains("[favorites]"));
    assert_eq!(
        std::fs::read_to_string(config.with_extension("toml.before-status-migration.bak")).unwrap(),
        original
    );
    let settings = crate::config::Config::load(&config).unwrap();
    assert_eq!(settings.window.main.x, Some(-120.0));
    assert_eq!(
        settings.build.uapmd_dir.unwrap(),
        PathBuf::from("X:/dependencies/uapmd")
    );
    status.save(&config).unwrap();
    assert_eq!(std::fs::read_to_string(&config).unwrap(), cleaned);
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path(&config)).unwrap()).unwrap();
    assert!(json.get("window").is_none() && json.get("build").is_none());
    assert!(json.get("effect").is_none() && json["favorites"].get("effect").is_none());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn json_wins_over_stale_toml_and_invalid_files_are_never_overwritten() {
    let directory = directory();
    std::fs::create_dir_all(&directory).unwrap();
    let config = directory.join("config.toml");
    Status {
        sequence_pattern: SequencePattern::Off,
        ..Default::default()
    }
    .save(&config)
    .unwrap();
    assert!(!config.exists(), "normal status save must not create TOML");
    std::fs::write(&config, "sequence_pattern = 'steps'\n# keep me\n").unwrap();
    assert_eq!(
        Status::load(&config).unwrap().sequence_pattern,
        SequencePattern::Off
    );
    assert!(std::fs::read_to_string(&config)
        .unwrap()
        .contains("# keep me"));
    std::fs::write(path(&config), "{broken").unwrap();
    let original = "sequence_pattern = 'steps'\n";
    std::fs::write(&config, original).unwrap();
    assert!(Status::load(&config).is_err());
    assert!(Status::default().save(&config).is_err());
    assert_eq!(std::fs::read_to_string(path(&config)).unwrap(), "{broken");
    assert_eq!(std::fs::read_to_string(&config).unwrap(), original);
    std::fs::write(path(&config), "{}").unwrap();
    std::fs::write(&config, "[broken").unwrap();
    assert!(Status::default().save(&config).is_err());
    assert_eq!(std::fs::read_to_string(path(&config)).unwrap(), "{}");
    assert_eq!(std::fs::read_to_string(&config).unwrap(), "[broken");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sparse_status_keeps_effect_order_and_empty_favorite_slots() {
    let mut status: Status = toml::from_str("[[effects]]\nformat='CLAP'\nid='first'\n[[effects]]\nformat='VST3'\nid='second'\n[favorites]\neffects=['', 'favo2']").unwrap();
    status.migrate_effects();
    let json = serde_json::to_string(&status).unwrap();
    let restored: Status = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.effects, status.effects);
    assert_eq!(restored.favorites.effects, ["", "favo2"]);
    let empty: serde_json::Value = serde_json::to_value(Status::default()).unwrap();
    assert!(empty.get("effects").is_none());
    assert!(empty["favorites"].get("effects").is_none());
    for pattern in
        std::iter::once(SequencePattern::Off).chain(SequencePattern::TYPES.iter().copied())
    {
        let json = serde_json::to_string(&Status {
            sequence_pattern: pattern,
            selected_sequence: pattern,
            ..Default::default()
        })
        .unwrap();
        let restored: Status = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.sequence_pattern, pattern);
        assert_eq!(restored.selected_sequence, pattern);
    }
    for &velocity in SequenceVelocity::TYPES {
        let json = serde_json::to_string(&Status {
            sequence_velocity: velocity,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            serde_json::from_str::<Status>(&json)
                .unwrap()
                .sequence_velocity,
            velocity
        );
    }
}

#[test]
fn stopped_selection_and_legacy_history_keep_transport_and_scanning_fallback() {
    for pattern in ["off", "steps", "guitar_arpeggio"] {
        let legacy: Status = toml::from_str(&format!("sequence_pattern = '{pattern}'")).unwrap();
        let expected = if pattern == "guitar_arpeggio" {
            SequencePattern::GuitarArpeggio
        } else {
            SequencePattern::Steps
        };
        assert_eq!(
            legacy.sequence_pattern.selection(legacy.selected_sequence),
            expected
        );
        assert_eq!(
            legacy.sequence_pattern == SequencePattern::Off,
            pattern == "off"
        );
    }
    let history: Status = toml::from_str("[last_played]\nformat = 'CLAP'\nid = 'synth'").unwrap();
    assert_eq!(history.sequence_pattern, SequencePattern::Steps);
    assert!(history.effects.is_empty() && history.favorites.instrument.is_none());
    assert!(history.last_played.unwrap().bundle_path.is_empty());
    let status = Status {
        sequence_pattern: SequencePattern::Off,
        selected_sequence: SequencePattern::GuitarArpeggio,
        ..Default::default()
    };
    let restored: Status = serde_json::from_str(&serde_json::to_string(&status).unwrap()).unwrap();
    assert_eq!(restored.sequence_pattern, SequencePattern::Off);
    assert_eq!(
        restored
            .sequence_pattern
            .selection(restored.selected_sequence),
        SequencePattern::GuitarArpeggio
    );
}

#[test]
fn normal_save_preserves_live_user_edits_byte_for_byte() {
    let directory = directory();
    std::fs::create_dir_all(&directory).unwrap();
    let config = directory.join("config.toml");
    let original = "# preferences\r\n[window.main]\r\nx=1.0 # home\r\ny=2.0\r\n";
    std::fs::write(&config, original).unwrap();
    let mut status = Status::load(&config).unwrap();
    assert!(status.on_right);
    assert!(serde_json::from_str::<Status>("{}").unwrap().on_right);
    status.on_right = false;
    let edited = "# edited while running\r\n[window.main]\r\nx=3.0 # new\r\ny=4.0\r\n[custom]\r\nvalue='keep'\r\n";
    std::fs::write(&config, edited).unwrap();
    status.save(&config).unwrap();
    let restored = Status::load(&config).unwrap();
    assert!(!restored.on_right);
    assert!(!crate::scope_ui::ScopeUi::with_on_right(restored.on_right).on_right);
    assert_eq!(std::fs::read_to_string(&config).unwrap(), edited);
    assert!(!config
        .with_extension("toml.before-status-migration.bak")
        .exists());
    std::fs::remove_dir_all(directory).unwrap();
}
