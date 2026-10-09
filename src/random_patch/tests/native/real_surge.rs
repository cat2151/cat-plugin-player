//! Opt-in installed Surge acceptance; all application writes use the supplied directory.
use super::*;

fn render(app: &App, id: i32) -> Vec<f32> {
    let processor = app.host.create_chain(id, &[], false, 48_000, 256).unwrap();
    let mut audio = unsafe { processor.audio_ref() };
    let mut sequencer = crate::seq::Sequencer::new(48_000);
    sequencer.delay_start(crate::plugin_specific::automatic_note_start_delay(
        &app.instances
            .iter()
            .find(|instance| instance.id == id)
            .unwrap()
            .plugin,
    ));
    let mut events = crate::seq::EventBuf::new();
    let mut samples = Vec::new();
    let start = Instant::now();
    for block in 0..100 {
        events.clear();
        sequencer.render(
            SequencePattern::Steps,
            crate::SequenceVelocity::V100,
            256,
            &mut events,
        );
        let mut output = [0.0; 512];
        assert!(audio.process(events.as_slice(), &mut output, 2));
        samples.extend(output);
        app.host.pump_startup();
        let due = start + Duration::from_secs_f64((block + 1) as f64 * 256.0 / 48_000.0);
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
    }
    samples
}

#[test]
#[ignore = "requires installed Surge XT CLAP, factory Bass 1/2 and S07_NATIVE_WORK_DIR"]
fn native_real_surge_random_state_sound_and_stopped_play() {
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S07_NATIVE_WORK_DIR").unwrap());
    let bundle = std::env::var_os("S07_SURGE_BUNDLE").unwrap();
    let patches = PathBuf::from(std::env::var_os("S07_SURGE_PATCHES").unwrap());
    let surge = PluginKey {
        format: "CLAP".into(),
        id: crate::random_patch_catalog::SURGE_ID.into(),
        name: "Surge XT".into(),
        vendor: "Surge Synth Team".into(),
        bundle_path: PathBuf::from(bundle).to_string_lossy().into_owned(),
    };
    let path = root.join("config.toml");
    let mut app = app(&path);
    let index = add(&mut app, &surge);
    app.load_plugin(index);
    wait(&mut app);
    let id = app.instrument_id().unwrap();
    let first =
        cmrt_patches::prepare_clap_patch_state(&surge.id, &patches.join("Bass 1.fxp")).unwrap();
    app.host.load_state(id, &first).unwrap();
    let first_audio = render(&app, id);
    let before = app.host.save_state(id).unwrap();
    app.sequence_pattern = SequencePattern::Off;
    app.selected_sequence = SequencePattern::Steps;
    let second =
        cmrt_patches::prepare_clap_patch_state(&surge.id, &patches.join("Bass 2.fxp")).unwrap();
    app.apply_random_patch(prepared(&surge, &second));
    wait(&mut app);
    assert!(app.status.contains("Random patch:"), "{}", app.status);
    assert_eq!(app.instrument_id(), Some(id));
    assert_eq!(app.sequence_pattern, SequencePattern::Off);
    let saved = state_store::load(&path, &surge).unwrap().unwrap();
    assert_eq!(
        saved, second,
        "persist the accepted patch before audio-thread settling"
    );
    let second_audio = render(&app, id);
    let after = app.host.save_state(id).unwrap();
    assert!(!crate::plugin_specific::same_favorite_state(
        &surge, &before, &after
    ));
    // Surge upgrades factory revision 20 to revision 24 on save; metadata and
    // rendered output establish the known patch without pretending bytes match.
    assert!(String::from_utf8_lossy(&after).contains("name=\"Bass two\""));
    assert!(String::from_utf8_lossy(&saved).contains("name=\"Bass two\""));
    let rms = |samples: &[f32]| {
        (samples.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
    };
    let difference: Vec<_> = first_audio
        .iter()
        .zip(&second_audio)
        .map(|(a, b)| a - b)
        .collect();
    eprintln!(
        "real Surge Bass 1/2: state {} -> {} bytes; RMS {} -> {}; difference RMS {}",
        before.len(),
        after.len(),
        rms(&first_audio),
        rms(&second_audio),
        rms(&difference)
    );
    assert!(rms(&first_audio) > 0.000001 && rms(&second_audio) > 0.000001);
    assert!(rms(&difference) > 0.001);
}
