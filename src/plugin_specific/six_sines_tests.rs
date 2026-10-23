use super::*;

fn plugin() -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "org.baconpaul.six-sines".into(),
        name: "Six Sines".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/Six Sines.clap".into(),
    }
}

/// Macro levels 1-6 sit at 40000 + 250k; 40001 is the parameter after Macro 1 Level.
fn patch(levels: [f64; 6], neighbour: f64, extra: &str) -> Vec<u8> {
    let mut xml = String::from(r#"<patch id="org.baconpaul.six-sines" version="12"><params>"#);
    for (k, v) in levels.iter().enumerate() {
        xml += &format!(r#"<p id="{}" v="{v:.6}" />"#, 40000 + 250 * k);
    }
    xml += &format!(r#"<p id="40001" v="{neighbour:.6}" />"#);
    xml +=
        r#"</params><macroNames><macroName idx="0"><![CDATA[Macro 1]]></macroName></macroNames>"#;
    xml += &format!(r#"<dawExtraState>{extra}</dawExtraState></patch>"#);
    let mut state = xml.into_bytes();
    state.push(0);
    state
}

const LEVELS: [f64; 6] = [0.0, -1.0, 1.0, 0.5, -0.25, 0.75];

#[test]
fn only_macro_level_values_are_excluded() {
    let a = patch(LEVELS, 0.3, "");
    let b = patch([1.0, 0.0, -1.0, -0.5, 0.25, -0.75], 0.3, "");
    assert!(same_sound(&plugin(), &a, &b));
    assert!(crate::plugin_specific::same_favorite_state(
        &plugin(),
        &a,
        &b
    ));
    for other in [
        patch(LEVELS, 0.4, ""),
        patch([1.5, -1.0, 1.0, 0.5, -0.25, 0.75], 0.3, ""),
        patch([0.0, -1.0, 1.0, 0.5, -0.25, f64::NAN], 0.3, ""),
        patch(LEVELS, 0.3, r#"<p id="40000" v="1" />"#),
    ] {
        assert!(!same_sound(&plugin(), &a, &other));
    }
    let text = |state: &[u8]| String::from_utf8(state[..state.len() - 1].to_vec()).unwrap();
    let with_nul = |xml: String| {
        let mut state = xml.into_bytes();
        state.push(0);
        state
    };
    // A Macro level outside `<params>` is still compared.
    let moved = |state: &[u8]| {
        with_nul(
            text(state)
                .replace("<params>", "<params><other>")
                .replace("</params>", "</other></params>"),
        )
    };
    assert!(!same_sound(&plugin(), &moved(&a), &moved(&b)));
    // The macro name is part of the sound's identity.
    let renamed = with_nul(text(&a).replace("[Macro 1]", "[Bright]"));
    assert!(!same_sound(&plugin(), &a, &renamed));
    let mut trailing = b.clone();
    trailing.push(0);
    assert!(!same_sound(&plugin(), &a, &trailing));
    let other_root = |state: &[u8]| with_nul(text(state).replace("six-sines", "other"));
    assert!(!same_sound(&plugin(), &other_root(&a), &other_root(&b)));
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
}
