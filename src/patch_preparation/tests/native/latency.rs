//! Device-free browser dispatch and real startup/shutdown restoration acceptance.
use super::*;
use sha2::{Digest, Sha256};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn rms(samples: &[f32]) -> f64 {
    (samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
}

fn six_sines() -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "org.baconpaul.six-sines".into(),
        name: "Six Sines".into(),
        vendor: "BaconPaul".into(),
        bundle_path: std::env::var("PATCH_LATENCY_BUNDLE").unwrap(),
    }
}

fn patch(key: &PluginKey, name: &str) -> Prepared {
    let path = PathBuf::from(std::env::var_os("PATCH_LATENCY_FACTORY").unwrap())
        .join("Templates")
        .join(format!("{name}.sxsnp"));
    let state = crate::state_request::catalog(&key.id, Path::new(&key.bundle_path), &path).unwrap();
    let mut prepared = prepared(key, &state);
    prepared.candidate.display = name.into();
    prepared.candidate.path = path;
    prepared.candidate.bundle_path = key.bundle_path.clone().into();
    prepared
}

#[test]
#[ignore = "requires Six Sines, PATCH_LATENCY_BUNDLE/FACTORY and S06_NATIVE_WORK_DIR; memory audio"]
fn native_browser_memory_latency_and_restart() {
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap()).join("latency");
    let path = root.join("config.toml");
    let key = six_sines();
    let mut app = app(&path);
    app.mml_input = Default::default();
    add(&mut app, &key);
    let ctx = egui::Context::default();
    crate::session::headless::enable();
    let _sink = crate::session::headless::Guard;
    let mut records = Vec::new();
    let mut sounds = Vec::new();
    let mut first_sounds = Vec::new();
    let mut signatures = Vec::new();
    let mut previous = None;
    for name in ["INIT Sine", "INIT EP", "INIT Sine"] {
        let selected = patch(&key, name);
        let expected = selected.state;
        let start = Instant::now();
        app.patch_browser.open = true;
        app.patch_browser.requests.select(selected.candidate, 0);
        let mut max_dispatch = Duration::ZERO;
        let deadline = start + Duration::from_secs(30);
        loop {
            assert!(Instant::now() < deadline, "browser dispatch timeout");
            let frame = Instant::now();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                app.host.pump_startup();
                app.handle_events();
                app.poll_browser_patch(ctx);
            });
            max_dispatch = max_dispatch.max(frame.elapsed());
            if app
                .patch_browser
                .requests
                .applied
                .as_ref()
                .is_some_and(|c| c.display == name)
                && !app.preset_change_busy()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let applied = start.elapsed();
        let id = app.instrument_id().unwrap();
        if let Some(previous) = previous {
            assert_eq!(id, previous, "reuse the same plugin instance");
        }

        assert_eq!(state_store::load(&path, &key).unwrap(), Some(expected));
        assert_eq!(
            state_store::load_preset_name(&path, &key).as_deref(),
            Some(name)
        );
        let mut sound = Vec::new();
        let mut first_signal = None;
        let mut block_times = Vec::new();
        for block in 0..100 {
            let output = crate::session::headless::render();
            if first_signal.is_none() && output.iter().any(|s| s.abs() > 1e-5) {
                first_signal = Some(start.elapsed().as_secs_f64() * 1000.0);
                first_sounds.push(rms(&output));
            }
            block_times.push(start.elapsed().as_secs_f64() * 1000.0);
            sound.extend(output);
            app.host.pump_startup();
            let due =
                start + applied + Duration::from_secs_f64((block + 1) as f64 * 1024.0 / 48000.0);
            if let Some(wait) = due.checked_duration_since(Instant::now()) {
                std::thread::sleep(wait);
            }
        }
        assert!(
            first_signal.is_some(),
            "new patch must produce memory audio"
        );
        let features: Vec<_> = sound
            .chunks(2048)
            .take(10)
            .map(|block| {
                let sum: f64 = block.iter().map(|s| f64::from(*s).powi(2)).sum();
                let slope: f64 = block
                    .chunks_exact(2)
                    .collect::<Vec<_>>()
                    .windows(2)
                    .map(|w| {
                        (f64::from(w[1][0]) - f64::from(w[0][0])).powi(2)
                            + (f64::from(w[1][1]) - f64::from(w[0][1])).powi(2)
                    })
                    .sum();
                (rms(block), (slope / sum.max(f64::MIN_POSITIVE)).sqrt())
            })
            .collect();
        // The first reuse block can contain the previous voice's release/queued
        // buffer. Identify the new note by its stable first-note spectral shape,
        // then independently compare the A-B-A signatures below.
        let mut reference: Vec<_> = features[4..10].iter().map(|f| f.1).collect();
        reference.sort_by(f64::total_cmp);
        let signature = reference[reference.len() / 2];
        let first_new = features
            .iter()
            .position(|&(level, brightness)| {
                level > 1e-5 && (brightness - signature).abs() / signature < 0.2
            })
            .expect("new patch signature must occur in the first note");
        signatures.push(signature);
        records.push(serde_json::json!({
            "patch": name, "first_instance": previous.is_none(),
            "state_sha256": digest(&state_store::load(&path, &key).unwrap().unwrap()),
            "selected_to_saved_ms": applied.as_secs_f64() * 1000.0,
            "selected_to_first_memory_signal_ms": first_signal,
            "selected_to_first_new_sound_ms": block_times[first_new],
            "first_new_sound_block": first_new,
            "first_note_brightness": signature,
            "max_headless_dispatch_ms": max_dispatch.as_secs_f64() * 1000.0,
            "spans_ms": crate::session::headless::spans(),
            "preparation_worker_ms": crate::session::headless::preparations(),
        }));
        let rms = rms(&sound);
        assert!(rms > 1e-5);
        eprintln!("browser {name}: applied={applied:?} max headless egui dispatch={max_dispatch:?} memory RMS={rms}");
        sounds.push(rms);
        previous = Some(id);
    }
    assert!(
        (sounds[0] - sounds[1]).abs() > 0.001,
        "different patches must sound different"
    );
    assert!(
        (sounds[0] - sounds[2]).abs() < 0.001,
        "A-B-A must return to A"
    );
    eprintln!("first nonzero block RMS A-B-A: {first_sounds:?}");
    assert!((signatures[0] - signatures[1]).abs() / signatures[0] > 0.4);
    assert!((signatures[0] - signatures[2]).abs() / signatures[0] < 0.1);
    app.patch_browser.close();
    app.save_on_shutdown();
    crate::session::headless::clear();
    drop(app);
    let status = crate::status::Status::load(&path).unwrap();
    let mut restored = super::app(&path);
    let _restored_sink = crate::session::headless::Guard;
    restored.restore = status.last_played;
    restored.restore_effect = status.effects.into();
    restored.restore_before_gui();
    wait(&mut restored);
    assert_eq!(
        restored.instances[0].preset_name.as_deref(),
        Some("INIT Sine")
    );
    let mut restored_sound = Vec::new();
    for _ in 0..100 {
        restored_sound.extend(crate::session::headless::render());
        restored.host.pump_startup();
    }
    assert!(restored_sound.iter().any(|s| s.abs() > 1e-5));
    crate::session::headless::disable();
    let report = serde_json::json!({
        "conditions": "48kHz/1024/stereo/Steps velocity100/no effects/device/window; real browser worker/transaction/persistence and retained processor, shared sequencer+macro sender; paced memory blocks; excludes cpal callback/window/OS delivery",
        "executable": std::env::current_exe().unwrap(),
        "executable_sha256": digest(&std::fs::read(std::env::current_exe().unwrap()).unwrap()),
        "bundle_sha256": digest(&std::fs::read(&key.bundle_path).unwrap()),
        "bundle": key.bundle_path, "factory": std::env::var("PATCH_LATENCY_FACTORY").unwrap(),
        "records": records, "rms_aba": sounds, "first_nonzero_block_rms_aba": first_sounds,
        "onset_detector": "first 1024-frame block matching median first-note brightness (blocks4..9), relative error <20%; distinct B and returning A verified; excludes prior release/queued buffer",
        "restart": "passed",
    });
    let report_path =
        std::env::var_os("PATCH_LATENCY_JSON").expect("external JSON output required");
    std::fs::write(report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}

#[test]
#[ignore = "touches real user session; external backup guard must set PATCH_LATENCY_REAL_PHASE/WORK; no device or GUI"]
fn native_real_session_browser_and_restart() {
    let phase = std::env::var("PATCH_LATENCY_REAL_PHASE").unwrap();
    assert!(std::env::var_os(crate::native_library::process::ROOT).is_none());
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.load_plugin_cache();
    app.restore_before_gui();
    wait(&mut app);
    assert!(
        app.instrument_id().is_some(),
        "normal startup must restore the actual session"
    );
    let work = PathBuf::from(std::env::var_os("PATCH_LATENCY_REAL_WORK").unwrap());
    if phase == "apply" {
        let key = six_sines();
        add(&mut app, &key);
        let effects = app.effect_ids();
        let bypass = app.effect_bypassed;
        let velocity = app.sequence_velocity;
        let selected = app.selected_sequence;
        let patch = patch(&key, "INIT EP");
        app.apply_patch(patch);
        wait(&mut app);
        assert!(app.status.contains("Browse patch:"), "{}", app.status);
        assert_eq!(app.effect_ids(), effects);
        assert_eq!(app.effect_bypassed, bypass);
        assert_eq!(app.sequence_velocity, velocity);
        assert_eq!(app.selected_sequence, selected);
        let audio = super::real_surge::render(&app, app.instrument_id().unwrap());
        assert!(audio.iter().any(|s| s.abs() > 1e-5));
        app.save_on_shutdown();
        std::fs::write(
            work.join("applied-status.json"),
            std::fs::read(crate::status::path(app.config_path.as_ref().unwrap())).unwrap(),
        )
        .unwrap();
    } else {
        assert_eq!(phase, "restart");
        assert_eq!(app.instances[0].plugin.id, "org.baconpaul.six-sines");
        assert_eq!(app.instances[0].preset_name.as_deref(), Some("INIT EP"));
        // In production the output callback installs Six Sines' queued state.
        // This device-free test must process it before reading the restored sound.
        let audio = super::real_surge::render(&app, app.instrument_id().unwrap());
        assert!(audio.iter().any(|s| s.abs() > 1e-5));
        let sound = app.host.save_state(app.instrument_id().unwrap()).unwrap();
        let key = six_sines();
        let expected = state_store::load(app.config_path.as_ref().unwrap(), &key)
            .unwrap()
            .unwrap();
        assert!(crate::plugin_specific::same_favorite_state(
            &key, &sound, &expected
        ));
        app.save_on_shutdown();
    }
}
