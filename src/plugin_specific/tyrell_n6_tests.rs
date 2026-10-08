//! Opt-in regression against installed TyrellN6 and a read-only favorite.
use super::*;
use crate::{favorites_store::Library, ffi, sequence_pattern::SequencePattern};
use std::ffi::{c_char, c_void, CStr};

unsafe extern "C" fn wake(_: *mut c_void) {}

#[test]
#[ignore = "requires saved TyrellN6 UI_op 9/10 favorites (read only)"]
fn saved_tyrell_ui_transition_reuses_immutable_favorite() {
    let user_config = crate::config::path().unwrap();
    let user_library = Library::load(&user_config).unwrap();
    let states: Vec<_> = user_library
        .entries
        .iter()
        .filter(|f| f.plugin.format == "CLAP" && f.plugin.id == "com.u-he.TyrellN6")
        .map(|f| (f, user_library.state(&user_config, &f.id).unwrap()))
        .collect();
    let pair = states
        .iter()
        .enumerate()
        .find_map(|(i, (a, left))| {
            states.iter().skip(i + 1).find_map(|(b, right)| {
                let l = tyrell_settings(left)?;
                let r = tyrell_settings(right)?;
                (a.sequence_pattern == b.sequence_pattern
                    && l != r
                    && ui_operation(l.trim_end_matches('\n'))
                        == ui_operation(r.trim_end_matches('\n'))
                    && ui_operation(l.trim_end_matches('\n')).is_some())
                .then_some((a, left, right))
            })
        })
        .expect("save a matching UI_op 9/10 pair first");
    let (favorite, left, right) = pair;
    let directory = std::env::temp_dir().join(format!(
        "cat-tyrell-ui-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = directory.join("config.toml");
    let mut library = Library::default();
    let first = library
        .add_automatic(
            &config,
            favorite.plugin.clone(),
            false,
            left,
            favorite.sequence_pattern,
        )
        .unwrap();
    let second = library
        .add_automatic(
            &config,
            favorite.plugin.clone(),
            false,
            right,
            favorite.sequence_pattern,
        )
        .unwrap();
    assert_eq!(library.entries.len(), 1);
    assert_eq!(first.id, second.id);
    assert_eq!(first.name, second.name);
    assert_eq!(library.state(&config, &second.id).unwrap(), *left);
    let pattern = if favorite.sequence_pattern == SequencePattern::Steps {
        SequencePattern::Off
    } else {
        SequencePattern::Steps
    };
    library
        .add_automatic(&config, favorite.plugin.clone(), false, right, pattern)
        .unwrap();
    assert_eq!(library.entries.len(), 2);
    std::fs::remove_dir_all(directory).unwrap();
}

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

#[test]
#[ignore = "requires TyrellN6 CLAP build 16976 and an existing favorite (read only)"]
fn native_tyrell_playback_deduplicates_but_cutoff_edits_survive() {
    let _dll = crate::native_library::load().unwrap();
    let user_config = crate::config::path().unwrap();
    let user_library = Library::load(&user_config).unwrap();
    let favorite = user_library
        .entries
        .iter()
        .find(|f| f.plugin.format == "CLAP" && f.plugin.id == "com.u-he.TyrellN6")
        .expect("save a TyrellN6 CLAP favorite first");
    let original = user_library.state(&user_config, &favorite.id).unwrap();
    assert!(
        tyrell_settings(&original).is_some(),
        "unsupported state layout"
    );
    let directory = std::env::temp_dir().join(format!(
        "cat-tyrell-dedup-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = directory.join("config.toml");
    let host = ffi::Host::new(wake, std::ptr::null_mut()).unwrap();
    let plugin = host.restore_plugin(&favorite.plugin).unwrap();
    let mut result: Option<Result<i32, String>> = None;
    host.create_instance(
        plugin.index,
        48000,
        1024,
        created,
        &mut result as *mut _ as *mut c_void,
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while result.is_none() {
        assert!(std::time::Instant::now() < deadline, "creation timed out");
        host.pump_startup();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let id = result.unwrap().unwrap();
    host.load_state(id, &original).unwrap();
    let mut library = Library::default();
    library
        .add(
            &config,
            favorite.plugin.clone(),
            false,
            &original,
            SequencePattern::Steps,
        )
        .unwrap();

    // Playback changes the opaque state, but must not grow the favorite list.
    for events in [
        vec![0x2090_3064, 0x20B0_0100],
        vec![0x2080_3000, 0x20B0_017F],
    ] {
        let processor = host.create_chain(id, &[], false, 48000, 1024).unwrap();
        let mut render = unsafe { processor.audio_ref() };
        let mut output = [0.0; 2048];
        assert!(render.process(&events, &mut output, 2));
        for _ in 0..20 {
            assert!(render.process(&[], &mut output, 2));
        }
        drop(processor); // Stop processing before requesting state.
        let played = host.save_state(id).unwrap();
        assert_ne!(played, original, "playback no longer changes opaque state");
        let saved = library
            .add_automatic(
                &config,
                favorite.plugin.clone(),
                false,
                &played,
                SequencePattern::Steps,
            )
            .unwrap();
        assert_eq!(library.entries.len(), 1);
        assert_eq!(library.state(&config, &saved.id).unwrap(), original);
        assert_eq!(Library::load(&config).unwrap().entries.len(), 1);
    }

    // Change an actual sound parameter through the native state loader.
    let text = std::str::from_utf8(&original).unwrap();
    let cutoff = text
        .lines()
        .find(|line| line.starts_with("Cutoff="))
        .unwrap();
    let new_cutoff = if cutoff == "Cutoff=0.00" {
        "Cutoff=85.00"
    } else {
        "Cutoff=0.00"
    };
    let edited = text.replacen(cutoff, new_cutoff, 1);
    host.load_state(id, edited.as_bytes()).unwrap();
    let edited = host.save_state(id).unwrap();
    assert!(!same_sound(&favorite.plugin, &original, &edited));
    library
        .add_automatic(
            &config,
            favorite.plugin.clone(),
            false,
            &edited,
            SequencePattern::Steps,
        )
        .unwrap();
    assert_eq!(library.entries.len(), 2);
    assert_eq!(Library::load(&config).unwrap().entries.len(), 2);
    host.destroy_instance(id);
    drop(host);
    std::fs::remove_dir_all(directory).unwrap();
}
