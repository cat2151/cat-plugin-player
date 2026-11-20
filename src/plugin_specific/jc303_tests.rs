use super::*;

fn plugin(format: &str, id: &str) -> PluginKey {
    PluginKey {
        format: format.into(),
        id: id.into(),
        name: "JC303".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/JC303.clap".into(),
    }
}

fn jc303() -> PluginKey {
    plugin("CLAP", ID)
}

fn state(param: &str) -> Vec<u8> {
    let mut state = b"VC2!\x10\x00\x00\x00".to_vec();
    state.extend_from_slice(
        format!(
            r#"<?xml version="1.0"?><APVTS><PARAM id="seqSwing" value="1.0"/>{param}<PARAM id="seqTempo" value="120.0"/></APVTS>"#
        )
        .as_bytes(),
    );
    state.push(0);
    state
}

#[test]
fn host_becomes_internal_in_either_attribute_order() {
    for (host, internal) in [
        (
            r#"<PARAM id="seqSyncMode" value="1.0"/>"#,
            r#"<PARAM id="seqSyncMode" value="0.0"/>"#,
        ),
        (
            r#"<PARAM value="1.0" id="seqSyncMode"/>"#,
            r#"<PARAM value="0.0" id="seqSyncMode"/>"#,
        ),
    ] {
        assert_eq!(internal_sync(&jc303(), &state(host)), Some(state(internal)));
    }
}

#[test]
fn internal_midi_clock_and_missing_modes_stay() {
    for param in [
        r#"<PARAM id="seqSyncMode" value="0.0"/>"#,
        r#"<PARAM value="2.0" id="seqSyncMode"/>"#,
        r#"<PARAM id="seqSyncMode" value="1.05"/>"#,
        "",
    ] {
        assert_eq!(internal_sync(&jc303(), &state(param)), None, "{param}");
    }
}

#[test]
fn other_plugins_stay() {
    let host = state(r#"<PARAM id="seqSyncMode" value="1.0"/>"#);
    for key in [plugin("VST3", ID), plugin("CLAP", "co.midilab.JC303x")] {
        assert_eq!(internal_sync(&key, &host), None);
    }
}
