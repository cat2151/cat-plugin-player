//! Read-only native probe: compare first-note output with several settling times.
use crate::{config, favorites_store::Library, ffi, seq, sequence_pattern::SequencePattern};
use std::{
    ffi::{c_char, c_void, CStr},
    time::{Duration, Instant},
};

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    let result = &mut *(user as *mut Option<Result<i32, String>>);
    *result = Some(if !error.is_null() {
        Err(CStr::from_ptr(error).to_string_lossy().into_owned())
    } else if id < 0 {
        Err("creation failed".into())
    } else {
        Ok(id)
    });
}

#[test]
#[ignore = "requires installed Surge XT CLAP/VST3 and saved favorites; reads user data only"]
fn native_first_note_settling_measurement() {
    let _dll = crate::native_library::load().unwrap();
    let path = config::path().unwrap();
    let library: Library = toml::from_str(
        &std::fs::read_to_string(path.parent().unwrap().join("favorites/index.toml")).unwrap(),
    )
    .unwrap();
    for (format, plugin_id) in [
        ("CLAP", "org.surge-synth-team.surge-xt"),
        ("VST3", "ABCDEF019182FAEB566D624153675854"),
    ] {
        let favorite = library
            .entries
            .iter()
            .find(|f| f.plugin.format == format && f.plugin.id == plugin_id && !f.effect)
            .expect("saved Surge instrument favorite required");
        let state = library.state(&path, &favorite.id).unwrap();
        let host = ffi::Host::new(wake, std::ptr::null_mut()).unwrap();
        let info = host.restore_plugin(&favorite.plugin).unwrap();
        for delay_ms in [0, 25, 50, 100] {
            let mut sounded = 0;
            let mut onset_ms = Vec::new();
            for _ in 0..3 {
                let mut result: Option<Result<i32, String>> = None;
                host.create_instance(
                    info.index,
                    48_000,
                    256,
                    created,
                    &mut result as *mut _ as *mut c_void,
                );
                let deadline = Instant::now() + Duration::from_secs(30);
                while result.is_none() {
                    assert!(Instant::now() < deadline, "creation timed out");
                    host.pump_startup();
                    std::thread::sleep(Duration::from_millis(1));
                }
                let id = result.unwrap().unwrap();
                host.load_state(id, &state).unwrap();
                let processor = host.create_chain(id, &[], false, 48_000, 256).unwrap();
                let mut render = unsafe { processor.audio_ref() };
                let mut sequencer = seq::Sequencer::new(48_000);
                sequencer.delay_start(Duration::from_millis(delay_ms));
                let mut events = seq::EventBuf::new();
                let mut output = [0.0; 512];
                let start = Instant::now();
                let mut first_signal = None;
                // Stay within the first held note of the guitar phrase.
                let total = delay_ms as usize * 48 + 7_680;
                for at in (0..total).step_by(256) {
                    events.clear();
                    sequencer.render(
                        SequencePattern::GuitarArpeggio,
                        crate::SequenceVelocity::V100,
                        256,
                        &mut events,
                    );
                    assert!(render.process(events.as_slice(), &mut output, 2));
                    if first_signal.is_none() {
                        first_signal = output
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .position(|frame| frame.iter().any(|s| s.abs() > 0.000001))
                            .map(|offset| (at + offset) as f64 / 48.0);
                    }
                    host.pump_startup();
                    let due = start + Duration::from_secs_f64((at + 256) as f64 / 48_000.0);
                    if let Some(wait) = due.checked_duration_since(Instant::now()) {
                        std::thread::sleep(wait);
                    }
                }
                if let Some(ms) = first_signal {
                    sounded += 1;
                    onset_ms.push(ms);
                }
                drop(processor);
                host.destroy_instance(id);
            }
            eprintln!("{format} favorite={} delay={delay_ms}ms first-note signal={sounded}/3 onset_ms={onset_ms:?}", favorite.name);
            if delay_ms == 100 {
                assert_eq!(sounded, 3, "100ms failed on {format}");
            }
        }
    }
}
