use super::*;
use crate::favorites_store::Library;
use flate2::{write::ZlibEncoder, Compression};
use std::io::Write;

fn plugin() -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "com.Plogue Art et Technologie, Inc.sforzando".into(),
        name: "sforzando".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/sforzando.clap".into(),
    }
}

fn wrap(xml: &str) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml.as_bytes()).unwrap();
    let mut state = b"CEGP".to_vec();
    state.extend_from_slice(&(xml.len() as u32).to_le_bytes());
    state.extend(encoder.finish().unwrap());
    state
}

fn fixture(counter: u32, runtime: &str, tuning: u32) -> Vec<u8> {
    wrap(&format!("<AriaSave version=\"1982\" productID=\"1014\"><Settings sc=\"{counter}\" quality=\"1\"/><Slot id=\"0\" sc=\"{counter}\" name=\"TableWarp2\" tuning=\"{tuning}\"><Main id=\"0\" value=\"1\"/>{runtime}</Slot><EffectSlot sc=\"{counter}\" name=\"Ambience\"/><GUI selectedTab=\"3\"/></AriaSave>"))
}

#[test]
fn saves_and_playback_match_but_sound_changes_survive() {
    let original = fixture(17, "", 0);
    let played = fixture(
        22,
        "<Param id=\"1\" value=\"0.5\"/><Param id=\"131\" value=\"0.8\"/>",
        0,
    );
    assert!(same_sound(&plugin(), &original, &played));
    for changed in [
        fixture(22, "", 1),
        fixture(22, "<Param id=\"7\" value=\"0.5\"/>", 0),
        fixture(22, "<Param id=\"1\" value=\"NaN\"/>", 0),
        fixture(22, "<Param id=\"131\" value=\"2\"/>", 0),
        fixture(22, "<Other sc=\"22\"/>", 0),
    ] {
        assert!(!same_sound(&plugin(), &original, &changed));
    }
    let xml = String::from_utf8({
        let mut xml = Vec::new();
        ZlibDecoder::new(&original[8..])
            .read_to_end(&mut xml)
            .unwrap();
        xml
    })
    .unwrap();
    for (from, to) in [
        ("TableWarp2", "OtherSound"),
        ("value=\"1\"", "value=\"0.8\""),
        ("quality=\"1\"", "quality=\"2\""),
        ("version=\"1982\"", "version=\"1983\""),
    ] {
        assert!(!same_sound(
            &plugin(),
            &original,
            &wrap(&xml.replace(from, to))
        ));
    }
}

#[test]
fn unknown_or_broken_states_do_not_match() {
    let a = fixture(17, "", 0);
    let b = fixture(18, "", 0);
    assert!(!same_sound(
        &PluginKey {
            format: "VST3".into(),
            ..plugin()
        },
        &a,
        &b
    ));
    assert!(!same_sound(
        &PluginKey {
            id: "other".into(),
            ..plugin()
        },
        &a,
        &b
    ));
    let mut bad_size = a.clone();
    bad_size[4] = bad_size[4].wrapping_add(1);
    let mut bad_header = a.clone();
    bad_header[0] = b'X';
    for broken in [a[..a.len() - 1].to_vec(), bad_size, bad_header] {
        assert!(!same_sound(&plugin(), &broken, &b));
    }
    let mut trailing = a.clone();
    trailing.push(0);
    assert!(!same_sound(&plugin(), &trailing, &b));
    assert!(settings(&wrap(
        "<AriaSave version=\"1982\" productID=\"1014\"><Slot></AriaSave>"
    ))
    .is_none());
}

#[test]
#[ignore = "requires installed sforzando CLAP and existing favorite, read only"]
fn native_sforzando_repeated_favorites_do_not_grow() {
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
        .find(|f| f.plugin.id == plugin().id && f.plugin.format == "CLAP")
        .unwrap();
    let original = user_library.state(&user_config, &favorite.id).unwrap();
    let dir = std::env::temp_dir().join(format!(
        "cat-sforzando-dedup-{}-{}",
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
    let mut xml = Vec::new();
    ZlibDecoder::new(&original[8..])
        .read_to_end(&mut xml)
        .unwrap();
    let xml = String::from_utf8(xml).unwrap();
    assert!(xml.contains("tuning=\"0\""));
    let changed = wrap(&xml.replace("tuning=\"0\"", "tuning=\"1\""));
    host.load_state(id, &changed).unwrap();
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
