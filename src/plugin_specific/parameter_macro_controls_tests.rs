use super::*;
use crate::macro_lane::patch_changed;
use crate::sequence_modulation::MacroTarget::Parameter;

fn plugin(id: &str) -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: id.into(),
        name: String::new(),
        vendor: String::new(),
        bundle_path: String::new(),
    }
}

const VAPORIZER2: &str = "com.vastdynamics.VAST2";
const SIX_SINES: &str = "org.baconpaul.six-sines";
const FLOE: &str = "com.floe-audio.floe";

/// Custom modulators 2 and 4 drive matrix rows; modulator 1 has a value but no row.
fn vaporizer2(modulator2: &str, tune: &str) -> Vec<u8> {
    let xml = format!(
        r#"<VASTvaporizer2 PatchVersion="VASTVaporizerParamsV2.20000" CustomModulator2Text="Cutoff"><PARAM id="m_fCustomModulator1" text="30"/><PARAM id="m_fCustomModulator2" text="{modulator2}"/><PARAM id="m_fMasterTune" text="{tune}"/><PARAM id="m_uModMatSrce1" text="CustomModulator2"/><PARAM id="m_uModMatSrce3" text="CustomModulator4"/></VASTvaporizer2>"#
    );
    let mut state = b"VC2!".to_vec();
    state.extend_from_slice(&(xml.len() as u32).to_le_bytes());
    state.extend_from_slice(xml.as_bytes());
    state.push(0);
    state
}

/// Macro 1 moves Op 2's source, Macro 2 moves another macro node, Macro 3's output has
/// no target; `levels` are Macro 1-3 Level, `neighbour` the parameter after Macro 1 Level.
fn six_sines(levels: [f64; 3], neighbour: f64) -> Vec<u8> {
    let params = [
        (1500 + 250 + 152, 400.0),
        (1500 + 250 + 161, 3.0),
        (40000 + 50, 401.0),
        (40000 + 60, 1.0),
        (10000 + 40, 412.0),
        (10000 + 55, 0.0),
        (40000, levels[0]),
        (40001, neighbour),
        (40250, levels[1]),
        (40500, levels[2]),
    ];
    let mut xml = String::from(r#"<patch id="org.baconpaul.six-sines" version="12"><params>"#);
    for (id, v) in params {
        xml += &format!(r#"<p id="{id}" v="{v:.6}" />"#);
    }
    xml += r#"</params><macroNames><macroName idx="0"><![CDATA[Macro 1]]></macroName><macroName idx="1"><![CDATA[Wobble]]></macroName></macroNames></patch>"#;
    let mut state = xml.into_bytes();
    state.push(0);
    state
}

/// Macro 1 is the CC1 one of the factory mapping; Macro 3 has no destination.
fn floe(macros: [f32; 4], volume: f32) -> Vec<u8> {
    let values: Vec<(u32, f32)> = [(0, volume)]
        .into_iter()
        .chain((101..).zip(macros))
        .collect();
    crate::instrument_macros::floe::tests::state(
        &values,
        [
            ("Reverb", 1),
            ("Brightness", 2),
            ("Unused", 0),
            ("Macro 4", 1),
        ],
        &[(1, 101)],
    )
}

fn rows(controls: &Controls) -> Vec<(usize, &str, Option<&str>, Option<u8>)> {
    (0..MACROS)
        .filter(|&i| controls.targets[i] == Some(Parameter))
        .map(|i| {
            (
                i + 1,
                controls.labels[i],
                controls.names[i].as_deref(),
                controls.macros[i],
            )
        })
        .collect()
}

#[test]
fn only_used_macros_get_parameter_rows_named_by_the_patch() {
    let check = |id, state: &[u8], expected: &[(usize, &str, Option<&str>, Option<u8>)]| {
        let controls = crate::plugin_specific::controls(&plugin(id), state);
        assert!(crate::plugin_specific::reports_controls(&plugin(id)));
        assert_eq!(controls.wheel, None);
        assert!(controls
            .targets
            .iter()
            .all(|t| t.is_none() || *t == Some(Parameter)));
        assert_eq!(rows(&controls), expected, "{id}");
    };
    check(
        VAPORIZER2,
        &vaporizer2("100", "440"),
        &[
            (2, "Custom modulator 2", Some("Cutoff"), Some(127)),
            (4, "Custom modulator 4", None, None),
        ],
    );
    // -1-1 levels show on the CC scale; the default "Macro 1" name counts as unnamed.
    check(
        SIX_SINES,
        &six_sines([-1.0, 0.5, 1.0], 0.0),
        &[
            (1, "Macro 1", None, Some(0)),
            (2, "Macro 2", Some("Wobble"), Some(95)),
        ],
    );
    // CC1 already moves Macro 1, so its row is left out.
    check(
        FLOE,
        &floe([0.25, 0.5, 0.75, 1.0], 0.8),
        &[
            (2, "Macro 2", Some("Brightness"), Some(64)),
            (4, "Macro 4", None, Some(127)),
        ],
    );
}

#[test]
fn unreadable_states_and_cc_instruments_get_no_parameter_rows() {
    for id in [VAPORIZER2, SIX_SINES, FLOE] {
        assert_eq!(controls(&plugin(id), b"broken"), Controls::default());
    }
    for id in ["org.surge-synth-team.surge-xt", "com.u-he.TyrellN6"] {
        assert!(crate::plugin_specific::reports_controls(&plugin(id)));
        assert_eq!(controls(&plugin(id), b"broken"), Controls::default());
    }
    assert!(!crate::plugin_specific::reports_controls(&plugin(
        "com.example.other"
    )));
}

/// Moving a macro row's parameter must not read as a patch change: that would release
/// the fixed values back to "-".
#[test]
fn moved_macro_parameters_are_no_patch_change() {
    let cases = [
        (
            VAPORIZER2,
            vaporizer2("100", "440"),
            vaporizer2("12.5", "440"),
            vaporizer2("100", "432"),
        ),
        (
            SIX_SINES,
            six_sines([-1.0, 0.5, 1.0], 0.0),
            six_sines([0.3, -0.7, 0.0], 0.0),
            six_sines([-1.0, 0.5, 1.0], 0.2),
        ),
        (
            FLOE,
            floe([0.25, 0.5, 0.75, 1.0], 0.8),
            floe([0.9, 0.1, 0.0, 0.3], 0.8),
            floe([0.25, 0.5, 0.75, 1.0], 0.7),
        ),
    ];
    for (id, state, moved, edited) in cases {
        let plugin = plugin(id);
        assert!(!patch_changed(&plugin, None, &state), "{id}");
        assert!(!patch_changed(&plugin, Some(&state), &moved), "{id}");
        assert!(patch_changed(&plugin, Some(&state), &edited), "{id}");
        assert_eq!(
            controls(&plugin, &state).targets,
            controls(&plugin, &moved).targets,
            "{id}"
        );
    }
}
