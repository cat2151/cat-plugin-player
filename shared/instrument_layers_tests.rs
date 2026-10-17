use super::*;
use crate::{host::PluginKey, patch_reference::Source, smf::model::Music};

fn reference(program: &str) -> PatchReference {
    PatchReference {
        plugin: PluginKey {
            format: "CLAP".into(),
            id: "dexed".into(),
            name: "Dexed".into(),
            vendor: "test".into(),
            bundle_path: "X:/plugins/dexed.clap".into(),
        },
        source: Source::Catalog {
            path: format!("X:/patches/organ.syx#{program}").into(),
            file: "X:/patches/organ.syx".into(),
        },
        display: "Organ".into(),
    }
}

fn roots() -> crate::patch_roots::Fixture {
    crate::patch_roots::Fixture {
        table: crate::patch_roots::Table {
            plugins: vec![crate::patch_roots::Plugin {
                format: "CLAP".into(),
                id: "dexed".into(),
                bundle_path: "X:/plugins/dexed.clap".into(),
                roots: vec!["X:/patches".into(), "X:/effects/reverb.clap".into()],
            }],
        },
        files: vec![
            "X:/patches/organ.syx".into(),
            "X:/effects/reverb.clap".into(),
        ],
        refreshed: 0,
    }
}

#[test]
fn smf_roundtrip_keeps_distinct_sources_layers_and_unrelated_events() {
    let original = vec![
        Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x51,
            data: vec![7, 161, 32],
        },
        Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x7f,
            data: vec![42, 17],
        },
        Event {
            tick: 480,
            delta: 480,
            status: 0x90,
            meta_type: 0,
            data: vec![60, 100],
        },
        Event {
            tick: 960,
            delta: 480,
            status: 0x80,
            meta_type: 0,
            data: vec![60, 0],
        },
        Event {
            tick: 960,
            delta: 0,
            status: 0xff,
            meta_type: 0x2f,
            data: vec![],
        },
    ];
    let mut track = Track {
        events: original.clone(),
    };
    let settings = Settings {
        layers: vec![
            Layer {
                patch: Some(reference("1")),
                ..Default::default()
            },
            Layer {
                id: 2,
                patch: Some(reference("2")),
                octave: -2,
                volume_db: -6.0,
                mute: true,
                solo: true,
            },
        ],
        effects: vec![Effect {
            patch: PatchReference {
                source: Source::Effect {
                    path: "X:/effects/reverb.clap".into(),
                    value: "Hall 2".into(),
                },
                ..reference("1")
            },
            bypass: true,
        }],
        ..Default::default()
    };
    write(&mut track, &settings, &mut roots()).unwrap();
    write(&mut track, &settings, &mut roots()).unwrap();
    let music = Music {
        format: 0,
        division: 480,
        tracks: vec![track],
    };
    let bytes = crate::smf::io::encode(&music).unwrap();
    let parsed = crate::smf::ffi::parse(&bytes).unwrap();
    assert_eq!(read(&parsed.tracks[0], &mut roots()).unwrap(), settings);
    assert_eq!(
        event_values(&parsed.tracks[0].events[1..]),
        event_values(&original)
    );
    let payload = &parsed.tracks[0].events[0].data[PREFIX.len()..];
    let json: serde_json::Value = serde_json::from_slice(payload).unwrap();
    assert!(json["layers"][0]["patch"].get("state").is_none());
    assert_eq!(
        json["layers"][1]["patch"]["source"]["path"], "organ.syx#2",
        "{json}"
    );
    assert!(!String::from_utf8_lossy(payload).contains("X:"));
    assert_ne!(settings.layers[0].patch, settings.layers[1].patch);
}

#[test]
fn bad_metadata_never_mutates_the_track() {
    let mut track = Track { events: vec![] };
    write(&mut track, &Settings::default(), &mut roots()).unwrap();
    for kind in 0..4 {
        let mut bad = track.clone();
        match kind {
            0 => bad.events.push(bad.events[0].clone()),
            1 => bad.events[0].tick = 1,
            2 => bad.events[0].data.extend(b"!"),
            _ => {
                bad.events[0].data = [PREFIX, br#"{"version":2,"layers":[],"effects":[]}"#].concat()
            }
        }
        let before = bad.clone();
        assert!(read(&bad, &mut roots()).is_err());
        assert!(write(&mut bad, &Settings::default(), &mut roots()).is_err());
        assert_eq!(event_values(&bad.events), event_values(&before.events));
    }
}

fn event_values(events: &[Event]) -> Vec<(u64, u32, u8, u8, Vec<u8>)> {
    events
        .iter()
        .map(|e| (e.tick, e.delta, e.status, e.meta_type, e.data.clone()))
        .collect()
}

#[test]
fn numeric_boundaries_and_duplicate_ids_are_validated() {
    let mut layer = Layer {
        octave: i32::MAX,
        ..Default::default()
    };
    assert_eq!(layer.pitch(60), None);
    layer.octave = i32::MIN;
    assert_eq!(layer.pitch(60), None);
    layer.octave = -1;
    assert_eq!(layer.pitch(12), Some(0));
    assert_eq!(layer.pitch(11), None);
    layer.volume_db = -6.0206;
    assert!((layer.gain().unwrap() - 0.5).abs() < 0.00001);
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
        layer.volume_db = value;
        assert!(layer.gain().is_err());
    }
    let mut settings = Settings::default();
    settings.layers.push(Layer::default());
    assert!(settings.validate().is_err());
    settings.layers.clear();
    assert!(settings.validate().is_ok());
}
