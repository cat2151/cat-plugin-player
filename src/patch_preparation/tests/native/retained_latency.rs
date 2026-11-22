//! Same browser worker, transaction, persistence and processor for both entry points.
use super::*;
use crate::{patch_browser_catalog::Snapshot, session::headless};

fn candidate_for(key: &PluginKey, path: PathBuf, label: &str) -> crate::patch_catalog::Candidate {
    let mut c = prepared(key, &[]).candidate;
    c.display = label.into();
    c.entry = cmrt_patch_select::PatchCatalogEntry::from_display(label.into());
    c.path = path;
    c.bundle_path = (&key.bundle_path).into();
    c
}

fn six_patch_name(state: &[u8]) -> String {
    let mut reader = quick_xml::Reader::from_reader(state);
    let quick_xml::events::Event::Start(root) = reader.read_event().unwrap() else {
        panic!("Six Sines state requires patch XML");
    };
    root.attributes()
        .map(Result::unwrap)
        .find(|a| a.key.as_ref() == b"name")
        .unwrap()
        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
        .unwrap()
        .into_owned()
}

fn press_random(app: &mut App, ctx: &egui::Context) {
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::R,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
            ..Default::default()
        },
        |ctx| {
            crate::main_menu::keyboard(ctx, true);
            egui::CentralPanel::default().show(ctx, |ui| app.playback_controls(ui));
        },
    );
    assert!(
        app.patch_browser.requests.desired.is_some(),
        "R must select the sole candidate"
    );
}

fn dispatch(
    app: &mut App,
    ctx: &egui::Context,
    selected: &crate::patch_catalog::Candidate,
    random: bool,
) -> Duration {
    // A singleton result makes actual R input deterministic without replacing RNG.
    app.patch_browser.snapshot = Some(Snapshot::build(vec![selected.clone()], &[]).unwrap());
    app.patch_browser.results = vec![0];
    app.patch_browser.selected = None;
    app.patch_browser.open = !random;
    let start = Instant::now();
    if random {
        press_random(app, ctx);
    } else {
        app.patch_browser.select(0);
    }
    let deadline = start + Duration::from_secs(40);
    loop {
        assert!(Instant::now() < deadline, "dispatch: {}", app.status);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            app.host.pump_startup();
            app.handle_events();
            app.poll_browser_patch(ctx);
        });
        if app
            .patch_browser
            .requests
            .applied
            .as_ref()
            .is_some_and(|c| c.display == selected.display)
            && !app.preset_change_busy()
        {
            break;
        }
        assert!(!app.status.contains("failed"), "{}", app.status);
        std::thread::sleep(Duration::from_millis(1));
    }
    start.elapsed()
}

fn sound(app: &App) -> (f64, Vec<f64>) {
    let start = Instant::now();
    let mut signature = Vec::new();
    let mut first = None;
    for block in 0..100 {
        let output = headless::render();
        let energy = output.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
        let rms = (energy / output.len() as f64).sqrt();
        let slope = output
            .as_chunks::<2>()
            .0
            .windows(2)
            .map(|w| {
                (f64::from(w[1][0]) - f64::from(w[0][0])).powi(2)
                    + (f64::from(w[1][1]) - f64::from(w[0][1])).powi(2)
            })
            .sum::<f64>();
        // Ignore early release/queued buffers. State identity is checked after
        // rendering too; the late-note signature is compared across A/B repeats.
        if (35..80).contains(&block) && rms > 1e-5 {
            signature.push((slope / energy).sqrt());
            first.get_or_insert(start.elapsed().as_secs_f64() * 1000.0);
        }
        app.host.pump_startup();
        let due = start + Duration::from_secs_f64((block + 1) as f64 * 1024.0 / 48000.0);
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
    }
    assert!(
        !signature.is_empty(),
        "selected patch must sustain memory audio after settling"
    );
    signature.sort_by(f64::total_cmp);
    (first.unwrap(), signature)
}

#[test]
#[ignore = "requires Surge/Six Sines and PATCH_LATENCY_* paths; isolated memory audio"]
fn native_retained_browser_memory_round_trip_and_restart() {
    let _library = crate::native_library::load().unwrap();
    let baseline = std::env::var_os("PATCH_RETENTION_BASELINE").is_some();
    let root = PathBuf::from(std::env::var_os(crate::native_library::process::ROOT).unwrap());
    let path = root.join("config.toml");
    let surge = PluginKey {
        format: "CLAP".into(),
        id: crate::patch_catalog::SURGE_ID.into(),
        name: "Surge XT".into(),
        vendor: "Surge Synth Team".into(),
        bundle_path: std::env::var("PATCH_LATENCY_SURGE_BUNDLE").unwrap(),
    };
    let six = PluginKey {
        format: "CLAP".into(),
        id: "org.baconpaul.six-sines".into(),
        name: "Six Sines".into(),
        vendor: "BaconPaul".into(),
        bundle_path: std::env::var("PATCH_LATENCY_BUNDLE").unwrap(),
    };
    let surge_factory = PathBuf::from(std::env::var_os("PATCH_LATENCY_SURGE_PATCHES").unwrap());
    let six_factory = PathBuf::from(std::env::var_os("PATCH_LATENCY_FACTORY").unwrap());
    let choices = [
        candidate_for(&surge, surge_factory.join("Bass 1.fxp"), "Surge Bass 1"),
        candidate_for(
            &six,
            six_factory.join("Templates/INIT Sine.sxsnp"),
            "Six INIT Sine",
        ),
        candidate_for(&surge, surge_factory.join("Bass 2.fxp"), "Surge Bass 2"),
        candidate_for(
            &six,
            six_factory.join("Templates/INIT EP.sxsnp"),
            "Six INIT EP",
        ),
    ];
    let mut app = app(&path);
    app.mml_input = Default::default();
    add(&mut app, &surge);
    add(&mut app, &six);
    headless::enable();
    let _sink = headless::Guard;
    let ctx = egui::Context::default();
    let mut ids = std::collections::HashMap::new();
    let mut normalized_states = std::collections::HashMap::<String, Vec<u8>>::new();
    let mut signatures = Vec::new();
    let mut records = Vec::new();
    let mut effects = Vec::new();
    for iteration in 0..8 {
        let selected = &choices[iteration % 4];
        let expected = crate::state_request::catalog(
            &selected.plugin_id,
            &selected.bundle_path,
            &selected.path,
        )
        .unwrap();
        headless::spans();
        headless::preparations();
        let applied = dispatch(&mut app, &ctx, selected, iteration % 2 == 0);
        let id = app.instrument_id().unwrap();
        let old = ids.insert(selected.plugin_id.clone(), id);
        if let Some(old) = old {
            assert_eq!(old == id, !baseline);
        }
        let spans = headless::spans();
        assert_eq!(
            spans.iter().filter(|(s, _)| *s == "create_request").count(),
            usize::from(old.is_none() || baseline)
        );
        let key = if iteration % 2 == 0 { &surge } else { &six };
        assert_eq!(state_store::load(&path, key).unwrap(), Some(expected));
        let (settled_signal, signature) = sound(&app);
        let median = signature[signature.len() / 2];
        signatures.push(median);
        let actual = app.host.save_state(id).unwrap();
        if iteration % 2 == 0 {
            let name = if iteration % 4 == 0 {
                "name=\"Bass one\""
            } else {
                "name=\"Bass two\""
            };
            assert!(
                String::from_utf8_lossy(&actual).contains(name),
                "wrong installed Surge patch"
            );
        } else {
            let persisted = state_store::load(&path, &six).unwrap().unwrap();
            // Installed Six Sines upgrades factory schema 6 to 12. Check the
            // chosen name, then compare normalized native state on its return.
            assert_eq!(six_patch_name(&actual), six_patch_name(&persisted));
            if let Some(previous) =
                normalized_states.insert(selected.display.clone(), actual.clone())
            {
                assert!(
                    crate::plugin_specific::same_favorite_state(&six, &actual, &previous),
                    "return must reproduce the normalized selected state"
                );
            }
        }
        assert_eq!(app.effect_ids(), effects);
        if iteration > 0 {
            assert!(app.effect_bypassed);
            assert_eq!(app.sequence_velocity, crate::SequenceVelocity::Range40To100);
            assert_eq!(
                app.sequence_modulation,
                Some(crate::SequenceModulation::Sweep)
            );
            assert_eq!(app.selected_sequence, SequencePattern::Steps);
        }
        assert!(app.retained_instruments.len() <= 1);
        assert!(app
            .retained_instruments
            .iter()
            .all(|r| r.instance.voice.is_none() && !app.host.ui_is_visible(r.instance.id)));
        records.push(serde_json::json!({"patch": selected.display, "id":id,
            "selected_to_saved_ms":applied.as_secs_f64()*1000.0,
            "selected_to_settled_memory_signal_ms":applied.as_secs_f64()*1000.0+settled_signal,
            "brightness":median,"spans_ms":spans,"preparation_worker_ms":headless::preparations()}));
        if iteration == 0 {
            // Connected effects and playback must survive all subsequent returns.
            let fx = add(&mut app, &key_for_effect());
            app.plugins[fx].kind = PluginKind::Effect;
            app.load_plugin(fx);
            wait(&mut app);
            effects = app.effect_ids();
            // The fixture effect emits silence: bypass it while preserving a
            // connected effect identity and the saved bypass contract.
            app.set_effect_bypass(true);
            app.sequence_velocity = crate::SequenceVelocity::Range40To100;
            app.sequence_modulation = Some(crate::SequenceModulation::Sweep);
            app.selected_sequence = SequencePattern::Steps;
            app.save_session();
        }
    }
    for i in 0..4 {
        assert!(
            (signatures[i] - signatures[i + 4]).abs() / signatures[i] < 0.25,
            "repeat must reproduce settled patch signature: {signatures:?}"
        );
    }
    for i in 0..2 {
        assert!(
            (signatures[i] - signatures[i + 2]).abs() / signatures[i] > 0.1,
            "A/B must have distinct settled signatures: {signatures:?}"
        );
    }
    app.patch_browser.close();
    app.save_on_shutdown();
    let saved = crate::status::Status::load(&path).unwrap();
    drop(app);
    headless::enable();
    let mut restored = App::prepare();
    restored.output = None;
    restored.restore_before_gui();
    wait(&mut restored);
    assert_eq!(restored.instances.len(), 2);
    assert!(restored.retained_instruments.is_empty());
    assert_eq!(restored.instances[0].plugin.id, six.id);
    assert_eq!(
        restored.instances[0].preset_name.as_deref(),
        Some("Six INIT EP")
    );
    assert_eq!(restored.sequence_velocity, saved.sequence_velocity);
    assert_eq!(restored.sequence_modulation, saved.sequence_modulation);
    assert_eq!(restored.effect_bypassed, saved.effect_bypassed);
    assert_eq!(restored.selected_sequence, saved.selected_sequence);
    assert_eq!(restored.instances[1].plugin.id, key_for_effect().id);
    let (_, signature) = sound(&restored);
    let brightness = signature[signature.len() / 2];
    assert!(
        (brightness - signatures[7]).abs() / signatures[7] < 0.25,
        "restart must reproduce the last selected sound"
    );
    let actual = restored
        .host
        .save_state(restored.instrument_id().unwrap())
        .unwrap();
    let persisted = state_store::load(&path, &six).unwrap().unwrap();
    assert!(crate::plugin_specific::same_favorite_state(
        &six, &actual, &persisted
    ));
    drop(restored);
    std::fs::write(std::env::var_os("PATCH_LATENCY_JSON").unwrap(), serde_json::to_vec_pretty(&serde_json::json!({
        "baseline_destroy_on_switch":baseline,"records":records,"signatures":signatures,"restart":"passed",
        "conditions":"48kHz/1024/stereo/paced memory audio; actual R and browser selection/worker/transaction/store/processor/sequencer; fixture effect after first selection; memory sink uses fixed Steps/V100 and normal automatic note delay, production uses selected transport/velocity and authorized Surge delay; excludes device/OS/UI delivery",
        "onset_detector":"first sustained nonzero block in settled window 35..79; conservative observation, not first note onset; patch state plus distinct A/B and returning spectral signatures reject old release/buffer evidence"
    })).unwrap()).unwrap();
}

fn key_for_effect() -> PluginKey {
    key("effect")
}
