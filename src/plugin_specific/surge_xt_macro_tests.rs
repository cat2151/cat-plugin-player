use super::*;

fn plugin() -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "org.surge-synth-team.surge-xt".into(),
        name: "Surge XT".into(),
        vendor: String::new(),
        bundle_path: String::new(),
    }
}

fn wrap(xml: &str) -> Vec<u8> {
    let mut state = b"sub3".to_vec();
    state.extend_from_slice(&(xml.len() as u32).to_le_bytes());
    state.extend_from_slice(&[0; 24]);
    state.extend_from_slice(xml.as_bytes());
    state
}

fn patch(routing: &str, cc: &str, channel: &str) -> Vec<u8> {
    wrap(&format!(
        "<patch revision=\"24\"><parameters><a_osc1_pitch value=\"0\">{routing}</a_osc1_pitch>\
         </parameters><dawExtraState populated=\"1\"><customcontrol_map><c p=\"1\" v=\"42\"/>{cc}\
         </customcontrol_map><customcontrol_chan_map>{channel}</customcontrol_chan_map>\
         </dawExtraState></patch>"
    ))
}

const MACRO1: &str = "<modrouting source=\"7\" depth=\"0.3\"/>";
const CC41: &str = "<c p=\"0\" v=\"41\"/>";
const ANY: &str = "<ch p=\"0\" v=\"-1\"/>";

fn macro1_cc(plugin: &PluginKey, state: &[u8]) -> Option<u8> {
    macro_ccs(plugin, state)[0]
}

#[test]
fn each_routed_macro_reports_its_own_controller() {
    let routing = "<modrouting source=\"8\" depth=\"0.3\"/><modrouting source=\"14\" depth=\"1\"/>\
                   <modrouting source=\"15\" depth=\"1\"/>";
    let maps = "<c p=\"7\" v=\"48\"/><c p=\"8\" v=\"49\"/>";
    let channels = "<ch p=\"1\" v=\"-1\"/><ch p=\"7\" v=\"0\"/>";
    let mut expected = [None; MACROS];
    expected[1] = Some(42);
    expected[7] = Some(48);
    assert_eq!(
        macro_ccs(&plugin(), &patch(routing, maps, channels)),
        expected
    );
}

#[test]
fn routed_macro_one_reports_its_saved_controller() {
    assert_eq!(macro1_cc(&plugin(), &patch(MACRO1, CC41, ANY)), Some(41));
    let learned = patch(MACRO1, "<c p=\"0\" v=\"20\"/>", "<ch p=\"0\" v=\"0\"/>");
    assert_eq!(macro1_cc(&plugin(), &learned), Some(20));
}

#[test]
fn unrouted_unmapped_other_channels_and_other_plugins_are_hidden() {
    let modwheel = "<modrouting source=\"6\" depth=\"0.6\"/>";
    assert_eq!(macro1_cc(&plugin(), &patch(modwheel, CC41, ANY)), None);
    assert_eq!(macro1_cc(&plugin(), &patch(MACRO1, "", ANY)), None);
    assert_eq!(macro1_cc(&plugin(), &patch(MACRO1, CC41, "")), None);
    let channel_two = "<ch p=\"0\" v=\"1\"/>";
    assert_eq!(
        macro1_cc(&plugin(), &patch(MACRO1, CC41, channel_two)),
        None
    );
    let off = "<c p=\"0\" v=\"-1\"/>";
    assert_eq!(macro1_cc(&plugin(), &patch(MACRO1, off, ANY)), None);
    let other = PluginKey {
        id: "org.surge-synth-team.surge-xt-fx".into(),
        ..plugin()
    };
    assert_eq!(macro1_cc(&other, &patch(MACRO1, CC41, ANY)), None);
    assert_eq!(macro1_cc(&plugin(), b"sub3"), None);
}

#[test]
fn current_wheel_and_macro_positions_read_on_the_cc_scale() {
    let state = wrap(
        "<patch revision=\"24\"><customcontroller><entry i=\"0\" bipolar=\"0\" v=\"1\"/>         <entry i=\"1\" bipolar=\"1\" v=\"-1\"/><entry i=\"2\" bipolar=\"1\" v=\"0\"/>         <entry i=\"3\" bipolar=\"0\" v=\"0.5\"/><entry i=\"8\" bipolar=\"0\" v=\"1\"/>         </customcontroller><modwheel s0=\"0.25\" s1=\"1\"/></patch>",
    );
    let controls = controls(&plugin(), &state);
    assert_eq!(controls.wheel, Some(32));
    assert_eq!(
        controls.macros,
        [
            Some(127),
            Some(0),
            Some(64),
            Some(64),
            None,
            None,
            None,
            None
        ]
    );
    assert_eq!(controls.ccs, [None; MACROS]);
}

#[test]
fn named_macros_report_their_names_and_unnamed_ones_none() {
    let state = wrap(
        "<patch revision=\"24\"><customcontroller>         <entry i=\"0\" bipolar=\"0\" v=\"0\" label=\"LFO Speed\"/>         <entry i=\"1\" bipolar=\"0\" v=\"0\" label=\"-\"/>         <entry i=\"2\" bipolar=\"0\" v=\"0\" label=\"Who&apos;s There?\"/>         <entry i=\"3\" bipolar=\"0\" v=\"0\" label=\"Reverb Decay&#x10;\"/>         <entry i=\"4\" bipolar=\"0\" v=\"0\" label=\"\"/>         <entry i=\"5\" bipolar=\"0\" v=\"0\"/></customcontroller></patch>",
    );
    let names = controls(&plugin(), &state).names;
    assert_eq!(names[0].as_deref(), Some("LFO Speed"));
    assert_eq!(names[1], None);
    assert_eq!(names[2].as_deref(), Some("Who's There?"));
    assert_eq!(names[3].as_deref(), Some("Reverb Decay"));
    assert_eq!(names[4..], [None, None, None, None]);
}

/// Organ 1 routes CC1 and Macro 1; each macro CC must reach its macro through the host's MIDI path.
#[test]
#[ignore = "requires installed Surge XT CLAP and its factory Organ 1 patch"]
fn native_organ_one_macros_follow_their_ccs() {
    use crate::ffi;
    use std::ffi::{c_char, c_void, CStr};
    unsafe extern "C" fn wake(_: *mut c_void) {}
    unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
        let result = &mut *(user as *mut Option<Result<i32, String>>);
        *result = Some(if error.is_null() && id >= 0 {
            Ok(id)
        } else if error.is_null() {
            Err("creation failed".into())
        } else {
            Err(CStr::from_ptr(error).to_string_lossy().into_owned())
        });
    }
    // Surge's .fxp: a 60-byte VST2 program header, then the same chunk as the plugin state.
    let fxp = std::fs::read("C:/ProgramData/Surge XT/patches_factory/Keys/Organ 1.fxp").unwrap();
    let organ = &fxp[60..];
    let _dll = crate::native_library::load().unwrap();
    // Read only: the scanned bundle path of the installed Surge XT CLAP.
    let cache = crate::config::path()
        .unwrap()
        .with_file_name("plugin-list-cache.json");
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(cache).unwrap()).unwrap();
    let key: PluginKey = entries
        .into_iter()
        .filter_map(|entry| serde_json::from_value(entry["plugin"].clone()).ok())
        .find(|key: &PluginKey| key.format == "CLAP" && key.id == plugin().id)
        .unwrap();
    let host = unsafe { ffi::Host::new(wake, std::ptr::null_mut()) }.unwrap();
    let info = host.restore_plugin(&key).unwrap();
    let mut result: Option<Result<i32, String>> = None;
    unsafe {
        host.create_instance(
            info.index,
            48000,
            256,
            created,
            &mut result as *mut _ as *mut c_void,
        )
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while result.is_none() {
        assert!(std::time::Instant::now() < deadline);
        host.pump_startup();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let id = result.unwrap().unwrap();
    host.load_state(id, organ).unwrap();
    let processor = host.create_chain(id, &[], false, 48000, 256).unwrap();
    let mut render = unsafe { processor.audio_ref() };
    let mut output = [0.0; 512];
    let mut macro_values = |events: &[u32]| {
        // Surge applies queued state and MIDI on its audio thread.
        assert!(render.process(events, &mut output, 2));
        assert!(render.process(&[], &mut output, 2));
        let saved = host.save_state(id).unwrap();
        let (xml, _) = parts(&saved, false).unwrap();
        let xml = std::str::from_utf8(xml).unwrap();
        let values: [f64; MACROS] = std::array::from_fn(|i| {
            let entry = &xml[xml.find(&format!("<entry i=\"{i}\"")).unwrap()..];
            let v = &entry[entry.find(" v=\"").unwrap() + 4..];
            v[..v.find('"').unwrap()].parse().unwrap()
        });
        (macro_ccs(&plugin(), &saved), values, saved)
    };
    let (ccs, initial, saved) = macro_values(&[]);
    eprintln!("Organ 1 macro CCs {ccs:?}, values {initial:?}");
    assert_eq!((ccs[0], initial[0]), (Some(41), 0.0));
    let (_, full, _) = macro_values(&[crate::sequence_modulation::cc(41, 127)]);
    assert_eq!(full[0], 1.0);
    let (_, half, _) = macro_values(&[crate::sequence_modulation::cc(41, 64)]);
    assert!((half[0] - 64.0 / 127.0).abs() < 0.01, "{half:?}");
    // CC1 moves the modulation wheel, not Macro 1.
    let (_, after_cc1, _) = macro_values(&[crate::sequence_modulation::cc1(127)]);
    assert_eq!(after_cc1[0], half[0]);
    // Macro 2-8 follow Surge's default CC42-48 the same way.
    let others: Vec<u32> = (1..MACROS as u8)
        .map(|i| crate::sequence_modulation::cc(41 + i, 127))
        .collect();
    let (_, moved, played) = macro_values(&others);
    eprintln!("after CC42-48 = 127: {moved:?}");
    for i in 1..MACROS {
        assert_ne!(moved[i], initial[i], "Macro {}", i + 1);
    }
    // Only macro positions differ: the same favorite sound.
    assert!(crate::plugin_specific::same_favorite_state(
        &plugin(),
        &saved,
        &played
    ));
    // The displayed "-" values read back what the CCs set.
    let (_, _, half_wheel) = macro_values(&[
        crate::sequence_modulation::cc1(64),
        crate::sequence_modulation::cc(41, 64),
    ]);
    let read = controls(&plugin(), &half_wheel);
    assert_eq!((read.wheel, read.macros[0]), (Some(64), Some(64)));
    assert_eq!(read.macros[1], Some(127));
    // A bipolar macro maps CC 0-127 onto -1-1.
    let (xml, _) = parts(&half_wheel, false).unwrap();
    let xml = std::str::from_utf8(xml).unwrap();
    let entry = xml.find("<entry i=\"0\"").unwrap();
    let flag = entry + xml[entry..].find("bipolar=\"0\"").unwrap();
    let bipolar = wrap(&format!(
        "{}bipolar=\"1\"{}",
        &xml[..flag],
        &xml[flag + 11..]
    ));
    host.load_state(id, &bipolar).unwrap();
    let (_, loaded, _) = macro_values(&[]);
    eprintln!("bipolar Macro 1 after loading: {}", loaded[0]);
    for (cc, expected) in [(0, -1.0), (127, 1.0), (32, 32.0 / 127.0 * 2.0 - 1.0)] {
        let (_, values, state) = macro_values(&[crate::sequence_modulation::cc(41, cc)]);
        eprintln!("bipolar Macro 1 after CC41 = {cc}: {}", values[0]);
        assert!((values[0] - expected).abs() < 0.01, "{values:?}");
        assert_eq!(controls(&plugin(), &state).macros[0], Some(cc));
    }
    // A patch change, and not a macro CC, makes Surge rescan its parameter list
    // within some blocks: the trigger to re-read the state.
    let mut blocks = |events: &[u32]| {
        for _ in 0..30 {
            assert!(render.process(events, &mut output, 2));
            host.pump();
        }
        host.parameter_list_changes(id)
    };
    let settled = blocks(&[]);
    assert_eq!(blocks(&[crate::sequence_modulation::cc(41, 100)]), settled);
    host.load_state(id, organ).unwrap();
    assert!(blocks(&[]) > settled);
    drop(processor);
}
