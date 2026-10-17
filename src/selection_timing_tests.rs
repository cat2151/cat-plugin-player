use crate::{
    chain_settings::Settings,
    selection::Selection,
    selection_tests::{directory, input},
};
use std::{sync::atomic::Ordering, sync::Arc};

#[test]
fn preview_starts_mid_song_restores_held_note_and_uses_whole_song_end() {
    let root = directory("start timing");
    let path = root.join("input.mid");
    input(&path, &Settings::default());
    let original = std::fs::read(&path).unwrap();
    let mut selection = Selection::load(path.clone(), 1000).unwrap();
    selection.set_timing(1000, 12.5, Some(60.0)).unwrap();
    assert_eq!(selection.position.load(Ordering::Acquire), 12500);
    assert_eq!(selection.timeline.duration, 60000);
    let mut player = crate::smf_timeline::LoopPlayer::new(
        selection.timeline.clone(),
        selection.position.clone(),
        0,
    );
    player.render(500);
    assert_eq!(player.position, 13000);
    assert!(player.words.contains(&0x2093_3c51));
    player.render(37001);
    assert!(player.words.contains(&0x2083_3c00));
    assert!(player.position > 50000);
    player.render(10000);
    assert_eq!(player.position, 1);
    assert!(player.words.contains(&0x2093_3c51));
    drop(player);
    // Playback timing never rewrites the SMF's original performance.
    assert_eq!(std::fs::read(&path).unwrap(), original);
    selection.save(&Settings::default()).unwrap();
    let before = crate::smf::ffi::parse(&original).unwrap();
    let after = crate::smf::io::load(&path).unwrap();
    let events = |music: &crate::smf::model::Music| {
        music.tracks[0]
            .events
            .iter()
            .map(|e| (e.tick, e.status, e.meta_type, e.data.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(events(&before), events(&after));
}

#[test]
fn invalid_timing_is_rejected_without_changing_cursor_or_duration() {
    let root = directory("bad start timing");
    let path = root.join("input.mid");
    input(&path, &Settings::default());
    let mut selection = Selection::load(path, 1000).unwrap();
    let original_end = selection.timeline.duration;
    for (start, end) in [
        (f64::NAN, None),
        (-1.0, None),
        (f64::INFINITY, None),
        (60.0, None),
        (1.0, Some(1.5)),
        (60.0, Some(60.0)),
        (1.0, Some(f64::INFINITY)),
    ] {
        assert!(selection.set_timing(1000, start, end).is_err());
        assert_eq!(selection.position.load(Ordering::Acquire), 0);
        assert_eq!(selection.timeline.duration, original_end);
    }
    let _audio_owner = Arc::clone(&selection.timeline);
    assert!(selection.set_timing(1000, 1.0, None).is_err());
    assert_eq!(selection.position.load(Ordering::Acquire), 0);
}
