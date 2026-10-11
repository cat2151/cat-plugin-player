//! Opt-in real load transactions; all state writes go to a fresh temporary directory.
use super::*;
use crate::config::PluginKey;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

fn settle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(Instant::now() < deadline, "{}", app.status);
        app.host.pump_startup();
        app.handle_events();
        app.poll_browser_load();
        if app.pending.is_none() && app.browser_auto_effect.is_none() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
#[ignore = "requires installed Surge XT/Effects CLAP and built native shim"]
fn sole_plugin_native_startup_stopped_restore_and_browsers() {
    let _library = crate::native_library::load().unwrap();
    let root = std::env::temp_dir().join(format!(
        "cat-autoload-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut app = App::prepare();
    app.config_path = Ok(root.join("config.toml"));
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    app.plugins.clear();
    app.initialize_favorites();
    let bundle = PathBuf::from(std::env::var_os("ProgramFiles").unwrap())
        .join("Common Files/CLAP/Surge Synth Team");
    for (name, id, kind) in [
        (
            "Surge XT",
            "org.surge-synth-team.surge-xt",
            PluginKind::Instrument,
        ),
        (
            "Surge XT Effects",
            "org.surge-synth-team.surge-xt-fx",
            PluginKind::Effect,
        ),
    ] {
        let key = PluginKey {
            name: name.into(),
            id: id.into(),
            format: "CLAP".into(),
            vendor: "Surge Synth Team".into(),
            bundle_path: bundle
                .join(format!("{name}.clap"))
                .to_string_lossy()
                .into_owned(),
        };
        let mut plugin = app
            .host
            .restore_plugin(&key)
            .expect("installed Surge plugin missing");
        plugin.kind = kind;
        app.plugins.push(plugin);
    }
    app.sequence_pattern = SequencePattern::Off;
    app.selected_sequence = SequencePattern::GuitarArpeggio;
    settle(&mut app);
    assert!(app.instrument_id().is_some(), "{}", app.status);
    assert_eq!(app.sequence_pattern, SequencePattern::GuitarArpeggio);
    assert!(app.effect_ids().is_empty());

    // Simulate a stopped saved session that has finished restoration.
    app.sequence_pattern = SequencePattern::Off;
    app.auto_start_pending = true;
    app.poll_browser_load();
    assert_eq!(app.sequence_pattern, SequencePattern::GuitarArpeggio);
    app.sequence_pattern = SequencePattern::Off;
    app.poll_browser_load();
    assert_eq!(
        app.sequence_pattern,
        SequencePattern::Off,
        "manual Stop must stay stopped"
    );

    // T's entry point after the instrument was removed.
    let id = app.instrument_id().unwrap();
    app.instances.clear();
    app.host.destroy_instance(id);
    app.open_patch_browser();
    settle(&mut app);
    assert!(app.patch_browser.open);
    assert!(app.instrument_id().is_some());
    assert_eq!(app.sequence_pattern, SequencePattern::GuitarArpeggio);
    app.patch_browser.close();

    // X's entry point loads the source first, then the sole effect, exactly once.
    let id = app.instrument_id().unwrap();
    app.instances.clear();
    app.host.destroy_instance(id);
    app.open_effect_browser();
    settle(&mut app);
    assert!(app.instrument_id().is_some(), "{}", app.status);
    assert_eq!(app.effect_ids().len(), 1, "{}", app.status);
    assert_eq!(app.effect_browser.target, app.effect_id());
    let ids = app.effect_ids();
    app.effect_browser.close();
    app.open_effect_browser();
    settle(&mut app);
    assert_eq!(
        app.effect_ids(),
        ids,
        "reopening must not duplicate an effect"
    );

    // Multiple instruments cannot be chosen implicitly.
    let id = app.instrument_id().unwrap();
    app.remove_plugin(id);
    let mut another = app.plugins[0].clone();
    another.id = "test.other".into();
    another.name = "Another synth".into();
    app.plugins.push(another);
    app.effect_browser.close();
    app.open_patch_browser();
    assert!(app.pending.is_none());
    assert!(app.instrument_id().is_none());
}
