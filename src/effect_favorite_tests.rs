use super::*;
use crate::{SequenceModulation, SequencePattern, SequenceVelocity};

#[test]
#[ignore = "requires installed Dexed and Dragonfly Hall Reverb CLAP"]
fn native_effect_recall_preserves_playback_when_reconnecting() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.restore = None;
    app.restore_effect = None;
    app.output = None;
    app.deferred_scan = false;
    let dir = std::env::temp_dir().join(format!("cat-effect-recall-{}", std::process::id()));
    assert!(!dir.exists());
    let path = dir.join("config.toml");
    app.config_path = Ok(path.clone());
    app.initialize_favorites();
    app.start_scan(false);
    settle(&mut app);
    let dexed = app
        .plugins
        .iter()
        .position(|p| p.name == "Dexed" && p.format == "CLAP")
        .unwrap();
    let reverb = app
        .plugins
        .iter()
        .position(|p| p.name == "Dragonfly Hall Reverb" && p.format == "CLAP")
        .unwrap();
    app.load_plugin(dexed);
    settle(&mut app);
    app.load_plugin(reverb);
    settle(&mut app);
    let source = app.instrument_id().unwrap();
    let effect = app.effect_id().unwrap();
    app.sequence_pattern = SequencePattern::Steps;
    app.add_favorite(effect);
    let favorite = app.favorites.library.entries[0].id.clone();
    assert!(app.remove_plugin(effect));
    // Reload metadata too: effect entries omit playback controls on disk.
    app.favorites.library = Library::load(&path).unwrap();
    for pattern in [SequencePattern::GuitarArpeggio, SequencePattern::Off] {
        app.sequence_pattern = pattern;
        app.selected_sequence = SequencePattern::GuitarArpeggio;
        app.sequence_velocity = SequenceVelocity::Range40To100;
        app.sequence_modulation = SequenceModulation::Sweep;
        for reconnect in [true, false] {
            app.load_favorite(&favorite);
            settle(&mut app);
            assert_eq!(app.sequence_pattern, pattern, "reconnect={reconnect}");
            assert_eq!(app.selected_sequence, SequencePattern::GuitarArpeggio);
            assert_eq!(app.sequence_velocity, SequenceVelocity::Range40To100);
            assert_eq!(app.sequence_modulation, SequenceModulation::Sweep);
            assert_eq!(app.instrument_id(), Some(source));
            assert!(app.effect_id().is_some(), "{}", app.status);
            assert_eq!(
                crate::config::Config::load(&path).unwrap().sequence_pattern,
                pattern
            );
        }
        assert!(app.remove_plugin(app.effect_id().unwrap()));
    }
    drop(app);
    std::fs::remove_dir_all(dir).unwrap();
}

fn settle(app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while app.pending.is_some() || app.scanning {
        assert!(
            std::time::Instant::now() < deadline,
            "native operation timed out"
        );
        app.host.pump_startup();
        app.handle_events();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
