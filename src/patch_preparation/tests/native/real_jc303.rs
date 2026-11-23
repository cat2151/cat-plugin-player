//! Opt-in installed JC303 acceptance (ADR 0044); all application writes use the supplied directory.
use super::*;
use std::ffi::{c_char, c_void};

unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    unsafe {
        *(user as *mut Option<Result<i32, String>>) =
            Some(crate::ffi::error_string(error).map_or(Ok(id), Err));
    }
}

fn sync_mode(state: &[u8]) -> String {
    let text = String::from_utf8_lossy(state);
    let at = text.find(r#"id="seqSyncMode""#).unwrap();
    let start = text[..at].rfind('<').unwrap();
    let end = at + text[at..].find('>').unwrap();
    let value = &text[start..end];
    let value = &value[value.find(r#"value=""#).unwrap() + 7..];
    value[..value.find('"').unwrap()].to_owned()
}

fn with_mode(state: &[u8], from: &str, to: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(state).into_owned();
    let replaced = text
        .replace(
            &format!(r#"<PARAM id="seqSyncMode" value="{from}"/>"#),
            &format!(r#"<PARAM id="seqSyncMode" value="{to}"/>"#),
        )
        .replace(
            &format!(r#"<PARAM value="{from}" id="seqSyncMode"/>"#),
            &format!(r#"<PARAM value="{to}" id="seqSyncMode"/>"#),
        );
    assert_ne!(replaced, text);
    replaced.into_bytes()
}

#[test]
#[ignore = "requires installed JC303 CLAP and S07_NATIVE_WORK_DIR"]
fn native_real_jc303_plays_incoming_notes_after_host_sync() {
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S07_NATIVE_WORK_DIR").unwrap());
    let jc303 = PluginKey {
        format: "CLAP".into(),
        id: "co.midilab.JC303".into(),
        name: "JC303".into(),
        vendor: "midilab".into(),
        bundle_path: std::env::var("S07_JC303_BUNDLE")
            .unwrap_or_else(|_| r"C:\Program Files\Common Files\CLAP\JC303.clap".into()),
    };
    let path = root.join("jc303").join("config.toml");
    let mut app = app(&path);
    let index = add(&mut app, &jc303);

    // A fresh instance starts in Host sync and plays JC303's own pattern, not the notes.
    let mut result: Box<Option<Result<i32, String>>> = Box::new(None);
    unsafe {
        app.host.create_instance(
            app.plugins[index].index,
            48_000,
            256,
            created,
            &mut *result as *mut _ as *mut c_void,
        )
    };
    while result.is_none() {
        app.host.pump_startup();
        std::thread::sleep(Duration::from_millis(1));
    }
    let raw = result.take().unwrap().unwrap();
    assert_eq!(sync_mode(&app.host.save_state(raw).unwrap()), "1.0");
    let unswitched = render(&app.host, raw);
    app.host.destroy_instance(raw);

    // Each comparison uses a new instance: a render ends with a held note, which
    // would turn the next render's first notes into slides.
    // A new load without saved state reaches the chain with Internal sync.
    app.load_plugin(index);
    wait(&mut app);
    let first = app.instrument_id().unwrap();
    let internal = app.host.save_state(first).unwrap();
    assert_eq!(sync_mode(&internal), "0.0");
    app.poll_macro_lane();
    assert_eq!(
        app.macro_lane.controls.labels,
        [
            "Cutoff",
            "Resonance",
            "Filt. EnvMod",
            "Filt. Decay",
            "Accent",
            "Waveform",
            "OD Drive",
            "OD Dry/Wet"
        ]
    );
    assert!(app.macro_lane.controls.macros.iter().all(Option::is_some));
    let expected = render(&app.host, first);
    assert!(expected.iter().any(|s| *s != 0.0));
    assert_ne!(unswitched, expected);

    // A saved Host state restored on the next load plays the notes the same way.
    let host = with_mode(&internal, "0.0", "1.0");
    app.host.load_state(first, &host).unwrap();
    app.load_plugin(index);
    wait(&mut app);
    let second = app.instrument_id().unwrap();
    assert_ne!(second, first);
    assert_eq!(app.host.save_state(second).unwrap(), internal);
    assert_eq!(render(&app.host, second), expected);

    // A Host state loaded into the running instance (favorite, patch, history)
    // is switched by the same chain preparation.
    app.host.load_state(second, &host).unwrap();
    assert!(app
        .prepare_voice(second, &[], false, None)
        .unwrap()
        .is_none());
    assert_eq!(sync_mode(&app.host.save_state(second).unwrap()), "0.0");

    // Midi Clock waits for a MIDI Start the app never sends, so it stays.
    app.host
        .load_state(second, &with_mode(&internal, "0.0", "2.0"))
        .unwrap();
    assert!(app
        .prepare_voice(second, &[], false, None)
        .unwrap()
        .is_none());
    assert_eq!(sync_mode(&app.host.save_state(second).unwrap()), "2.0");
}

/// About 0.5 s of the Steps pattern from the first sample, paced like the audio device.
fn render(host: &crate::ffi::Host, id: i32) -> Vec<f32> {
    let processor = host.create_chain(id, &[], false, 48_000, 256).unwrap();
    let mut audio = unsafe { processor.audio_ref() };
    let mut sequencer = crate::seq::Sequencer::new(48_000);
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
        host.pump_startup();
        let due = start + Duration::from_secs_f64((block + 1) as f64 * 256.0 / 48_000.0);
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
    }
    samples
}
