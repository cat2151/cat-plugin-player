use super::*;
use crate::{
    host::PluginKey,
    patch_reference::{PatchReference, Source},
    smf::model::Event,
};

fn music() -> Music {
    Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![
                    Event {
                        tick: 0,
                        delta: 0,
                        status: 0xff,
                        meta_type: 0x51,
                        data: vec![7, 161, 32],
                    },
                    Event {
                        tick: 480,
                        delta: 480,
                        status: 0xff,
                        meta_type: 0x2f,
                        data: vec![],
                    },
                ],
            },
            Track {
                events: vec![Event {
                    tick: 0,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x7f,
                    data: vec![42],
                }],
            },
        ],
    }
}
fn settings() -> Effects {
    Effects {
        effects: vec![instrument_layers::Effect {
            patch: PatchReference {
                plugin: PluginKey {
                    format: "CLAP".into(),
                    id: "test.effect".into(),
                    name: "Effect".into(),
                    vendor: "Test".into(),
                    bundle_path: "X:/plugins/effect.clap".into(),
                },
                source: Source::Effect {
                    path: "X:/presets/hall.preset".into(),
                    value: "Hall 2".into(),
                },
                display: "Hall".into(),
            },
            bypass: true,
        }],
    }
}

fn roots() -> crate::patch_roots::Fixture {
    crate::patch_roots::Fixture {
        table: crate::patch_roots::Table {
            plugins: vec![crate::patch_roots::Plugin {
                format: "CLAP".into(),
                id: "test.effect".into(),
                bundle_path: "X:/plugins/effect.clap".into(),
                roots: vec!["X:/presets".into()],
            }],
        },
        files: vec!["X:/presets/hall.preset".into()],
        refreshed: 0,
    }
}

#[test]
fn master_reference_smf_roundtrip_preserves_preset_value_other_events_and_removes_empty_meta() {
    let mut music = music();
    let original = crate::smf::io::encode(&music).unwrap();
    let settings = settings();
    write(&mut music, &settings, &mut roots()).unwrap();
    let parsed = crate::smf::ffi::parse(&crate::smf::io::encode(&music).unwrap()).unwrap();
    assert_eq!(read(&parsed, &mut roots()).unwrap(), settings);
    assert!(!String::from_utf8_lossy(&music.tracks[0].events[0].data).contains("\"state\""));
    write(&mut music, &Effects::default(), &mut roots()).unwrap();
    assert_eq!(crate::smf::io::encode(&music).unwrap(), original);
}

#[test]
fn master_reference_invalid_placement_duplicates_version_and_legacy_meta_never_overwrite() {
    let mut good = music();
    write(&mut good, &settings(), &mut roots()).unwrap();
    for case in 0..6 {
        let mut bad = good.clone();
        match case {
            0 => {
                let duplicate = bad.tracks[0].events[0].clone();
                bad.tracks[0].events.insert(0, duplicate);
            }
            1 => bad.tracks[0].events[0].tick = 1,
            2 => {
                let event = bad.tracks[0].events.remove(0);
                bad.tracks[1].events.insert(0, event);
            }
            3 => bad.format = 2,
            4 => {
                let event = &mut bad.tracks[0].events[0];
                let mut value: serde_json::Value =
                    serde_json::from_slice(&event.data[PREFIX.len()..]).unwrap();
                value["version"] = 2.into();
                event.data = [PREFIX, &serde_json::to_vec(&value).unwrap()].concat();
            }
            _ => bad.tracks[0].events[0].data = crate::master_effects::PREFIX.to_vec(),
        }
        let before = format!("{bad:?}");
        assert!(read(&bad, &mut roots()).is_err());
        assert!(write(&mut bad, &Effects::default(), &mut roots()).is_err());
        assert_eq!(format!("{bad:?}"), before);
    }
}
