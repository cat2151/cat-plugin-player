use super::*;

fn slot(id: &str, state: u8) -> PluginState {
    PluginState {
        reference: None,
        plugin: PluginKey {
            format: "CLAP".into(),
            id: id.into(),
            name: id.into(),
            vendor: "test".into(),
            bundle_path: "X:/plugins/test.clap".into(),
        },
        state: vec![state],
        preset_name: Some("Browser preset".into()),
        bypass: false,
    }
}

#[test]
fn roundtrip_preserves_order_bypass_and_unrelated_events() {
    let original = Event {
        delta: 480,
        tick: 480,
        status: 0xb1,
        meta_type: 0,
        data: vec![1, 70],
    };
    let mut track = Track {
        events: vec![
            Event {
                delta: 0,
                tick: 0,
                status: 0xff,
                meta_type: 0x7f,
                data: vec![1, 2],
            },
            original.clone(),
        ],
    };
    let settings = Settings {
        instrument: Some(slot("instrument", 1)),
        effects: vec![slot("delay", 2), slot("reverb", 3)],
        bypass: true,
        ..Default::default()
    };
    write(&mut track, &settings).unwrap();
    write(&mut track, &settings).unwrap();
    assert_eq!(track.events.len(), 3);
    let music = crate::smf::model::Music {
        format: 0,
        division: 480,
        tracks: vec![track],
    };
    let bytes = crate::smf::io::encode(&music).unwrap();
    let loaded = crate::smf::ffi::parse(&bytes).unwrap();
    assert_eq!(read(&loaded.tracks[0]).unwrap(), settings);
    let cc = &loaded.tracks[0].events[2];
    assert_eq!(
        (cc.tick, cc.status, &cc.data),
        (original.tick, original.status, &original.data)
    );
}

#[test]
fn invalid_settings_are_rejected_without_mutation() {
    let mut track = Track { events: vec![] };
    let bad = Settings {
        version: 2,
        ..Default::default()
    };
    assert!(write(&mut track, &bad).is_err());
    assert!(track.events.is_empty());
    write(&mut track, &Settings::default()).unwrap();
    track.events.push(track.events[0].clone());
    assert!(read(&track).is_err());
    track.events.pop();
    track.events[0].tick = 1;
    assert!(read(&track).is_err());
    track.events[0].tick = 0;
    track.events[0].data.push(0);
    assert!(read(&track).is_err());
    assert!(Settings {
        instrument: Some(slot("", 1)),
        ..Default::default()
    }
    .validate()
    .is_err());
    assert!(Settings {
        effects: vec![slot("fx", 1)],
        ..Default::default()
    }
    .validate()
    .is_err());
}

#[test]
fn same_plugins_ignores_state_and_bypass_but_not_identity_or_order() {
    let base = Settings {
        instrument: Some(slot("instrument", 1)),
        effects: vec![slot("delay", 2), slot("reverb", 3)],
        ..Default::default()
    };
    let mut other = base.clone();
    other.instrument.as_mut().unwrap().state = vec![9];
    other.effects[1].state = vec![9];
    other.bypass = true;
    assert!(base.same_plugins(&other));
    other.effects.swap(0, 1);
    assert!(!base.same_plugins(&other));
    other.effects.pop();
    assert!(!base.same_plugins(&other));
    let mut other = base.clone();
    other.instrument = Some(slot("synth", 1));
    assert!(!base.same_plugins(&other));
    let mut other = base.clone();
    other.instrument.as_mut().unwrap().plugin.name = "renamed".into();
    other.effects[0].plugin.bundle_path = "X:\\plugins\\test.clap".into();
    assert!(base.same_plugins(&other));
    other.effects[0].plugin.bundle_path = "X:/elsewhere/test.clap".into();
    assert!(!base.same_plugins(&other));
    assert!(!base.same_plugins(&Settings::default()));
    assert!(!Settings::default().same_plugins(&Settings::default()));
}
