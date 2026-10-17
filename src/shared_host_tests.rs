//! Contract checks live in the source-owning app's ordinary test build.
use crate::{
    chain_settings::{PluginState, Settings},
    config::PluginKey,
    App,
};
use std::{
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

const CHILD: &str = "CAT_SHARED_HOST_TEST_CHILD";

/// Runs `body` in a fresh test process: a second native host in one process
/// never finishes opening, so each test needs its own process.
fn isolated(name: &str, body: fn()) {
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            &format!("shared_host_tests::{name}"),
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains("1 passed"),
        "{name} failed in child process\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
#[ignore = "requires installed Surge XT"]
fn shared_note_batches_preserve_duration_and_earlier_pitch() {
    isolated(
        "shared_note_batches_preserve_duration_and_earlier_pitch",
        note_batches,
    );
}

fn note_batches() {
    let _library = crate::native_library::load().unwrap();
    let chain = crate::shared_chain::Chain::open(48000, &Settings::default()).unwrap();
    let mut render = crate::shared_note_render::Render::new(unsafe { chain.audio_ref() });
    let mut velocities = [0; 128];
    let mut durations = [0; 128];
    let mut out = [0.0; 2048];
    let mut held = 0.0;
    let mut tail = 0.0;
    for block in 0..160 {
        velocities.fill(0);
        durations.fill(0);
        let change = match block {
            0 => {
                velocities[60] = 100;
                durations[60] = 48000;
                1
            }
            5 => {
                velocities[64] = 80;
                durations[64] = 1024;
                2
            }
            40 => 3,
            _ => 0,
        };
        chain.pump();
        assert!(render.process(&velocities, &durations, change, &mut out, 2));
        let energy = out.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
        if (25..35).contains(&block) {
            held += energy;
        }
        if block > 140 {
            tail += energy;
        }
    }
    assert!(held > 0.01, "later pitch interrupted earlier note");
    assert!(
        tail < held * 0.001,
        "stop failed to release notes: held={held}, tail={tail}"
    );
}

fn state(key: PluginKey, work: &std::path::Path) -> PluginState {
    let mut app = App::prepare();
    app.config_path = Ok(work.join("config.toml"));
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    app.plugins.clear();
    let mut plugin = app.host.restore_plugin(&key).unwrap();
    plugin.kind = crate::plugin_list::PluginKind::Instrument;
    app.plugins.push(plugin);
    app.load_plugin(0);
    let deadline = Instant::now() + Duration::from_secs(20);
    while app.pending.is_some() {
        assert!(Instant::now() < deadline);
        app.host.pump_startup();
        app.handle_events();
        std::thread::sleep(Duration::from_millis(1));
    }
    let id = app.instrument_id().expect(&app.status);
    let state = app.host.save_state(id).unwrap();
    PluginState {
        reference: None,
        plugin: key,
        state,
        preset_name: None,
        bypass: false,
    }
}

#[test]
#[ignore = "requires compiled CLAP fixture"]
fn shared_chain_rejects_failed_state_without_publishing_processor() {
    isolated(
        "shared_chain_rejects_failed_state_without_publishing_processor",
        rejects_failed_state,
    );
}

fn rejects_failed_state() {
    let _library = crate::native_library::load().unwrap();
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
    let mut settings = Settings {
        instrument: Some(PluginState {
            reference: None,
            plugin: key,
            state: vec![0],
            preset_name: None,
            bypass: false,
        }),
        ..Default::default()
    };
    assert!(crate::shared_chain::Chain::open(48000, &settings).is_err());
    settings.instrument.as_mut().unwrap().state = vec![1];
    assert!(crate::shared_chain::Chain::open(48000, &settings).is_ok());
}

#[test]
#[ignore = "requires installed Surge XT + Effects"]
fn shared_chain_real_instrument_effect_and_independent_same_pitch_tracks() {
    isolated(
        "shared_chain_real_instrument_effect_and_independent_same_pitch_tracks",
        || {
            let work =
                std::env::temp_dir().join(format!("cat-shared-chain-{}", std::process::id()));
            real_instrument_effect(&work);
            let _ = std::fs::remove_dir_all(work);
        },
    );
}

fn real_instrument_effect(work: &std::path::Path) {
    let _library = crate::native_library::load().unwrap();
    let source_key = crate::shared_chain::default_instrument().unwrap();
    let mut effect_key = source_key.clone();
    effect_key.id = "org.surge-synth-team.surge-xt-fx".into();
    effect_key.name = "Surge XT Effects".into();
    effect_key.bundle_path = PathBuf::from(&effect_key.bundle_path)
        .with_file_name("Surge XT Effects.clap")
        .to_string_lossy()
        .into_owned();
    let instrument = state(source_key, &work.join("source"));
    let effect = state(effect_key, &work.join("effect"));
    let settings = Settings {
        instrument: Some(instrument),
        effects: vec![effect],
        ..Default::default()
    };
    let first = crate::shared_chain::Chain::open(48000, &settings).unwrap();
    let mut second_settings = settings.clone();
    second_settings.bypass = true;
    let ep = PathBuf::from(std::env::var_os("ProgramData").unwrap())
        .join("Surge XT/patches_factory/Keys/DX EP.fxp");
    let slot = second_settings.instrument.as_mut().unwrap();
    slot.state = cmrt_patches::prepare_clap_patch_state(&slot.plugin.id, &ep).unwrap();
    let second = crate::shared_chain::Chain::open(48000, &second_settings).unwrap();
    let mut a = crate::shared_note_render::Render::new(unsafe { first.audio_ref() });
    let mut b = crate::shared_note_render::Render::new(unsafe { second.audio_ref() });
    let mut velocities = [0; 128];
    velocities[60] = 100;
    let mut durations = [0; 128];
    durations[60] = 48000;
    let mut out = [0.0; 2048];
    let mut energies = [0.0f64; 2];
    for block in 0..20 {
        first.pump();
        second.pump();
        assert!(a.process(&velocities, &durations, i32::from(block == 0), &mut out, 2));
        energies[0] += out.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
        assert!(b.process(&velocities, &durations, i32::from(block == 0), &mut out, 2));
        energies[1] += out.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
    }
    assert!(
        energies.iter().all(|v| *v > 0.001),
        "instrument/effect output: {energies:?}"
    );
    drop(first);
    let mut remaining = 0.0;
    for _ in 0..10 {
        second.pump();
        assert!(b.process(&velocities, &durations, 0, &mut out, 2));
        remaining += out.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
    }
    assert!(
        remaining > 0.001,
        "dropping another same-pitch track stopped this one"
    );
    eprintln!("shared native chain energies: active={energies:?}, surviving track={remaining}");
}

#[test]
#[ignore = "requires installed Surge XT"]
fn shared_chain_reload_sounds_like_fresh_open() {
    isolated("shared_chain_reload_sounds_like_fresh_open", || {
        let work = std::env::temp_dir().join(format!("cat-shared-reload-{}", std::process::id()));
        reload_sounds_like_fresh_open(&work);
        let _ = std::fs::remove_dir_all(work);
    });
}

/// Per-block RMS of one held-then-released note, starting from a fresh `Render`.
fn note_envelope(chain: &crate::shared_chain::Chain) -> Vec<f64> {
    let mut render = crate::shared_note_render::Render::new(unsafe { chain.audio_ref() });
    let mut velocities = [0; 128];
    velocities[60] = 100;
    let mut durations = [0; 128];
    durations[60] = 24000;
    let mut out = [0.0; 2048];
    (0..48)
        .map(|block| {
            chain.pump();
            assert!(render.process(&velocities, &durations, i32::from(block == 0), &mut out, 2));
            (out.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / out.len() as f64).sqrt()
        })
        .collect()
}

/// Envelope distance relative to `reference`'s energy.
fn distance(envelope: &[f64], reference: &[f64]) -> f64 {
    let diff: f64 = envelope
        .iter()
        .zip(reference)
        .map(|(a, b)| (a - b).powi(2))
        .sum();
    (diff / reference.iter().map(|v| v * v).sum::<f64>()).sqrt()
}

fn reload_sounds_like_fresh_open(work: &std::path::Path) {
    let _library = crate::native_library::load().unwrap();
    let key = crate::shared_chain::default_instrument().unwrap();
    let saw = Settings {
        instrument: Some(state(key, &work.join("source"))),
        ..Default::default()
    };
    let mut ep = saw.clone();
    let slot = ep.instrument.as_mut().unwrap();
    let path = PathBuf::from(std::env::var_os("ProgramData").unwrap())
        .join("Surge XT/patches_factory/Keys/DX EP.fxp");
    slot.state = cmrt_patches::prepare_clap_patch_state(&slot.plugin.id, &path).unwrap();
    assert!(saw.same_plugins(&ep));
    // The catalog spells the default instrument's bundle path with `\` only.
    let mut listed = ep.clone();
    let listed_key = &mut listed.instrument.as_mut().unwrap().plugin;
    listed_key.bundle_path = listed_key.bundle_path.replace('/', "\\");
    assert!(Settings::default().same_plugins(&listed));
    let fresh = |settings: &Settings| {
        note_envelope(&crate::shared_chain::Chain::open(48000, settings).unwrap())
    };
    let reference = fresh(&ep);
    let again = fresh(&ep);
    let other = fresh(&saw);
    // Leave the old sound ringing, as a switch during playback does.
    let dirty = crate::shared_chain::Chain::open(48000, &saw).unwrap();
    {
        let mut render = crate::shared_note_render::Render::new(unsafe { dirty.audio_ref() });
        let mut velocities = [0; 128];
        velocities[64] = 100;
        let mut durations = [0; 128];
        durations[64] = 480000;
        let mut out = [0.0; 2048];
        for block in 0..10 {
            dirty.pump();
            assert!(render.process(&velocities, &durations, i32::from(block == 0), &mut out, 2));
        }
    }
    let reloaded = note_envelope(&dirty.reload(48000, &ep).unwrap());
    let from_default = note_envelope(
        &crate::shared_chain::Chain::open(48000, &Settings::default())
            .unwrap()
            .reload(48000, &listed)
            .unwrap(),
    );
    let (noise, reload, contrast) = (
        distance(&again, &reference),
        distance(&reloaded, &reference),
        distance(&other, &reference),
    );
    let default_reload = distance(&from_default, &reference);
    eprintln!("reload envelope distance: fresh-vs-fresh={noise:.4}, reload-vs-fresh={reload:.4}, default-reload-vs-fresh={default_reload:.4}, other-patch={contrast:.4}");
    eprintln!("reference={reference:.4?}\nreloaded={reloaded:.4?}");
    assert!(
        contrast > 0.3,
        "the measure cannot tell patches apart: {contrast}"
    );
    // Fresh opens can be bit-identical, so the margin over their noise is absolute:
    // 1% of the envelope is about 0.09 dB.
    assert!(
        reload < noise + 0.01,
        "reloaded sound differs: noise={noise}, reload={reload}, contrast={contrast}"
    );
    assert!(
        default_reload < noise + 0.01,
        "sound reloaded over the default instrument differs: noise={noise}, reload={default_reload}"
    );
}
