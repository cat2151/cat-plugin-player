use super::*;
use crate::favorites_store::Library;

#[path = "surge_xt_startup_tests.rs"]
mod startup_tests;

fn plugin() -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "org.surge-synth-team.surge-xt".into(),
        name: "Surge XT".into(),
        vendor: String::new(),
        bundle_path: String::new(),
    }
}

#[test]
fn startup_delay_requires_exact_surge_format_and_id() {
    let mut key = plugin();
    key.name = "Renamed".into();
    assert_eq!(automatic_note_start_delay(&key).as_millis(), 100);
    key.format = "VST3".into();
    assert!(automatic_note_start_delay(&key).is_zero());
    key.id = "ABCDEF019182FAEB566D624153675854".into();
    assert_eq!(automatic_note_start_delay(&key).as_millis(), 100);
    key.id = "other".into();
    key.name = "Surge XT".into();
    assert!(automatic_note_start_delay(&key).is_zero());
}

fn wrap(xml: &str, tail: &[u8]) -> Vec<u8> {
    let mut state = b"sub3".to_vec();
    state.extend_from_slice(&(xml.len() as u32).to_le_bytes());
    state.extend_from_slice(&(tail.len() as u32).to_le_bytes());
    state.extend_from_slice(&[0; 20]);
    state.extend_from_slice(xml.as_bytes());
    state.extend_from_slice(tail);
    state
}

const XML: &str = "<patch revision=\"24\"><parameters><volume value=\"0\"/></parameters><modwheel s0=\"0\" s1=\"0\"/><dawExtraState><instanceZoomFactor v=\"-1\"/><tuning v=\"0\"/></dawExtraState></patch>";

#[test]
fn measured_runtime_values_match_but_sound_settings_do_not() {
    let original = wrap(XML, b"wave");
    let played = XML
        .replace("s0=\"0\"", "s0=\"0.33858266\"")
        .replace("s1=\"0\"", "s1=\"1\"")
        .replace("v=\"-1\"", "v=\"125\"");
    assert!(same_sound(&plugin(), &original, &wrap(&played, b"wave")));
    let mut vst = plugin();
    vst.format = "VST3".into();
    vst.id = "ABCDEF019182FAEB566D624153675854".into();
    assert!(same_sound(&vst, &original, &wrap(&played, b"wave")));
    for xml in [
        XML.replace("value=\"0\"", "value=\"1\""),
        XML.replace("tuning v=\"0\"", "tuning v=\"1\""),
        XML.replace("s0=\"0\"", "s0=\"NaN\""),
        XML.replace("s0=\"0\"", "s0=\"2\""),
        XML.replace("revision=\"24\"", "revision=\"25\""),
        XML.replace("<modwheel", "<other"),
        XML.replace("</patch>", ""),
        XML.replace("<modwheel", "<modwheel extra=\"0\""),
    ] {
        assert!(!same_sound(&plugin(), &original, &wrap(&xml, b"wave")));
    }
    assert!(!same_sound(&plugin(), &original, &wrap(&played, b"edit")));
    let mut broken = wrap(&played, b"wave");
    broken[4] ^= 1;
    assert!(!same_sound(&plugin(), &original, &broken));
    broken = wrap(&played, b"wave");
    broken.push(0);
    assert!(!same_sound(&plugin(), &original, &broken));
    let mut other = plugin();
    other.id = "other".into();
    assert!(!same_sound(&other, &original, &wrap(&played, b"wave")));
}

#[test]
fn vst_private_trailer_is_preserved_and_clap_rejects_it() {
    let mut vst = plugin();
    vst.format = "VST3".into();
    vst.id = "ABCDEF019182FAEB566D624153675854".into();
    let mut a = wrap(XML, &[]);
    let mut b = wrap(&XML.replace("v=\"-1\"", "v=\"125\""), &[]);
    let trailer = b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0JUCEPrivateData";
    a.extend_from_slice(trailer);
    b.extend_from_slice(trailer);
    assert!(same_sound(&vst, &a, &b));
    assert!(!same_sound(&plugin(), &a, &b));
    b[32 + XML.replace("v=\"-1\"", "v=\"125\"").len()] = 1;
    assert!(!same_sound(&vst, &a, &b));
}

#[test]
fn runtime_exclusions_are_scoped_to_exact_paths() {
    let a = XML.replace("</parameters>", "<modwheel s0=\"0\"/></parameters>");
    let b = a.replacen("s0=\"0\"", "s0=\"1\"", 1);
    assert!(!same_sound(&plugin(), &wrap(&a, &[]), &wrap(&b, &[])));
    let a = XML.replace(
        "</parameters>",
        "<instanceZoomFactor v=\"125\"/></parameters>",
    );
    let b = a.replace("v=\"125\"", "v=\"150\"");
    assert!(!same_sound(&plugin(), &wrap(&a, &[]), &wrap(&b, &[])));
}

#[test]
#[ignore = "requires installed Surge XT CLAP and existing favorite, read only"]
fn native_surge_xt_repeated_favorites_do_not_grow() {
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
    let _dll = crate::native_library::load().unwrap();
    let user_config = crate::config::path().unwrap();
    // Read metadata directly: never run a product library migration in a probe.
    let user_library: Library = toml::from_str(
        &std::fs::read_to_string(user_config.parent().unwrap().join("favorites/index.toml"))
            .unwrap(),
    )
    .unwrap();
    let favorite = user_library
        .entries
        .iter()
        .find(|f| f.name == "Surge XT 2" && f.plugin.id == plugin().id && f.plugin.format == "CLAP")
        .unwrap();
    let original = user_library.state(&user_config, &favorite.id).unwrap();
    let dir = std::env::temp_dir().join(format!(
        "cat-surge_xt-dedup-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = dir.join("config.toml");
    let host = ffi::Host::new(wake, std::ptr::null_mut()).unwrap();
    let info = host.restore_plugin(&favorite.plugin).unwrap();
    let mut result: Option<Result<i32, String>> = None;
    host.create_instance(
        info.index,
        48000,
        256,
        created,
        &mut result as *mut _ as *mut c_void,
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while result.is_none() {
        assert!(std::time::Instant::now() < deadline);
        host.pump_startup();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let id = result.unwrap().unwrap();
    host.load_state(id, &original).unwrap();
    // Confirm both reported saved pairs using read-only snapshots.
    for (a, b) in [
        ("auto Surge XT 3", "auto Surge XT 4"),
        ("Surge XT 2", "auto Surge XT 5"),
    ] {
        let a = user_library.entries.iter().find(|f| f.name == a).unwrap();
        let b = user_library.entries.iter().find(|f| f.name == b).unwrap();
        assert_eq!(a.sequence_pattern, b.sequence_pattern);
        let mut pair_library = Library::default();
        let pair_config = dir.join(&a.plugin.format).join("config.toml");
        let first_pair = pair_library
            .add_automatic(
                &pair_config,
                a.plugin.clone(),
                false,
                &user_library.state(&user_config, &a.id).unwrap(),
                a.sequence_pattern,
            )
            .unwrap();
        let second_pair = pair_library
            .add_automatic(
                &pair_config,
                b.plugin.clone(),
                false,
                &user_library.state(&user_config, &b.id).unwrap(),
                b.sequence_pattern,
            )
            .unwrap();
        assert_eq!(first_pair.id, second_pair.id);
        assert_eq!(pair_library.entries.len(), 1);
        assert!(same_sound(
            &a.plugin,
            &user_library.state(&user_config, &a.id).unwrap(),
            &user_library.state(&user_config, &b.id).unwrap()
        ));
    }
    let mut library = Library::default();
    let first = library
        .add(
            &config,
            favorite.plugin.clone(),
            false,
            &original,
            favorite.sequence_pattern,
        )
        .unwrap();
    library.rename(&config, &first.id, "Kept name").unwrap();
    for step in 0..6 {
        if step == 2 || step == 4 {
            let processor = host.create_chain(id, &[], false, 48000, 256).unwrap();
            let mut render = unsafe { processor.audio_ref() };
            let mut output = [0.0; 512];
            assert!(render.process(&[0x2090_3c64, 0x20b0_0164], &mut output, 2));
            assert!(render.process(&[0x2080_3c00, 0x20b0_0100], &mut output, 2));
            drop(processor);
        }
        let saved = host.save_state(id).unwrap();
        let reused = if step % 2 == 0 {
            library.add(
                &config,
                favorite.plugin.clone(),
                false,
                &saved,
                favorite.sequence_pattern,
            )
        } else {
            library.add_automatic(
                &config,
                favorite.plugin.clone(),
                false,
                &saved,
                favorite.sequence_pattern,
            )
        }
        .unwrap();
        assert_eq!(reused.id, first.id);
        assert_eq!(reused.name, "Kept name");
        assert_eq!(library.entries.len(), 1);
        host.load_state(id, &saved).unwrap();
    }
    assert_eq!(library.state(&config, &first.id).unwrap(), original);
    // A real sound-setting edit must still create a separate immutable snapshot.
    let (xml, tail) = parts(&original, false).unwrap();
    let xml = std::str::from_utf8(xml).unwrap();
    let from = "<a_osc1_pitch type=\"2\" value=\"0.00000000000000\"";
    assert!(xml.contains(from));
    let changed = wrap(
        &xml.replace(from, "<a_osc1_pitch type=\"2\" value=\"-6.00000000000000\""),
        tail,
    );
    host.load_state(id, &changed).unwrap();
    // Surge applies queued state on its audio thread after processing has started.
    let processor = host.create_chain(id, &[], false, 48000, 256).unwrap();
    let mut render = unsafe { processor.audio_ref() };
    let mut output = [0.0; 512];
    assert!(render.process(&[], &mut output, 2));
    drop(processor);
    let changed = host.save_state(id).unwrap();
    library
        .add(
            &config,
            favorite.plugin.clone(),
            false,
            &changed,
            favorite.sequence_pattern,
        )
        .unwrap();
    assert_eq!(library.entries.len(), 2);
    assert_eq!(Library::load(&config).unwrap().entries.len(), 2);
    drop(host);
    std::fs::remove_dir_all(dir).unwrap();
}
