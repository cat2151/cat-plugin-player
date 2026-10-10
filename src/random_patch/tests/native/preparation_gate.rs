//! Opt-in cross-host state transfer gate; does not modify user configuration.
use super::*;

#[test]
#[ignore = "requires installed catalog plugins and RANDOM_GATE_WORK_DIR / RANDOM_GATE_PLUGIN"]
fn native_preparation_cross_host_gate() {
    let _library = crate::native_library::load().unwrap();
    let requested = std::env::var("RANDOM_GATE_PLUGIN").unwrap();
    let root = PathBuf::from(std::env::var_os("RANDOM_GATE_WORK_DIR").unwrap());
    let (snapshot, _) = clap_mml_render_tui::patch_catalog_cache::load()
        .unwrap()
        .into_parts();
    let refs: Vec<_> = snapshot
        .audio_patches()
        .iter()
        .filter(|patch| {
            snapshot
                .patch_plugins()
                .audio_info_for_ref(&patch.reference)
                .unwrap()
                .plugin_id
                .as_deref()
                == Some(&requested)
        })
        .filter(|patch| {
            let selected: &[&str] = match requested.as_str() {
                "com.Plogue Art et Technologie, Inc.sforzando" => &[
                    "TableWarp2/Programs/TableWarp2.sfz",
                    "TableWarp2/Presets/com.Plogue.Aria/Keys/Airy Bells.ariax",
                ],
                "org.surge-synth-team.surge-xt" => &[
                    "patches_factory/Basses/Bass 1.fxp",
                    "patches_factory/Basses/Bass 2.fxp",
                ],
                "org.baconpaul.six-sines" => &["Bass/Bass 1.sxsnp", "Bass/Bass 2.sxsnp"],
                "com.vastdynamics.VAST2" => &["BA 5th Bass 2.vvp", "BA 5th Bass 3.vvp"],
                _ => &[],
            };
            selected.is_empty() || selected.contains(&patch.reference.display.as_str())
        })
        .take(2)
        .collect();
    assert_eq!(refs.len(), 2);
    let plugin = snapshot
        .patch_plugins()
        .audio_info_for_ref(&refs[0].reference)
        .unwrap();
    let key = PluginKey {
        format: "CLAP".into(),
        id: requested.clone(),
        name: plugin.name.clone(),
        vendor: String::new(),
        bundle_path: plugin.plugin_path.clone(),
    };
    let mut current = app(&root.join("current/config.toml"));
    let instrument = add(&mut current, &key);
    current.load_plugin(instrument);
    wait(&mut current);
    let mut states = Vec::new();
    let mut audios = Vec::new();
    for (index, patch) in refs.iter().enumerate() {
        let plugin = snapshot
            .patch_plugins()
            .audio_info_for_ref(&patch.reference)
            .unwrap();
        let resolved = PathBuf::from(plugin.patch_base.resolve(&patch.reference.display));
        let bundle = Path::new(&plugin.plugin_path);
        let started = Instant::now();
        // This is the production worker boundary: only owned paths/ID enter;
        // creation through destruction finishes before UAPMD creates anything.
        let id = requested.clone();
        let bundle_owned = bundle.to_path_buf();
        let resolved_owned = resolved.clone();
        let state = std::thread::spawn(move || {
            cmrt_patches::prepare_catalog_clap_patch_state(&id, &bundle_owned, &resolved_owned)
        })
        .join()
        .unwrap()
        .unwrap();
        eprintln!(
            "prepared {} {} bytes in {:?}: {}",
            requested,
            state.len(),
            started.elapsed(),
            resolved.display()
        );
        assert!(!state.is_empty());
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(format!("prepared-{index}.state")), &state).unwrap();
        let candidate = crate::random_patch_catalog::Candidate {
            format: "CLAP".into(),
            plugin_id: requested.clone(),
            plugin_name: plugin.name.clone(),
            display: patch.reference.display.clone(),
            path: resolved.clone(),
            bundle_path: bundle.to_path_buf(),
            entry: cmrt_patch_select::PatchCatalogEntry::from_display(
                patch.reference.display.clone(),
            ),
            measurement: Default::default(),
        };
        let pattern = if index == 0 {
            SequencePattern::Off
        } else {
            SequencePattern::Steps
        };
        current.sequence_pattern = pattern;
        current.apply_random_patch(Prepared {
            candidate,
            state: state.clone(),
        });
        wait(&mut current);
        assert!(
            current.status.contains("Random patch:"),
            "{}",
            current.status
        );
        assert_eq!(current.sequence_pattern, pattern);
        assert_eq!(
            state_store::load(current.config_path.as_ref().unwrap(), &key).unwrap(),
            Some(state.clone())
        );
        let live = current.instrument_id().unwrap();
        let selected_audio = super::real_surge::render(&current, live);
        assert!(selected_audio.iter().any(|x| x.abs() > 1e-5));
        if matches!(
            requested.as_str(),
            "org.baconpaul.six-sines" | "com.u-he.TyrellN6"
        ) {
            // Check the FIRST note, before the next Steps note can hide a lost
            // note-on. Ignore only the documented startup withholding interval.
            let delay = crate::plugin_specific::automatic_note_start_delay_at_rate(&key, 48_000);
            let start = (delay.as_secs_f64() * 48_000.0).ceil() as usize * 2;
            assert!(
                selected_audio[start..start + 7_200 * 2]
                    .iter()
                    .any(|x| x.abs() > 1e-5),
                "first note was lost or stayed silent: {}",
                patch.reference.display
            );
        }
        let mut restored = app(&root.join(format!("restored-{index}/config.toml")));
        let instrument = add(&mut restored, &key);
        restored.load_plugin(instrument);
        wait(&mut restored);
        let live = restored.instrument_id().unwrap();
        restored.host.load_state(live, &state).unwrap();
        let audio = super::real_surge::render(&restored, live);
        let rms =
            (audio.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / audio.len() as f64).sqrt();
        eprintln!(
            "restored {} {} RMS={rms}",
            requested, patch.reference.display
        );
        assert!(
            rms > 1e-6,
            "restored patch is silent: {}",
            patch.reference.display
        );
        states.push(state);
        audios.push(audio);
    }
    assert_ne!(
        states[0], states[1],
        "two patch references produced the same state"
    );
    let difference = (audios[0]
        .iter()
        .zip(&audios[1])
        .map(|(a, b)| f64::from(a - b).powi(2))
        .sum::<f64>()
        / audios[0].len() as f64)
        .sqrt();
    eprintln!("{} restored patch difference RMS={difference}", requested);
    assert!(
        difference > 1e-5,
        "two restored patches are indistinguishable"
    );
}
