use super::*;

// Exercise the application transitions and real DSP without opening a stream or
// writing user history. Run this filter separately from other native-host tests.
#[test]
#[ignore = "requires installed Dexed, Dragonfly Hall Reverb and Surge XT VST3"]
fn native_effect_routing_and_session_restore() {
    use crate::plugin_list::PluginKind;
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.restore = None;
    app.restore_effect = Default::default();
    app.output = None;
    app.deferred_scan = false;
    let directory = std::env::temp_dir().join(format!("cat-routing-{}", std::process::id()));
    assert!(
        !directory.exists(),
        "temporary test directory already exists"
    );
    let path = directory.join("config.toml");
    app.config_path = Ok(path.clone());
    app.start_scan(false);
    settle(&mut app);
    let instrument = app
        .plugins
        .iter()
        .position(|p| p.name == "Dexed" && p.format == "CLAP")
        .unwrap();
    let effect = app
        .plugins
        .iter()
        .position(|p| p.name == "Dragonfly Hall Reverb" && p.format == "CLAP")
        .unwrap();
    let vst_instrument = app
        .plugins
        .iter()
        .position(|p| p.name == "Surge XT" && p.format == "VST3")
        .unwrap();
    let vst_effect = app
        .plugins
        .iter()
        .position(|p| p.name == "Surge XT Effects" && p.format == "VST3")
        .unwrap();
    app.load_plugin(effect);
    assert!(app.pending.is_none() && app.instances.is_empty());
    app.load_plugin(instrument);
    settle(&mut app);
    let source = app.instrument_id().unwrap();
    let source_key = app.instances[0].plugin.clone();
    let dry = capture(&app, source, None, false);
    app.load_plugin(effect);
    settle(&mut app);
    assert_eq!(app.instrument_id(), Some(source));
    let fx = app.effect_id().unwrap();
    let fx_key = app.instances[1].plugin.clone();
    assert_eq!(app.instances.len(), 2);
    let session = crate::status::Status::load(&path).unwrap();
    assert_eq!(session.last_played, Some(source_key.clone()));
    assert_eq!(session.effects, vec![fx_key.clone()]);
    let wet = capture(&app, source, Some(fx), false);
    let bypassed = capture(&app, source, Some(fx), true);
    let energy = |samples: &[f32]| samples.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
    let tail = |samples: &[f32]| energy(&samples[48_000 * 2..]);
    eprintln!(
        "routing: dry_tail={}, wet_tail={}, bypass_tail={}",
        tail(&dry),
        tail(&wet),
        tail(&bypassed)
    );
    assert!(
        energy(&dry) > 0.001 && energy(&wet) > 0.001,
        "audio input did not reach effect output"
    );
    assert!(
        tail(&wet) > tail(&dry) * 2.0 + 0.001,
        "effect produced no measurable reverb tail"
    );
    assert!(
        tail(&bypassed) < tail(&wet) * 0.5,
        "bypass still processed the effect"
    );

    let before = app.host.save_state(fx).unwrap();
    app.load_plugin(vst_instrument);
    settle(&mut app);
    assert_ne!(app.instrument_id(), Some(source));
    assert_eq!(app.effect_id(), Some(fx));
    assert_eq!(app.host.save_state(fx).unwrap(), before);
    let cross = capture(&app, app.instrument_id().unwrap(), Some(fx), false);
    assert!(
        energy(&cross) > 0.001,
        "VST3 instrument -> CLAP effect failed"
    );
    app.load_plugin(vst_effect);
    settle(&mut app);
    assert_eq!(app.effect_id(), Some(fx));
    assert_eq!(app.instances.len(), 3);
    let vst_fx = app.effect_ids()[1];
    app.load_plugin(instrument);
    settle(&mut app);
    let cross = capture(&app, app.instrument_id().unwrap(), Some(vst_fx), false);
    assert!(
        energy(&cross) > 0.001,
        "CLAP instrument -> VST3 effect failed"
    );

    // Reject a missing native index without destroying either old slot or history.
    let old_source = app.instrument_id();
    let old_fx = app.effect_id();
    let previous_config = std::fs::read(crate::status::path(&path)).unwrap();
    let mut invalid = app.plugins[effect].clone();
    invalid.index = i32::MAX;
    invalid.id = "missing.invalid.effect".into();
    app.plugins.push(invalid);
    app.load_plugin(app.plugins.len() - 1);
    settle(&mut app);
    assert_eq!(app.instrument_id(), old_source);
    assert_eq!(app.effect_id(), old_fx);
    assert_eq!(
        std::fs::read(crate::status::path(&path)).unwrap(),
        previous_config
    );
    assert!(app.status.contains("Could not load"));

    // Metadata alone cannot make an input-less synth a valid audio effect.
    let mut no_input = app.plugins[instrument].clone();
    no_input.kind = PluginKind::Effect;
    app.plugins.push(no_input);
    app.load_plugin(app.plugins.len() - 1);
    settle(&mut app);
    assert_eq!(app.instrument_id(), old_source);
    assert_eq!(app.effect_id(), old_fx);
    assert!(app.status.contains("Could not connect"));
    assert_eq!(
        std::fs::read(crate::status::path(&path)).unwrap(),
        previous_config
    );
    assert!(app
        .host
        .create_chain(
            old_source.unwrap(),
            &[old_source.unwrap()],
            false,
            48_000,
            1024
        )
        .is_none());

    // A save failure leaves the old chain intact.
    let blocker = directory.join("blocked");
    std::fs::write(&blocker, b"not a directory").unwrap();
    app.config_path = Ok(blocker.join("config.toml"));
    assert!(!app.remove_plugin(old_fx.unwrap()));
    assert_eq!(app.effect_id(), old_fx);
    app.config_path = Ok(path.clone());
    assert!(app.remove_plugin(old_fx.unwrap()));
    assert_eq!(app.effect_ids(), vec![vst_fx]);
    assert!(app.remove_plugin(vst_fx));
    assert!(app.effect_ids().is_empty());
    assert!(crate::status::Status::load(&path)
        .unwrap()
        .effects
        .is_empty());
    app.load_plugin(effect);
    settle(&mut app);
    // B toggles bypass from the main window, but not while the MML editor takes keys.
    app.mml_input.open = true;
    press_b(&mut app);
    assert!(!app.effect_bypassed);
    app.mml_input.open = false;
    press_b(&mut app);
    assert!(app.effect_bypassed);
    let saved = crate::status::Status::load(&path).unwrap();
    assert!(saved.effect_bypassed);
    let fx_state = app.host.save_state(app.effect_id().unwrap()).unwrap();
    let source_state = app.host.save_state(app.instrument_id().unwrap()).unwrap();
    drop(app);

    let mut next = App::prepare();
    next.output = None;
    next.deferred_scan = false;
    next.config_path = Ok(path.clone());
    next.restore = saved.last_played;
    next.restore_effect = saved.effects.into();
    next.effect_bypassed = saved.effect_bypassed;
    next.restore_before_gui();
    settle(&mut next);
    assert_eq!(next.instances.len(), 2, "{}", next.status);
    assert_eq!(next.instances[0].kind, PluginKind::Instrument);
    assert_eq!(next.instances[1].kind, PluginKind::Effect);
    assert!(next.effect_bypassed);
    assert_eq!(
        next.host.save_state(next.instrument_id().unwrap()).unwrap(),
        source_state
    );
    assert_eq!(
        next.host.save_state(next.effect_id().unwrap()).unwrap(),
        fx_state
    );
    assert!(next.restore.is_none() && next.restore_effect.is_empty() && !next.restoring);
    drop(next);
    // The live WASAPI callback owns one stream for both slots. Rebuilding it
    // for Bypass and Remove must not leave a held note or race a state API.
    let mut live = restore_app(&path);
    assert!(live.output.is_some(), "no usable default audio device");
    live.restore_before_gui();
    settle(&mut live);
    assert_eq!(live.instances.len(), 2);
    assert!(live.instances[0].voice.as_ref().unwrap().has_rendered());
    assert!(live.instances[1].voice.is_none());
    live.instances[0]
        .voice
        .as_ref()
        .unwrap()
        .set_sequence(crate::SequencePattern::Off);
    live.set_effect_bypass(false);
    assert_eq!(live.sequence_pattern, crate::SequencePattern::Off);
    assert_eq!(
        live.instances[0].voice.as_ref().unwrap().sequence_pattern(),
        crate::SequencePattern::Off
    );
    live.pause_audio();
    drop(live);

    // A moved effect uses catalog fallback before the chain is ready to play.
    let mut relocated = crate::status::Status::load(&path).unwrap();
    relocated.effects[0].bundle_path = "X:/missing/reverb.clap".into();
    relocated.save(&path).unwrap();
    let mut moved = restore_app(&path);
    moved.output = None;
    moved.restore_before_gui();
    assert!(moved.restoring && !moved.restore_effect.is_empty());
    moved.start_scan(false);
    settle(&mut moved);
    assert_eq!(moved.instances.len(), 2, "{}", moved.status);
    assert!(!moved.restoring);
    drop(moved);

    // A genuinely missing effect reports the failure, plays the instrument
    // directly and preserves the saved session instead of overwriting it.
    let mut missing = crate::status::Status::load(&path).unwrap();
    let key = &mut missing.effects[0];
    key.id = "missing.effect.for.test".into();
    key.bundle_path = "X:/missing/effect.clap".into();
    missing.effect_bypassed = true;
    missing.save(&path).unwrap();
    let bytes = std::fs::read(crate::status::path(&path)).unwrap();
    let mut absent = restore_app(&path);
    absent.output = None;
    absent.restore_before_gui();
    absent.start_scan(false);
    settle(&mut absent);
    assert_eq!(absent.instances.len(), 1);
    assert!(!absent.restoring && !absent.effect_bypassed);
    assert!(absent
        .config_error
        .as_ref()
        .unwrap()
        .contains("Previous effect not found"));
    assert_eq!(std::fs::read(crate::status::path(&path)).unwrap(), bytes);
    drop(absent);
    std::fs::remove_dir_all(directory).unwrap();
}

fn press_b(app: &mut App) {
    use eframe::egui;
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::B,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = egui::Context::default().run(input, |ctx| app.routing_panel(ctx, false));
}

fn settle(app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while app.pending.is_some() || app.scanning {
        assert!(
            std::time::Instant::now() < deadline,
            "callback timed out: {}",
            app.status
        );
        app.host.pump_startup();
        app.handle_events();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn capture(app: &App, source: i32, effect: Option<i32>, bypass: bool) -> Vec<f32> {
    let effects: Vec<_> = effect.into_iter().collect();
    let processor = app
        .host
        .create_chain(source, &effects, bypass, 48_000, audio::MAX_BLOCK_FRAMES)
        .unwrap();
    let mut renderer = unsafe { processor.audio_ref() };
    let mut block = [0.0; 2048];
    // Start from released notes, including if a previous capture was run.
    assert!(renderer.process(&[0x20B0_7800], &mut block, 2));
    let mut samples = Vec::new();
    for i in 0..96 {
        let events = match i {
            0 => vec![crate::seq::note_on(0, 60, 100)],
            16 => vec![crate::seq::note_off(0, 60)],
            _ => Vec::new(),
        };
        assert!(renderer.process(&events, &mut block, 2));
        samples.extend_from_slice(&block);
    }
    samples
}

fn restore_app(path: &std::path::Path) -> App {
    let saved = crate::status::Status::load(path).unwrap();
    let mut app = App::prepare();
    app.config_path = Ok(path.to_owned());
    app.restore = saved.last_played;
    app.restore_effect = saved.effects.into();
    app.effect_bypassed = saved.effect_bypassed;
    app.deferred_scan = false;
    app
}

#[path = "routing_chain_tests.rs"]
mod chain_tests;
