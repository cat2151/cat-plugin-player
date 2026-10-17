use crate::{chain_settings, master_effects, selection::Selection, smf};

#[test]
fn master_selection_saves_only_master_and_keeps_preview_track_chain() {
    let root = crate::selection_tests::directory("master save");
    let path = root.join("input.mid");
    crate::selection_tests::input(&path, &chain_settings::Settings::default());
    let mut original = smf::io::load(&path).unwrap();
    original.format = 1;
    let effect = chain_settings::PluginState {
        reference: None,
        plugin: crate::config::PluginKey {
            format: "CLAP".into(),
            id: "fixture.effect".into(),
            name: "Effect".into(),
            vendor: "fixture".into(),
            bundle_path: "X:/plugins/effect.clap".into(),
        },
        state: vec![1],
        preset_name: Some("Room".into()),
        bypass: false,
    };
    let master = master_effects::Settings {
        effects: vec![effect.clone()],
        ..Default::default()
    };
    master_effects::write(&mut original, &master).unwrap();
    std::fs::write(&path, smf::io::encode(&original).unwrap()).unwrap();
    let mut selection = Selection::load_mode(path.clone(), 48000, true).unwrap();
    assert_eq!(
        selection.preview_chain,
        Some(chain_settings::Settings::default())
    );
    assert_eq!(selection.settings.effects, master.effects);
    let mut changed = selection.settings.clone();
    changed.effects[0].bypass = true;
    selection.save(&changed).unwrap();
    let returned = smf::io::load(&path).unwrap();
    assert_eq!(
        chain_settings::read(&returned.tracks[0]).unwrap(),
        chain_settings::Settings::default()
    );
    assert!(master_effects::read(&returned).unwrap().effects[0].bypass);
    changed.effects.clear();
    selection.save(&changed).unwrap();
    let cleared = smf::io::load(&path).unwrap();
    assert!(!cleared.tracks[0].events.iter().any(master_effects::owned));
    assert_eq!(
        cleared.tracks[0]
            .events
            .iter()
            .filter(|e| e.status != 0xff)
            .map(|e| (&e.data, e.tick))
            .collect::<Vec<_>>(),
        original.tracks[0]
            .events
            .iter()
            .filter(|e| e.status != 0xff)
            .map(|e| (&e.data, e.tick))
            .collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "requires built native shim and CLAP state fixture; does not emit audio"]
fn native_master_effect_processes_input_bypass_and_empty_chain_pass_through() {
    let _library = crate::native_library::load().unwrap();
    let effect = chain_settings::PluginState {
        reference: None,
        plugin: crate::config::PluginKey {
            format: "CLAP".into(),
            id: "cat.test.state.effect".into(),
            name: "State effect".into(),
            vendor: "cat test".into(),
            bundle_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/shim/Release/clap_state_test_plugin.clap")
                .to_string_lossy()
                .into_owned(),
        },
        state: vec![1],
        preset_name: None,
        bypass: false,
    };
    let bypassed = chain_settings::PluginState {
        bypass: true,
        ..effect.clone()
    };
    for (effects, expected) in [(vec![], 0.25), (vec![bypassed], 0.25), (vec![effect], 0.0)] {
        let settings = master_effects::Settings {
            effects,
            ..Default::default()
        };
        let chain = crate::chain::Chain::open_master(48000, &settings).unwrap();
        let mut render = unsafe { chain.audio_ref() };
        for channels in [1, 2, 4] {
            let mut input = vec![0.25; 32 * channels];
            assert!(render.process_input(&mut input, channels));
            for (i, v) in input.iter().enumerate() {
                let processed = channels == 1 || i % channels < 2;
                assert_eq!(
                    *v,
                    if processed { expected } else { 0.25 },
                    "{channels}ch #{i}"
                );
            }
        }
        let mut oversized = vec![0.25; 2050];
        assert!(!render.process_input(&mut oversized, 2));
        assert!(oversized.iter().all(|v| *v == 0.0));
    }
}
