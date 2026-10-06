use super::*;
use crate::{favorites_store::Library, sequence_pattern::SequencePattern};

fn plugin() -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "com.vastdynamics.VAST2".into(),
        name: "Vaporizer2".into(),
        vendor: "VAST Dynamics".into(),
        bundle_path: "X:/plugins/VASTvaporizer2.clap".into(),
    }
}

fn wrap(xml: &str) -> Vec<u8> {
    let mut state = b"VC2!".to_vec();
    state.extend_from_slice(&(xml.len() as u32).to_le_bytes());
    state.extend_from_slice(xml.as_bytes());
    state.push(0);
    state
}

fn fixture(time: &str, x: &str, tune: &str) -> Vec<u8> {
    wrap(&format!("<VASTvaporizer2 PatchVersion=\"VASTVaporizerParamsV2.20000\"><PARAM id=\"m_fMasterTune\" text=\"{tune}\"/><chunkData><msegData0 m_fAttackTimeExternalSet=\"{time}\"><msegPoint1 xVal=\"{x}\" yVal=\"1.0\"/></msegData0></chunkData></VASTvaporizer2>"))
}

#[test]
fn round_trip_drift_matches_but_sound_edits_survive() {
    let base = fixture("10.0", "0.01639344228660604", "440");
    assert!(same_sound(
        &plugin(),
        &base,
        &fixture("10.0000000001", "0.01639344228659529", "440")
    ));
    for changed in [
        fixture("10.00001", "0.01639344228660604", "440"),
        fixture("10.0", "0.0164", "440"),
        fixture("10.0", "0.01639344228660604", "432"),
        fixture("NaN", "0.01639344228660604", "440"),
        fixture("inf", "0.01639344228660604", "440"),
    ] {
        assert!(!same_sound(&plugin(), &base, &changed));
    }
    let text = std::str::from_utf8(xml(&base).unwrap()).unwrap();
    assert!(!same_sound(
        &plugin(),
        &base,
        &wrap(&text.replace("yVal=\"1.0\"", "yVal=\"1.0000000001\""))
    ));
    for (from, to) in [
        ("V2.20000", "V2.30000"),
        ("chunkData", "otherData"),
        ("msegData0", "msegData5"),
    ] {
        let a = wrap(&text.replace(from, to));
        let b = wrap(&text.replace(from, to).replace("10.0\"", "10.0000000001\""));
        assert!(!same_sound(&plugin(), &a, &b), "{from}");
    }
}

#[test]
fn unknown_identity_and_malformed_layout_do_not_match() {
    let a = fixture("10.0", "0.1", "440");
    let b = fixture("10.0000000001", "0.1", "440");
    for key in [
        PluginKey {
            format: "VST3".into(),
            ..plugin()
        },
        PluginKey {
            id: "other".into(),
            ..plugin()
        },
    ] {
        assert!(!same_sound(&key, &a, &b));
    }
    for i in 0..2 {
        let (mut a, mut b) = (a.clone(), b.clone());
        if i == 0 {
            a[4] ^= 1;
            b[4] ^= 1;
        } else {
            a.pop();
            b.pop();
        }
        assert!(!same_sound(&plugin(), &a, &b));
    }
    assert!(!same_sound(
        &plugin(),
        &wrap("<VASTvaporizer2>"),
        &wrap("<VASTvaporizer2>")
    ));
}

#[test]
#[ignore = "requires installed Vaporizer2 CLAP and saved favorites (read only)"]
fn native_vaporizer_round_trip_deduplicates_and_tuning_edits_survive() {
    use crate::ffi;
    use std::ffi::{c_char, c_void};
    unsafe extern "C" fn wake(_: *mut c_void) {}
    unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
        *(user as *mut Option<Result<i32, String>>) =
            Some(ffi::error_string(error).map_or(Ok(id), Err));
    }
    let _dll = crate::native_library::load().unwrap();
    let config = crate::config::path().unwrap();
    let saved = Library::load(&config).unwrap();
    let entries: Vec<_> = saved
        .entries
        .iter()
        .filter(|e| e.plugin.format == "CLAP" && e.plugin.id == plugin().id)
        .collect();
    assert!(!entries.is_empty());
    let original = saved.state(&config, &entries[0].id).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "cat-vaporizer-favorites-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let temporary_config = directory.join("config.toml");
    let mut library = Library::default();
    for entry in entries {
        library
            .add_automatic(
                &temporary_config,
                entry.plugin.clone(),
                entry.effect,
                &saved.state(&config, &entry.id).unwrap(),
                entry.sequence_pattern,
            )
            .unwrap();
    }
    assert_eq!(
        library.entries.len(),
        1,
        "existing drift-only snapshots should merge"
    );
    let host = ffi::Host::new(wake, std::ptr::null_mut()).unwrap();
    let info = host
        .restore_plugin(
            &saved
                .entries
                .iter()
                .find(|e| e.plugin.id == plugin().id)
                .unwrap()
                .plugin,
        )
        .unwrap();
    let mut result: Option<Result<i32, String>> = None;
    host.create_instance(
        info.index,
        48000,
        1024,
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
    let round_trip = host.save_state(id).unwrap();
    assert_ne!(original, round_trip, "expected measured load/save drift");
    assert!(same_sound(&plugin(), &original, &round_trip));
    let key = saved
        .entries
        .iter()
        .find(|e| e.plugin.id == plugin().id)
        .unwrap()
        .plugin
        .clone();
    let retained = library
        .state(&temporary_config, &library.entries[0].id)
        .unwrap();
    let favorite = library
        .add_automatic(
            &temporary_config,
            key.clone(),
            false,
            &round_trip,
            SequencePattern::Steps,
        )
        .unwrap();
    assert_eq!(library.entries.len(), 1);
    assert_eq!(
        library.state(&temporary_config, &favorite.id).unwrap(),
        retained
    );
    let processor = host.create_chain(id, &[], false, 48000, 1024).unwrap();
    let mut audio = unsafe { processor.audio_ref() };
    let mut output = [0.0; 2048];
    for events in [
        &[0x2090_3064, 0x20B0_017F][..],
        &[0x2080_3000, 0x20B0_0100][..],
    ] {
        assert!(audio.process(events, &mut output, 2));
    }
    drop(processor);
    let played = host.save_state(id).unwrap();
    assert!(same_sound(&plugin(), &round_trip, &played));
    let text = std::str::from_utf8(xml(&played).unwrap()).unwrap();
    let before = "id=\"m_fMasterTune\" text=\"440\"";
    assert!(text.contains(before));
    host.load_state(
        id,
        &wrap(&text.replacen(before, "id=\"m_fMasterTune\" text=\"432\"", 1)),
    )
    .unwrap();
    let edited = host.save_state(id).unwrap();
    assert!(!same_sound(&plugin(), &played, &edited));
    library
        .add_automatic(
            &temporary_config,
            key,
            false,
            &edited,
            SequencePattern::Steps,
        )
        .unwrap();
    assert_eq!(library.entries.len(), 2);
    assert_eq!(Library::load(&temporary_config).unwrap().entries.len(), 2);
    host.destroy_instance(id);
    drop(host);
    std::fs::remove_dir_all(directory).unwrap();
}
