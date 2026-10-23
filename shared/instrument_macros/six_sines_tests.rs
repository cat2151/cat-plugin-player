use super::super::Instrument;

fn patch(params: &[(u32, f64)], names: &[&str]) -> Vec<u8> {
    let mut xml = String::from(r#"<patch id="org.baconpaul.six-sines" version="12"><params>"#);
    for (id, v) in params {
        xml += &format!(r#"<p id="{id}" v="{v:.6}" />"#);
    }
    xml += "</params><macroNames>";
    for (i, name) in names.iter().enumerate() {
        xml += &format!(r#"<macroName idx="{i}"><![CDATA[{name}]]></macroName>"#);
    }
    xml += "</macroNames><dawExtraState><p id=\"40060\" v=\"1\" /></dawExtraState></patch>";
    let mut state = xml.into_bytes();
    state.push(0);
    state
}

fn summary(state: &[u8]) -> Vec<(usize, Option<f64>, Option<String>)> {
    Instrument::of("CLAP", "org.baconpaul.six-sines")
        .unwrap()
        .routed("CLAP", state)
        .unwrap()
        .into_iter()
        .map(|m| {
            assert!(m.bipolar && m.ccs.is_empty());
            (m.number, m.position, m.name)
        })
        .collect()
}

#[test]
fn macro_sources_count_only_with_a_target() {
    let state = patch(
        &[
            // Op 2 Source slot 1: Macro 1 onto a target.
            (1500 + 250 + 152, 400.0),
            (1500 + 250 + 161, 3.0),
            // Op 1 Self slot 0: Macro 2's modulated output with no target.
            (10000 + 40, 411.0),
            (10000 + 55, 0.0),
            // Matrix 15 slot 2: Macro 3's modulated output onto a target.
            (30000 + 200 * 14 + 74, 412.0),
            (30000 + 200 * 14 + 82, 1.0),
            // Main Output slot 0: an LFO, not a macro.
            (620, 300.0),
            (650, 1.0),
            (40000, -1.0),
            (40000 + 500, 0.5),
        ],
        &["Macro 1", "Macro 2", " Growl "],
    );
    assert_eq!(
        summary(&state),
        [(1, Some(0.0), None), (3, Some(0.75), Some("Growl".into()))]
    );
}

#[test]
fn a_macro_node_driven_by_another_macro_uses_it() {
    // Macro 6's slot 1 takes Macro 5; a missing level reads as no saved position.
    let state = patch(
        &[(40000 + 250 * 5 + 52, 404.0), (40000 + 250 * 5 + 61, 2.0)],
        &[],
    );
    assert_eq!(summary(&state), [(5, None, None)]);
    let state = patch(
        &[(50000 + 1000 + 104, 405.0), (50000 + 1000 + 202, 4.0)],
        &[],
    );
    assert_eq!(summary(&state), [(6, None, None)]);
}

#[test]
fn identifies_level_parameters_and_rejects_other_states() {
    let i = Instrument::of("CLAP", "org.baconpaul.six-sines").unwrap();
    assert!(i.parameter(2, "Macro 2 Level"));
    assert!(!i.parameter(2, "Macro 2"));
    assert!(i
        .routed("CLAP", b"<patch id=\"other\"></patch>\0")
        .is_none());
    assert!(i.routed("CLAP", b"").is_none());
    assert!(Instrument::of("VST3", "org.baconpaul.six-sines").is_none());
    assert_eq!(i.range(), (-1.0, 1.0));
}
