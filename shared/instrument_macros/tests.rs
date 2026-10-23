use super::*;

fn vaporizer2(xml: &str) -> Vec<u8> {
    let mut state = b"VC2!".to_vec();
    state.extend((xml.len() as u32).to_le_bytes());
    state.extend(xml.as_bytes());
    state.push(0);
    state
}

#[test]
fn vaporizer2_lists_matrix_sources_with_positions_names_and_row_polarity() {
    let state = vaporizer2(
        r#"<VASTvaporizer2 PatchVersion="VASTVaporizerParamsV2.20000" CustomModulator1Text="LPF" CustomModulator3Text="Custom3" CustomModulator4Text="REVERB">
  <PARAM id="m_fCustomModulator1" text="100"/>
  <PARAM id="m_fCustomModulator2" text="60"/>
  <PARAM id="m_fCustomModulator3" text="30"/>
  <PARAM id="m_fCustomModulator4" text="101"/>
  <PARAM id="m_uModMatSrce1" text="CustomModulator3"/>
  <PARAM id="m_uModMatSrce2" text="LFO1"/>
  <PARAM id="m_uModMatSrce3" text="CustomModulator1"/>
  <PARAM id="m_uModMatSrce4" text="CustomModulator4"/>
  <PARAM id="m_uModMatSrce5" text="CustomModulator5"/>
  <PARAM id="m_uModMatSrce7" text="CustomModulator4"/>
  <PARAM id="m_uModMatPolarity1" text="Bipolar"/>
  <PARAM id="m_uModMatPolarity3" text="Unipolar"/>
  <PARAM id="m_uModMatPolarity4" text="Bipolar"/>
  <PARAM id="m_uModMatPolarity7" text="Bipolar"/>
  <PARAM id="m_uModMatPolarity2" text="Bipolar"/>
  <chunkData><PARAM id="m_uModMatSrce6" text="CustomModulator2"/></chunkData>
</VASTvaporizer2>"#,
    );
    let instrument = Instrument::of("CLAP", "com.vastdynamics.VAST2").unwrap();
    let macros = instrument.routed("CLAP", &state).unwrap();
    let summary: Vec<_> = macros
        .iter()
        .map(|m| (m.number, m.position, m.bipolar, m.name.as_deref()))
        .collect();
    assert_eq!(
        summary,
        [
            (1, Some(1.0), false, Some("LPF")),
            (3, Some(0.3), true, None),
            (4, None, true, Some("REVERB")),
        ]
    );
    assert!(instrument.parameter(1, "Custom modulator 1"));
    assert!(!instrument.parameter(1, "Custom modulator 10"));
    assert!(instrument.routed("CLAP", b"VC2!").is_none());
    assert!(Instrument::of("VST3", "com.vastdynamics.VAST2").is_none());
}

#[test]
fn vaporizer2_macro_is_bipolar_only_when_every_driven_row_is() {
    let state = |second: &str| {
        vaporizer2(&format!(
            r#"<VASTvaporizer2><PARAM id="m_fCustomModulator2" text="50"/><PARAM id="m_uModMatSrce1" text="CustomModulator2"/><PARAM id="m_uModMatSrce2" text="CustomModulator2"/><PARAM id="m_uModMatPolarity1" text="Bipolar"/><PARAM id="m_uModMatPolarity2" text="{second}"/></VASTvaporizer2>"#
        ))
    };
    let bipolar = |second| {
        Instrument::Vaporizer2
            .routed("CLAP", &state(second))
            .unwrap()[0]
            .bipolar
    };
    assert!(bipolar("Bipolar"));
    assert!(!bipolar("Unipolar"));
}
