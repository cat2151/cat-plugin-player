//! Event positions inside a block reach the plugin at the sample the UMP words name.
use super::*;
use std::ffi::{c_char, c_void};
use std::time::Duration;

const RATE: u32 = 48_000;

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    let result = &mut *(user as *mut Option<Result<i32, String>>);
    *result = Some(match ffi::error_string(error) {
        Some(error) => Err(error),
        None if id < 0 => Err("creation failed".into()),
        None => Ok(id),
    });
}

/// Peaks of each rendered block after `first` frames carrying `events`, then 480-frame
/// blocks whose events `later` gives by block start frame.
fn render(
    first: (usize, &[u32]),
    mut later: impl FnMut(usize, &mut EventBuf),
    seconds: usize,
) -> Vec<(usize, f32)> {
    let candidate = crate::patch_catalog::load()
        .unwrap()
        .into_iter()
        .find(|c| c.plugin_id == "com.u-he.Zebralette3" && c.path.ends_with("Square Brass.h2p"))
        .expect("Zebralette3 factory patch Square Brass");
    let state = crate::state_request::catalog(
        &candidate.plugin_id,
        &candidate.bundle_path,
        &candidate.path,
    )
    .unwrap();
    let host = unsafe { ffi::Host::new(wake, std::ptr::null_mut()) }.unwrap();
    let key = crate::config::PluginKey {
        format: "CLAP".into(),
        id: candidate.plugin_id.clone(),
        bundle_path: candidate.bundle_path.to_string_lossy().into(),
        name: String::new(),
        vendor: String::new(),
    };
    let plugin = host.restore_plugin(&key).unwrap();
    let mut result: Box<Option<Result<i32, String>>> = Box::new(None);
    unsafe {
        host.create_instance(
            plugin.index,
            RATE,
            MAX_BLOCK_FRAMES,
            created,
            &mut *result as *mut _ as *mut c_void,
        )
    };
    while result.is_none() {
        host.pump_startup();
        std::thread::sleep(Duration::from_millis(1));
    }
    let id = result.take().unwrap().unwrap();
    host.load_state(id, &state).unwrap();
    let processor = host
        .create_chain(id, &[], false, RATE, MAX_BLOCK_FRAMES)
        .unwrap();
    let mut audio = unsafe { processor.audio_ref() };
    let mut out = vec![0.0f32; MAX_BLOCK_FRAMES as usize * 2];
    let mut events = EventBuf::new();
    let mut peaks = Vec::new();
    let mut at = 0;
    while at < RATE as usize * seconds {
        let frames = if at == 0 { first.0 } else { 480 };
        events.clear();
        if at == 0 {
            for word in first.1 {
                events.push(*word);
            }
        } else {
            later(at, &mut events);
        }
        let block = &mut out[..frames * 2];
        block.fill(0.0);
        assert!(audio.process(events.as_slice(), block, 2));
        peaks.push((at, block.iter().fold(0.0f32, |m, s| m.max(s.abs()))));
        at += frames;
    }
    drop(processor);
    host.destroy_instance(id);
    peaks
}

fn peak(peaks: &[(usize, f32)], from_ms: usize, to_ms: usize) -> f32 {
    peaks
        .iter()
        .filter(|(at, _)| (from_ms * 48..to_ms * 48).contains(at))
        .fold(0.0, |m, (_, p)| m.max(*p))
}

#[test]
#[ignore = "installed Zebralette3 CLAP and its factory patches"]
fn native_zebralette3_plays_and_releases_events_late_in_a_block() {
    let _library = crate::native_library::load().unwrap();
    // 700 samples lies inside the block, but past it when read as 1/31250 s ticks.
    let peaks = render(
        (
            1024,
            &[
                crate::seq::jr_timestamp(700),
                crate::seq::note_on(0, 60, 100),
            ],
        ),
        |at, events| {
            // The block that starts in [500 ms, 510 ms).
            if (24_000..24_480).contains(&at) {
                events.push(note_off(0, 60));
            }
        },
        2,
    );
    assert!(peak(&peaks, 0, 500) > 0.05, "the note never sounded");
    assert!(peak(&peaks, 1500, 2000) < 0.005, "the note kept sounding");

    // The startup case: the phrase starts in the device's first 1024-frame block.
    let mut sequencer = Sequencer::new(RATE);
    sequencer.set_phrase(Some(std::sync::Arc::new(
        crate::timed_sequence::Phrase::parse("crrrrdrr", RATE).unwrap(),
    )));
    sequencer.set_modulation(Some(SequenceModulation::Sweep));
    let mut first = EventBuf::new();
    sequencer.render(
        SequencePattern::Custom,
        SequenceVelocity::Range80To127,
        1024,
        &mut first,
    );
    let peaks = render(
        (1024, first.as_slice()),
        |_, events| {
            sequencer.render(
                SequencePattern::Custom,
                SequenceVelocity::Range80To127,
                480,
                events,
            )
        },
        2,
    );
    // The first note is released at 250 ms and the next starts at 1250 ms.
    assert!(
        peak(&peaks, 900, 1200) < 0.005,
        "the first note kept sounding"
    );
    assert!(
        peak(&peaks, 1300, 1600) > 0.05,
        "the next note never sounded"
    );
}
