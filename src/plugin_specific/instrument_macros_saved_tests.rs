//! Macro usage read from the user's saved favorites.
use crate::{favorites_store::Library, instrument_macros::Instrument};

/// Each used macro as `(number, name, ccs)`.
type Used = Vec<(usize, Option<String>, Vec<u8>)>;

/// Each saved favorite of `id` with its display name.
fn saved(id: &str) -> Vec<(String, Used)> {
    let config = crate::config::path().unwrap();
    let library: Library = toml::from_str(
        &std::fs::read_to_string(config.parent().unwrap().join("favorites/index.toml")).unwrap(),
    )
    .unwrap();
    library
        .entries
        .iter()
        .filter(|e| e.plugin.id == id)
        .map(|e| {
            let instrument = Instrument::of(&e.plugin.format, id).unwrap();
            let state = library.state(&config, &e.id).unwrap();
            let macros = instrument
                .routed(&e.plugin.format, &state)
                .unwrap_or_else(|| panic!("{} state is not readable", e.name));
            let macros = macros
                .into_iter()
                .map(|m| (m.number, m.name, m.ccs))
                .collect();
            println!("{}: {macros:?}", e.name);
            (e.name.clone(), macros)
        })
        .collect()
}

#[test]
#[ignore = "requires saved Six Sines CLAP favorites; reads user files only"]
fn saved_six_sines_favorites_use_macros_1_and_2() {
    let saved = saved("org.baconpaul.six-sines");
    let expected = vec![(1, None, vec![]), (2, None, vec![])];
    assert!(saved.iter().any(|(_, macros)| *macros == expected));
    // The others are patches without macro routing.
    for (name, macros) in saved {
        assert!(macros.is_empty() || macros == expected, "{name}");
    }
}

#[test]
#[ignore = "requires saved Floe CLAP favorites; reads user files only"]
fn saved_floe_favorites_use_cc1_macro_1_and_brightness() {
    let saved = saved("com.floe-audio.floe");
    assert!(!saved.is_empty());
    for (name, macros) in saved {
        assert_eq!(
            macros,
            [
                (1, Some("Reverb".into()), vec![1]),
                (2, Some("Brightness".into()), vec![])
            ],
            "{name}"
        );
    }
}

/// The macro rows shown for a state as `(number, name)`.
type Rows = Vec<(usize, Option<String>)>;

/// Each saved favorite of `id` with the macro rows shown for it.
fn rows(id: &str) -> Vec<(String, Rows)> {
    let config = crate::config::path().unwrap();
    let library: Library = toml::from_str(
        &std::fs::read_to_string(config.parent().unwrap().join("favorites/index.toml")).unwrap(),
    )
    .unwrap();
    library
        .entries
        .iter()
        .filter(|e| e.plugin.id == id)
        .map(|e| {
            let state = library.state(&config, &e.id).unwrap();
            let controls = crate::plugin_specific::controls(&e.plugin, &state);
            let rows = (0..crate::sequence_modulation::MACROS)
                .filter(|&i| {
                    controls.targets[i] == Some(crate::sequence_modulation::MacroTarget::Parameter)
                })
                .map(|i| (i + 1, controls.names[i].clone()))
                .collect();
            println!("{}: {rows:?} {:?}", e.name, controls.macros);
            (e.name.clone(), rows)
        })
        .collect()
}

#[test]
#[ignore = "requires saved Six Sines CLAP favorites; reads user files only"]
fn saved_six_sines_favorites_show_macro_rows_1_and_2() {
    let saved = rows("org.baconpaul.six-sines");
    let expected = vec![(1, None), (2, None)];
    assert!(saved.iter().any(|(_, rows)| *rows == expected));
    for (name, rows) in saved {
        assert!(rows.is_empty() || rows == expected, "{name}");
    }
}

#[test]
#[ignore = "requires saved Floe CLAP favorites; reads user files only"]
fn saved_floe_favorites_show_only_brightness() {
    let saved = rows("com.floe-audio.floe");
    assert!(!saved.is_empty());
    for (name, rows) in saved {
        assert_eq!(rows, [(2, Some("Brightness".into()))], "{name}");
    }
}

#[test]
#[ignore = "requires saved Vaporizer2 CLAP favorites; reads user files only"]
fn saved_vaporizer2_favorites_show_their_routed_modulators() {
    let id = "com.vastdynamics.VAST2";
    let routed = saved(id);
    let shown = rows(id);
    assert!(routed.iter().any(|(_, macros)| !macros.is_empty()));
    for ((name, macros), (_, rows)) in routed.into_iter().zip(shown) {
        let expected: Vec<_> = macros.into_iter().map(|(n, name, _)| (n, name)).collect();
        assert_eq!(rows, expected, "{name}");
    }
}
