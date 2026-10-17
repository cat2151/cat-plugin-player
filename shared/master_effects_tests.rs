use super::*;
use crate::{
    chain_settings::PluginState,
    host::PluginKey,
    smf::{io, model::Track},
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
                    tick: 240,
                    delta: 240,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 90],
                }],
            },
        ],
    }
}
fn settings() -> Settings {
    Settings {
        effects: vec![PluginState {
            reference: None,
            plugin: PluginKey {
                format: "CLAP".into(),
                id: "effect".into(),
                name: "Effect".into(),
                vendor: "fixture".into(),
                bundle_path: "X:/plugins/effect.clap".into(),
            },
            state: vec![1, 2, 3],
            preset_name: Some("Room".into()),
            bypass: false,
        }],
        ..Default::default()
    }
}
#[test]
fn master_round_trip_preserves_conductor_performance_and_track_chains() {
    let mut music = music();
    let original = music.clone();
    crate::chain_settings::write(&mut music.tracks[0], &TrackSettings::default()).unwrap();
    let track_settings = crate::chain_settings::read(&music.tracks[0]).unwrap();
    let mut master = settings();
    write(&mut music, &master).unwrap();
    master.effects[0].bypass = true;
    write(&mut music, &master).unwrap();
    let mut returned = crate::smf::ffi::parse(&io::encode(&music).unwrap()).unwrap();
    assert_eq!(read(&returned).unwrap(), master);
    assert_eq!(
        crate::chain_settings::read(&returned.tracks[0]).unwrap(),
        track_settings
    );
    assert_eq!(
        returned.tracks[0]
            .events
            .iter()
            .filter(|e| owned(e))
            .count(),
        1
    );
    assert_eq!(
        returned.tracks[1].events[0].data,
        original.tracks[1].events[0].data
    );
    write(&mut returned, &Settings::default()).unwrap();
    assert_eq!(read(&returned).unwrap(), Settings::default());
    assert!(!returned.tracks[0].events.iter().any(owned));
    assert!(returned.tracks[0]
        .events
        .iter()
        .any(|e| e.meta_type == 0x51 && e.data == vec![7, 161, 32]));
}
#[test]
fn master_rejects_duplicate_misplaced_unknown_and_non_format_one_metadata() {
    let mut music = music();
    write(&mut music, &settings()).unwrap();
    let meta = music.tracks[0].events[0].clone();
    music.tracks[0].events.push(meta.clone());
    assert!(read(&music).is_err());
    music.tracks[0].events.pop();
    music.tracks[0].events[0].tick = 1;
    assert!(read(&music).is_err());
    music.tracks[0].events[0].tick = 0;
    music.tracks[0].events.remove(0);
    music.tracks[1].events.insert(0, meta);
    assert!(read(&music).is_err());
    let mut music = super::tests::music();
    let mut invalid = settings();
    invalid.version = 2;
    assert!(write(&mut music, &invalid).is_err());
    for format in [0, 2] {
        music.format = format;
        assert!(write(&mut music, &settings()).is_err());
    }
}
