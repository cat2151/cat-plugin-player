//! Native check that opening the editor of a minimal-data-home instance rebuilds it.
use crate::{native_library::process::ROOT, session_load::LoadPurpose, App, SequencePattern};
use std::{
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

const ENV: &str = "SURGE_DATA_HOME";
const CHILD: &str = "CAT_SURGE_EDITOR_REBUILD_CHILD";
const NAME: &str = "native_editor_rebuilds_minimal_surge_with_normal_data_home";

pub(super) fn settle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.pending.is_some() {
        assert!(Instant::now() < deadline, "load timed out: {}", app.status);
        app.host.pump();
        app.handle_events();
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub(super) fn peak(app: &App, id: i32) -> f32 {
    let processor = app.host.create_chain(id, &[], false, 48_000, 256).unwrap();
    let mut audio = unsafe { processor.audio_ref() };
    let mut sequencer = crate::seq::Sequencer::new(48_000);
    let mut events = crate::seq::EventBuf::new();
    let mut peak = 0.0f32;
    for _ in 0..100 {
        events.clear();
        sequencer.render(
            SequencePattern::Steps,
            crate::SequenceVelocity::V100,
            256,
            &mut events,
        );
        let mut output = [0.0; 512];
        assert!(audio.process(events.as_slice(), &mut output, 2));
        peak = output.iter().fold(peak, |p, s| p.max(s.abs()));
        app.host.pump_startup();
        std::thread::sleep(Duration::from_secs_f64(256.0 / 48_000.0));
    }
    peak
}

/// One App per process: native host startup state is process-wide.
fn child(case: &str) {
    let root =
        std::env::temp_dir().join(format!("cat-surge-editor-rebuild-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let test = module_path!().split_once("::").unwrap().1;
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            &format!("{test}::{NAME}"),
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(CHILD, case)
        .env(ROOT, &root)
        .env_remove("CMRT_SURGE_MINIMAL_DATA_HOME")
        .env_remove(ENV);
    if case.ends_with("set") && !case.ends_with("unset") {
        let custom = root.join("custom-home");
        let source = PathBuf::from(std::env::var_os("ProgramData").unwrap()).join("Surge XT");
        assert!(Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&custom)
            .arg(source)
            .output()
            .unwrap()
            .status
            .success());
        command.env(ENV, custom);
    }
    let out = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stderr.lines().filter(|l| l.starts_with("rebuild:")) {
        eprintln!("{line}");
    }
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        out.status.success() && stdout.contains("1 passed"),
        "child {case} failed\n{stdout}\n{stderr}"
    );
}

#[test]
#[ignore = "requires installed Surge XT CLAP/VST3 and factory patches; briefly opens editors, memory-only audio"]
fn native_editor_rebuilds_minimal_surge_with_normal_data_home() {
    let Ok(case) = std::env::var(CHILD) else {
        for case in ["clap-unset", "clap-set", "vst3-unset", "vst3-set", "manual"] {
            child(case);
        }
        return;
    };
    let _dll = crate::native_library::load().unwrap();
    let before_env = std::env::var_os(ENV);
    let mut app = App::prepare();
    app.output = None;
    let mut key = crate::selection_host::default_instrument().unwrap();
    if case.starts_with("vst3") {
        key.format = "VST3".into();
        key.id = "ABCDEF019182FAEB566D624153675854".into();
        key.bundle_path = PathBuf::from(std::env::var_os("CommonProgramFiles").unwrap())
            .join("VST3/Surge Synth Team/Surge XT.vst3")
            .to_string_lossy()
            .into_owned();
    }
    // As after `load_plugin_cache`: no deferred scan keeps the actions busy.
    let plugin = app.host.restore_plugin(&key).unwrap();
    app.plugins.push(plugin);
    app.deferred_scan = false;
    app.restore = Some(key.clone());
    app.restore_before_gui();
    settle(&mut app);
    assert_eq!(app.instances.len(), 1, "{}", app.status);
    let old = app.instances[0].id;
    assert!(app.instances[0].minimal_data_home, "{}", app.status);

    // A non-default sound, so that an unrestored new instance would differ.
    let initial = app.host.save_state(old).unwrap();
    let patch = PathBuf::from(std::env::var_os("ProgramData").unwrap())
        .join("Surge XT/patches_factory/Basses/Bass 2.fxp");
    let state =
        cmrt_patches::prepare_clap_patch_state("org.surge-synth-team.surge-xt", &patch).unwrap();
    app.host.load_state(old, &state).unwrap();
    // Opening Surge's editor adds editor-only fields (zoom, MSEG view) to its state.
    app.host.show_ui(old).unwrap();
    app.host.hide_ui(old);
    let edited = edited_state(&app.host.save_state(old).unwrap(), key.format == "VST3");
    app.host.load_state(old, &edited).unwrap();
    let before = app.host.save_state(old).unwrap();
    assert!(!crate::plugin_specific::same_favorite_state(
        &key, &initial, &before
    ));

    app.favorites.active.push((old, "favorite".into()));
    app.sequence_pattern = SequencePattern::Off;
    app.selected_sequence = SequencePattern::GuitarArpeggio;
    app.sequence_velocity = crate::SequenceVelocity::Range40To100;
    app.sequence_modulation = crate::SequenceModulation::Sweep;
    app.instances[0].sweep_cc1 = true;
    app.instances[0].preset_name = Some("edited Bass 2".into());
    let catalog = super::catalog_probe::CatalogProbe::start();
    let started = Instant::now();
    if case == "manual" {
        app.load_plugin(0);
    } else {
        app.open_editor(old);
    }
    assert!(
        matches!(
            app.pending.as_ref().map(|p| &p.purpose),
            Some(LoadPurpose::Editor(_)) | Some(LoadPurpose::Manual)
        ),
        "{}",
        app.status
    );
    assert!(app.pending.as_ref().unwrap().data_home.is_none());
    assert_eq!(std::env::var_os(ENV), before_env);
    // Pressing again while rebuilding keeps the one pending creation.
    app.open_editor(old);
    assert!(matches!(
        app.pending.as_ref().map(|p| &p.purpose),
        Some(LoadPurpose::Editor(_)) | Some(LoadPurpose::Manual)
    ));
    settle(&mut app);
    let elapsed = started.elapsed();
    catalog.assert_normal();
    if case.ends_with("set") && !case.ends_with("unset") {
        assert!(catalog
            .paths()
            .iter()
            .any(|p| p.contains("/custom-home/patches_factory/Basses/")));
    }

    assert_eq!(app.instances.len(), 1, "{}", app.status);
    let new = app.instances[0].id;
    assert_ne!(new, old, "{}", app.status);
    assert!(!app.instances[0].minimal_data_home);
    assert_eq!(std::env::var_os(ENV), before_env);
    assert_eq!(app.favorites.active, [(new, "favorite".to_string())]);
    assert_eq!(
        app.host.ui_is_visible(new),
        case != "manual",
        "{}",
        app.status
    );
    if case == "manual" {
        app.host.show_ui(new).unwrap();
    }
    app.host.hide_ui(new);
    let after = app.host.save_state(new).unwrap();
    assert!(crate::plugin_specific::same_favorite_state(
        &key, &before, &after
    ));
    assert_eq!(app.sequence_pattern, SequencePattern::Off);
    assert_eq!(app.selected_sequence, SequencePattern::GuitarArpeggio);
    assert_eq!(app.sequence_velocity, crate::SequenceVelocity::Range40To100);
    assert_eq!(app.sequence_modulation, crate::SequenceModulation::Sweep);
    assert!(app.instances[0].sweep_cc1);
    assert_eq!(
        app.instances[0].preset_name.as_deref(),
        Some("edited Bass 2")
    );
    assert!(app.editor_rebuild.is_none());
    app.open_editor(new);
    assert!(app.pending.is_none(), "already normal must not rebuild");
    app.host.hide_ui(new);
    let peak = peak(&app, new);
    assert!(peak > 1e-3, "silent chain: {peak}");
    eprintln!(
        "rebuild: {case} {:.1} ms, peak {peak:.3}",
        elapsed.as_secs_f64() * 1000.0
    );
}

pub(super) fn edited_state(state: &[u8], vst: bool) -> Vec<u8> {
    let (xml, tail) = super::super::parts(state, vst).unwrap();
    let xml = std::str::from_utf8(xml).unwrap();
    let from = "<a_osc1_pitch type=\"2\" value=\"0.00000000000000\"";
    assert!(xml.contains(from));
    let edited = xml.replace(from, "<a_osc1_pitch type=\"2\" value=\"-6.00000000000000\"");
    let mut out = state[..4].to_vec();
    out.extend_from_slice(&(edited.len() as u32).to_le_bytes());
    out.extend_from_slice(&state[8..32]);
    out.extend_from_slice(edited.as_bytes());
    out.extend_from_slice(tail);
    out
}
