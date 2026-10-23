use crate::{
    chain_settings::Settings,
    selection::{exit_key, Selection},
    smf::{
        self,
        model::{Event, Music, Track},
    },
};
use eframe::egui;
use std::path::PathBuf;

pub(super) fn directory(name: &str) -> PathBuf {
    let root = std::env::var_os("SELECTION_TEST_WORK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = root.join(format!(
        "selection-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

pub(super) fn input(path: &std::path::Path, settings: &Settings) {
    let mut track = Track {
        events: vec![
            Event {
                delta: 0,
                tick: 0,
                status: 0x93,
                meta_type: 0,
                data: vec![60, 81],
            },
            Event {
                delta: 48000,
                tick: 48000,
                status: 0x83,
                meta_type: 0,
                data: vec![60, 0],
            },
            Event {
                delta: 480,
                tick: 48480,
                status: 0xff,
                meta_type: 0x2f,
                data: vec![],
            },
        ],
    };
    crate::chain_settings::write(&mut track, settings).unwrap();
    std::fs::write(
        path,
        smf::io::encode(&Music {
            format: 0,
            division: 480,
            tracks: vec![track],
        })
        .unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires CLAP fixture and default audio output; fixture renders silence"]
fn native_selection_patch_effect_favorite_keep_position_and_pending_exit_saves_final_state() {
    use crate::{chain_settings::PluginState, config::PluginKey, plugin_list::PluginKind};
    use std::sync::atomic::Ordering;
    let _library = crate::native_library::load().unwrap();
    let root = directory("native");
    let path = root.join("input.mid");
    let config = root.join("config.toml");
    let key = PluginKey {
        format: "CLAP".into(),
        id: "cat.test.state.normal".into(),
        name: "Fixture".into(),
        vendor: "cat test".into(),
        bundle_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/shim/Release/clap_state_test_plugin.clap")
            .to_string_lossy()
            .into_owned(),
    };
    input(
        &path,
        &Settings {
            instrument: Some(PluginState {
                reference: None,
                plugin: key.clone(),
                state: vec![1],
                preset_name: None,
                bypass: false,
            }),
            ..Default::default()
        },
    );
    let original = std::fs::read(&path).unwrap();
    let mut app = crate::App::prepare_mode(true);
    app.config_path = Ok(config.clone());
    app.deferred_scan = false;
    assert!(app.output.is_some(), "default f32 audio output is required");
    app.configure_selection(path.clone(), crate::cli::Browser::Patch)
        .unwrap();
    app.configure_browser_only(crate::cli::Browser::Patch);
    let rate = app.sample_rate();
    app.selection
        .as_mut()
        .unwrap()
        .set_timing(rate, 1.25, None)
        .unwrap();
    assert_eq!(
        app.selection
            .as_ref()
            .unwrap()
            .position
            .load(Ordering::Acquire),
        (f64::from(rate) * 1.25) as u64
    );
    assert!(app.take_browser_focus(crate::cli::Browser::Patch));
    assert!(!app.take_browser_focus(crate::cli::Browser::Patch));
    app.restore_before_gui();
    let settle = |app: &mut crate::App| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.pending.is_some() {
            assert!(std::time::Instant::now() < deadline, "{}", app.status);
            app.host.pump_startup();
            app.handle_events();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    };
    settle(&mut app);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app
        .selection
        .as_ref()
        .unwrap()
        .position
        .load(Ordering::Acquire)
        < 1000
    {
        assert!(std::time::Instant::now() < deadline);
        app.host.pump_startup();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let position = || {
        app.selection
            .as_ref()
            .unwrap()
            .position
            .load(Ordering::Acquire)
    };
    let before = position();
    app.apply_patch(crate::patch_preparation::Prepared {
        candidate: crate::patch_catalog::Candidate {
            format: key.format.clone(),
            plugin_id: key.id.clone(),
            plugin_name: key.name.clone(),
            display: "Browser tone".into(),
            path: "X:/patches/tone.fxp".into(),
            bundle_path: key.bundle_path.clone().into(),
            entry: cmrt_patch_select::PatchCatalogEntry::from_display("Browser tone".into()),
            measurement: Default::default(),
        },
        state: vec![4],
    });
    assert!(
        app.selection
            .as_ref()
            .unwrap()
            .position
            .load(Ordering::Acquire)
            >= before,
        "patch restarted input"
    );
    let mut fx = key.clone();
    fx.id = "cat.test.state.effect".into();
    fx.name = "Fixture effect".into();
    let mut plugin = app.host.restore_plugin(&fx).unwrap();
    plugin.kind = PluginKind::Effect;
    app.plugins.push(plugin);
    let before = app
        .selection
        .as_ref()
        .unwrap()
        .position
        .load(Ordering::Acquire);
    app.apply_effect(crate::effect_preparation::Prepared {
        candidate: crate::effect_catalog::Candidate {
            plugin_id: fx.id.clone(),
            plugin_name: fx.name.clone(),
            name: "Browser effect".into(),
            category: "test".into(),
            kind: "effect".into(),
            state: None,
            preset: cmrt_core::audio_effect::PresetLocation {
                path: "X:/presets/effect".into(),
                value: "effect".into(),
                display: "Browser effect".into(),
            },
        },
        target: None,
        state: vec![5],
    });
    app.patch_browser.open = false;
    app.selection_browser_closed(crate::cli::Browser::Patch);
    assert!(app.selection.as_ref().unwrap().exit_requested);
    assert!(app.patch_browser.open);
    let ctx = egui::Context::default();
    assert!(!app.selection_exit(&ctx));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    settle(&mut app);
    assert!(
        app.selection
            .as_ref()
            .unwrap()
            .position
            .load(Ordering::Acquire)
            >= before,
        "effect restarted input"
    );
    app.selection.as_mut().unwrap().exit_requested = false;
    let mut fx2 = fx.clone();
    fx2.id = "cat.test.state.effect2".into();
    fx2.name = "Fixture effect two".into();
    let mut plugin = app.host.restore_plugin(&fx2).unwrap();
    plugin.kind = PluginKind::Effect;
    app.plugins.push(plugin);
    let mut candidate = app.effect_browser.applied_in_target().unwrap().clone();
    candidate.plugin_id = fx2.id;
    candidate.plugin_name = fx2.name;
    candidate.preset.display = "Second effect".into();
    app.apply_effect(crate::effect_preparation::Prepared {
        candidate,
        target: None,
        state: vec![7],
    });
    settle(&mut app);
    let ids = app.effect_ids();
    assert_eq!(ids.len(), 2);
    let before = app
        .selection
        .as_ref()
        .unwrap()
        .position
        .load(Ordering::Acquire);
    assert!(app.reorder_effect(ids[1], ids[0], false));
    assert_eq!(app.effect_ids(), vec![ids[1], ids[0]]);
    assert!(
        app.selection
            .as_ref()
            .unwrap()
            .position
            .load(Ordering::Acquire)
            >= before,
        "reorder restarted input"
    );
    let before = app
        .selection
        .as_ref()
        .unwrap()
        .position
        .load(Ordering::Acquire);
    app.set_effect_bypass(true);
    assert!(app.effect_bypassed);
    assert!(
        app.selection
            .as_ref()
            .unwrap()
            .position
            .load(Ordering::Acquire)
            >= before,
        "bypass restarted input"
    );
    app.set_effect_bypass(false);
    let before = app
        .selection
        .as_ref()
        .unwrap()
        .position
        .load(Ordering::Acquire);
    assert!(app.remove_plugin(ids[1]));
    assert_eq!(app.effect_ids(), vec![ids[0]]);
    assert!(
        app.selection
            .as_ref()
            .unwrap()
            .position
            .load(Ordering::Acquire)
            >= before,
        "remove restarted input"
    );
    app.initialize_favorites();
    let favorite = app
        .favorites
        .library
        .add(
            &config,
            key,
            false,
            &[6],
            crate::favorites_store::FavoriteCapture {
                preset_name: Some("Favorite tone".into()),
                sequence_pattern: crate::SequencePattern::Custom,
                selected_sequence: Some(crate::SequencePattern::Custom),
                mml: Some("invalid MML must not replace the SMF".into()),
            },
        )
        .unwrap();
    let before = app
        .selection
        .as_ref()
        .unwrap()
        .position
        .load(Ordering::Acquire);
    app.load_favorite(&favorite.id);
    settle(&mut app);
    assert!(app.status.contains("Loaded favorite"), "{}", app.status);
    assert!(
        app.selection
            .as_ref()
            .unwrap()
            .position
            .load(Ordering::Acquire)
            >= before,
        "favorite restarted input"
    );
    assert!(app.mml_input.confirmed.is_empty());
    app.selection.as_mut().unwrap().path = root.join("missing/input.mid");
    app.selection.as_mut().unwrap().exit_requested = true;
    assert!(!app.selection_exit(&ctx));
    assert!(app.patch_browser.open);
    assert!(app
        .config_error
        .as_ref()
        .unwrap()
        .contains("Could not save selection"));
    assert!(!app.selection.as_ref().unwrap().saved);
    app.selection.as_mut().unwrap().path = path.clone();
    app.patch_browser.open = false;
    app.selection_browser_closed(crate::cli::Browser::Patch);
    assert!(app.selection_exit(&ctx), "{:?}", app.config_error);
    let music = smf::io::load(&path).unwrap();
    let returned = crate::chain_settings::read(&music.tracks[0]).unwrap();
    assert_eq!(returned.instrument.unwrap().state, vec![6]);
    assert_eq!(returned.effects[0].state, vec![5]);
    assert_eq!(
        music.tracks[0]
            .events
            .iter()
            .find(|e| e.status == 0x93)
            .unwrap()
            .data,
        vec![60, 81]
    );
}

#[test]
fn save_failure_keeps_input_and_can_retry_without_losing_selection() {
    let root = directory("save");
    let path = root.join("input.mid");
    input(&path, &Settings::default());
    let original = std::fs::read(&path).unwrap();
    let mut selection = Selection::load(path.clone(), 48000).unwrap();
    selection.path = root.join("missing/input.mid");
    assert!(selection.save(&Settings::default()).is_err());
    assert!(!selection.saved);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    selection.path = path.clone();
    selection.save(&Settings::default()).unwrap();
    assert!(selection.saved);
    let loaded = smf::io::load(&path).unwrap();
    assert_eq!(
        loaded.tracks[0]
            .events
            .iter()
            .find(|e| e.status == 0x93)
            .unwrap()
            .data,
        vec![60, 81]
    );
    let mut selection = Selection::load(path.clone(), 48000).unwrap();
    std::fs::write(&path, b"external edit").unwrap();
    assert!(selection.save(&Settings::default()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"external edit");
}

#[test]
fn q_in_text_is_consumed_as_text_and_escape_always_requests_save() {
    let ctx = egui::Context::default();
    let id = egui::Id::new("query");
    let mut query = String::new();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(egui::TextEdit::singleline(&mut query).id(id))
                .request_focus();
        });
    });
    let event = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let input = egui::RawInput {
        events: vec![event(egui::Key::Q), egui::Event::Text("q".into())],
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        assert!(!exit_key(ctx));
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(egui::TextEdit::singleline(&mut query).id(id));
        });
    });
    assert_eq!(query, "q");
    let _ = ctx.run(
        egui::RawInput {
            events: vec![event(egui::Key::Escape)],
            ..Default::default()
        },
        |ctx| {
            assert!(exit_key(ctx));
        },
    );
    ctx.memory_mut(|m| m.surrender_focus(id));
    let _ = ctx.run(
        egui::RawInput {
            events: vec![event(egui::Key::Q)],
            ..Default::default()
        },
        |ctx| {
            assert!(exit_key(ctx));
        },
    );
}
