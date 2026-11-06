use super::*;

#[test]
fn trims_only_named_contiguous_edge_runs_and_keeps_the_original_boundaries() {
    let mut kit = crate::drum_browser::tests::candidate("kit");
    kit.measurement.drum_kit_notes = Some((0..=127).rev().collect());
    kit.measurement.drum_kit_note_names = (0..=127)
        .map(|n| {
            (
                n,
                match n {
                    0..=36 => "Kick 2".into(),
                    37..=38 => "Snare".into(),
                    80..=127 => "Triangle".into(),
                    _ => format!("Drum {n}"),
                },
            )
        })
        .collect();
    assert_eq!(main_notes(&kit), (36..=80).collect::<Vec<_>>());
    assert_eq!(notes(&kit).unwrap(), (0..=127).collect::<Vec<_>>());
    for rate in [44100, 48000] {
        let program = crate::drum_preview::program(&main_notes(&kit), rate);
        let mut player = crate::drum_preview::Player::default();
        player.replace(program);
        let mut events = crate::seq::EventBuf::new();
        player.render(rate, 1, &mut events);
        assert_eq!(events.as_slice(), &[crate::seq::note_on(0, 36, 127)]);
        assert_eq!(program.pitches[80] >> 8, 44 * u64::from(rate) / 4 + 1);
        assert!(program.pitches[..36].iter().all(|&n| n == 0));
        assert!(program.pitches[81..].iter().all(|&n| n == 0));
        events.clear();
        player.replace(crate::preview::Notes::default());
        player.render(rate, rate as usize * 40, &mut events);
        assert_eq!(events.as_slice(), &[crate::seq::note_off(0, 36)]);
        assert!(!player.active());
    }
}

#[test]
fn unknown_names_gaps_and_single_name_kits_do_not_lose_all_notes() {
    let mut kit = crate::drum_browser::tests::candidate("kit");
    kit.measurement.drum_kit_notes = Some(vec![0, 1, 2, 4, 5]);
    kit.measurement.drum_kit_note_names.clear();
    assert_eq!(main_notes(&kit), vec![0, 1, 2, 4, 5]);
    kit.measurement.drum_kit_note_names = vec![(0, " ".into()), (1, " ".into())];
    assert_eq!(main_notes(&kit), vec![0, 1, 2, 4, 5]);
    kit.measurement.drum_kit_note_names = (0..=5).map(|n| (n, "Kick".into())).collect();
    assert_eq!(main_notes(&kit), vec![2, 4]);
    kit.measurement.drum_kit_notes = Some(vec![0, 1, 2]);
    assert_eq!(main_notes(&kit), vec![2]);
    kit.measurement.drum_kit_notes = None;
    assert!(main_notes(&kit).is_empty());
}

#[test]
fn native_restored_main_kit_waits_for_metadata_and_consumes_autoplay_once() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    let kit = crate::drum_browser::tests::candidate("kit");
    app.drum_browser.saved.applied = Some(identity(&kit));
    app.patch_browser.restore_playing = Some(identity(&kit));
    app.auto_start_pending = true;
    app.poll_browser_load();
    assert!(app.auto_start_pending);
    app.patch_browser.requests.applied = Some(kit);
    app.poll_browser_load();
    assert!(!app.auto_start_pending);
    assert!(app.drum_browser.main_audition);
    assert_eq!(app.sequence_pattern, crate::SequencePattern::Off);
    app.stop_drum_audio();
    app.poll_browser_load();
    assert!(!app.drum_browser.main_audition);
}
