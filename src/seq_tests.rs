use super::*;

#[test]
fn startup_wait_preserves_first_note_and_phrase_timing_across_blocks() {
    for rate in [44_100, 48_000] {
        for block in [64, 256, 1024] {
            let mut seq = Sequencer::new(rate);
            seq.delay_start(std::time::Duration::from_millis(100));
            let mut events = Vec::new();
            let mut buf = EventBuf::new();
            for start in (0..rate as usize / 2).step_by(block) {
                buf.clear();
                seq.render(
                    SequencePattern::Steps,
                    SequenceVelocity::V100,
                    block,
                    &mut buf,
                );
                let mut at = start;
                for &word in buf.as_slice() {
                    if word & 0xFFFF_0000 == 0x0020_0000 {
                        at += (word & 0xFFFF) as usize;
                    } else {
                        events.push((at, word));
                    }
                }
            }
            assert_eq!(
                note_ons(&events),
                vec![
                    (rate as usize / 10, NOTES[0]),
                    (rate as usize * 35 / 100, NOTES[1])
                ]
            );
        }
    }
}

#[test]
fn startup_wait_elapses_while_stopped_and_does_not_repeat_on_pattern_change() {
    let mut seq = Sequencer::new(1000);
    seq.delay_start(std::time::Duration::from_millis(100));
    let mut buf = EventBuf::new();
    seq.render(SequencePattern::Off, SequenceVelocity::V100, 100, &mut buf);
    assert!(buf.as_slice().is_empty());
    seq.render(SequencePattern::Steps, SequenceVelocity::V100, 1, &mut buf);
    assert_eq!(buf.as_slice(), &[note_on(0, NOTES[0], 100)]);
    buf.clear();
    seq.render(
        SequencePattern::GuitarArpeggio,
        SequenceVelocity::V100,
        1,
        &mut buf,
    );
    assert_eq!(
        buf.as_slice(),
        &[note_off(0, NOTES[0]), note_on(0, GUITAR_NOTES[0], 100)]
    );
}

#[test]
fn velocity_changes_apply_to_next_notes_without_restarting_either_pattern() {
    for pattern in [SequencePattern::Steps, SequencePattern::GuitarArpeggio] {
        let mut seq = Sequencer::new(1000);
        let mut buf = EventBuf::new();
        seq.render(pattern, SequenceVelocity::V100, 1, &mut buf);
        let first_note = if pattern == SequencePattern::Steps {
            NOTES[0]
        } else {
            GUITAR_NOTES[0]
        };
        assert_eq!(buf.as_slice(), &[note_on(0, first_note, 100)]);
        buf.clear();
        seq.render(pattern, SequenceVelocity::V127, 250, &mut buf);
        let ons: Vec<_> = buf
            .as_slice()
            .iter()
            .copied()
            .filter(|w| w & 0xFFF0_0000 == 0x2090_0000)
            .collect();
        let second_note = if pattern == SequencePattern::Steps {
            NOTES[1]
        } else {
            GUITAR_NOTES[1]
        };
        assert_eq!(ons, vec![note_on(0, second_note, 127)]);
    }
}

#[test]
fn ranged_velocity_alternates_whole_phrases_independent_of_blocks() {
    for &(pattern, count) in &[
        (SequencePattern::Steps, 4),
        (SequencePattern::GuitarArpeggio, 6),
        (SequencePattern::Csus4CArpeggio, 6),
        (SequencePattern::Fmaj7G6Arpeggio, 8),
    ] {
        for (choice, low, high) in [
            (SequenceVelocity::Range40To100, 40, 100),
            (SequenceVelocity::Range80To127, 80, 127),
        ] {
            for block in [1, 137, 1024] {
                let mut seq = Sequencer::new(1000);
                let mut velocities = Vec::new();
                while velocities.len() < count * 3 {
                    let mut buf = EventBuf::new();
                    seq.render(pattern, choice, block, &mut buf);
                    velocities.extend(
                        buf.as_slice()
                            .iter()
                            .filter(|w| **w & 0xFFF0_0000 == 0x2090_0000)
                            .map(|w| (w & 0x7F) as u8),
                    );
                }
                let up: Vec<u8> = (0..count)
                    .map(|i| low + ((high - low) as usize * i / (count - 1)) as u8)
                    .collect();
                let down: Vec<u8> = (0..count)
                    .map(|i| high - ((high - low) as usize * i / (count - 1)) as u8)
                    .collect();
                assert_eq!(&velocities[..count], up);
                assert_eq!(&velocities[count..count * 2], down);
                assert_eq!(&velocities[count * 2..count * 3], up);
                let mut buf = EventBuf::new();
                seq.render(SequencePattern::Off, choice, 1, &mut buf);
                assert_eq!(seq.current_values().0, 0);
                buf.clear();
                seq.render(pattern, choice, 1, &mut buf);
                assert_eq!(seq.current_values().0, low);
            }
        }
    }
}

#[test]
fn old_random_velocity_migrates_to_upper_range() {
    let config: crate::status::Status = toml::from_str("sequence_velocity = 'random'").unwrap();
    assert_eq!(config.sequence_velocity, SequenceVelocity::Range80To127);
    assert!(!toml::to_string(&config).unwrap().contains("random"));
}

/// Renders `total` samples in blocks of `block` and returns (absolute sample, word)
/// for every note on/off.
fn run(
    seq: &mut Sequencer,
    pattern: SequencePattern,
    total: usize,
    block: usize,
    start: usize,
) -> Vec<(usize, u32)> {
    let mut events = Vec::new();
    let mut done = 0;
    while done < total {
        let frames = block.min(total - done);
        let mut buf = EventBuf::new();
        seq.render(pattern, SequenceVelocity::default(), frames, &mut buf);
        let mut offset = 0;
        for &word in buf.as_slice() {
            if word >> 16 == 0x0020 {
                offset += (word & 0xFFFF) as usize;
            } else {
                assert!(offset < frames, "event outside its block");
                events.push((start + done + offset, word));
            }
        }
        done += frames;
    }
    events
}

fn note_ons(events: &[(usize, u32)]) -> Vec<(usize, u8)> {
    events
        .iter()
        .filter(|(_, w)| w & 0xFFF0_0000 == 0x2090_0000)
        .map(|&(t, w)| (t, ((w >> 8) & 0x7F) as u8))
        .collect()
}

#[test]
fn steps_land_on_exact_samples_whatever_the_block_size() {
    for block in [64, 480, 1024, 1000, 12000, 30000] {
        let mut seq = Sequencer::new(48_000);
        let events = run(&mut seq, SequencePattern::Steps, 96_000, block, 0);
        let ons = note_ons(&events);
        let expected: Vec<(usize, u8)> = (0..8).map(|i| (i * 12_000, NOTES[i % 4])).collect();
        assert_eq!(ons, expected, "block size {block}");
    }
}

#[test]
fn every_note_is_released_when_the_next_one_starts() {
    let mut seq = Sequencer::new(44_100);
    let events = run(&mut seq, SequencePattern::Steps, 44_100, 512, 0);
    let mut sounding: Option<u8> = None;
    for &(_, w) in &events {
        let note = ((w >> 8) & 0x7F) as u8;
        if w & 0xFFF0_0000 == 0x2080_0000 {
            assert_eq!(sounding.take(), Some(note));
        } else {
            assert_eq!(sounding, None, "note on while another note sounds");
            sounding = Some(note);
        }
    }
}

#[test]
fn switching_off_releases_the_note_and_restarts_from_the_top() {
    let mut seq = Sequencer::new(48_000);
    let first = run(&mut seq, SequencePattern::Steps, 13_000, 1024, 0); // steps at 0 and 12000
    assert_eq!(note_ons(&first), vec![(0, NOTES[0]), (12_000, NOTES[1])]);

    let off = run(&mut seq, SequencePattern::Off, 2048, 1024, 13_000);
    assert_eq!(off, vec![(13_000, note_off(0, NOTES[1]))]);

    let again = run(&mut seq, SequencePattern::Steps, 1024, 1024, 15_048);
    assert_eq!(note_ons(&again), vec![(15_048, NOTES[0])]);
}

#[test]
fn guitar_notes_overlap_release_together_and_repeat_after_silence() {
    for rate in [44_100, 48_000] {
        for block in [64, 1024, 1000, 30000] {
            let mut seq = Sequencer::new(rate);
            let events = run(
                &mut seq,
                SequencePattern::GuitarArpeggio,
                rate as usize * 10,
                block,
                0,
            );
            let sample = |ms: usize| ms * rate as usize / 1000;
            let note_offsets = [0, 175, 325, 450, 550, 625];
            let mut expected = Vec::new();
            for cycle in 0..3 {
                for (i, note) in GUITAR_NOTES.iter().enumerate() {
                    expected.push((
                        sample(cycle * 3125 + note_offsets[i]),
                        note_on(0, *note, VELOCITY),
                    ));
                }
                for note in GUITAR_NOTES {
                    expected.push((sample(cycle * 3125 + 2625), note_off(0, note)));
                }
            }
            for (i, note) in GUITAR_NOTES.iter().enumerate() {
                let at = sample(9375 + note_offsets[i]);
                if at < rate as usize * 10 {
                    expected.push((at, note_on(0, *note, VELOCITY)));
                }
            }
            assert_eq!(events, expected, "rate {rate}, block {block}");
        }
    }
}

#[test]
fn chord_cycles_use_half_guitar_intervals_and_release_before_alternating() {
    type Case = (SequencePattern, &'static [&'static [u8]], &'static [usize]);
    let cases: [Case; 2] = [
        (
            SequencePattern::Csus4CArpeggio,
            &[&[60, 65, 67], &[60, 64, 67]],
            &[0, 87_500, 162_500],
        ),
        (
            SequencePattern::Fmaj7G6Arpeggio,
            &[&[53, 69, 72, 76], &[55, 71, 74, 76]],
            &[0, 87_500, 162_500, 225_000],
        ),
    ];
    for (pattern, chords, offsets) in cases {
        for rate in [44_100, 48_000] {
            for block in [64, 1000, 1024, 30_000] {
                let mut seq = Sequencer::new(rate);
                let phrase = offsets.last().unwrap() + 2_500_000;
                let sample = |micros: usize| micros * rate as usize / 1_000_000;
                let events = run(&mut seq, pattern, sample(phrase * 4), block, 0);
                let mut expected = Vec::new();
                for cycle in 0..4 {
                    let chord = chords[cycle % 2];
                    for (&note, &offset) in chord.iter().zip(offsets) {
                        expected
                            .push((sample(cycle * phrase + offset), note_on(0, note, VELOCITY)));
                    }
                    for &note in chord {
                        expected.push((
                            sample(cycle * phrase + offsets.last().unwrap() + 2_000_000),
                            note_off(0, note),
                        ));
                    }
                }
                assert_eq!(events, expected, "{pattern:?}, rate {rate}, block {block}");
            }
        }
    }
}

#[test]
fn changing_or_stopping_chord_cycles_releases_notes_and_restarts_first_chord() {
    for (pattern, chord) in [
        (SequencePattern::Csus4CArpeggio, &[60, 65, 67][..]),
        (SequencePattern::Fmaj7G6Arpeggio, &[53, 69, 72, 76][..]),
    ] {
        for &next in std::iter::once(&SequencePattern::Off).chain(SequencePattern::TYPES) {
            if next == pattern {
                continue;
            }
            let mut seq = Sequencer::new(48_000);
            run(&mut seq, pattern, 12_000, 1024, 0);
            let changed = run(&mut seq, next, 1, 1, 12_000);
            let expected: Vec<_> = chord
                .iter()
                .map(|&note| (12_000, note_off(0, note)))
                .collect();
            assert_eq!(&changed[..chord.len()], expected);
            run(&mut seq, SequencePattern::Off, 1, 1, 12_001);
            let restarted = run(&mut seq, pattern, 1, 1, 12_002);
            assert_eq!(restarted, vec![(12_002, note_on(0, chord[0], VELOCITY))]);
        }
    }
}

#[test]
fn switching_patterns_releases_all_guitar_notes_and_restarts() {
    for elapsed in [1, 12_001, 30_001, 126_001] {
        for pattern in [SequencePattern::Off, SequencePattern::Steps] {
            let mut seq = Sequencer::new(48_000);
            let events = run(&mut seq, SequencePattern::GuitarArpeggio, elapsed, 1024, 0);
            let sounding: Vec<_> = GUITAR_NOTES
                .iter()
                .copied()
                .filter(|note| {
                    events
                        .iter()
                        .any(|(_, word)| *word == note_on(0, *note, VELOCITY))
                        && !events.iter().any(|(_, word)| *word == note_off(0, *note))
                })
                .collect();
            let changed = run(&mut seq, pattern, 1024, 1024, elapsed);
            let mut expected: Vec<_> = sounding
                .iter()
                .map(|note| (elapsed, note_off(0, *note)))
                .collect();
            if pattern == SequencePattern::Steps {
                expected.push((elapsed, note_on(0, NOTES[0], VELOCITY)));
            }
            assert_eq!(changed, expected);
            run(&mut seq, SequencePattern::Off, 1024, 1024, elapsed + 1024);
            let restarted = run(
                &mut seq,
                SequencePattern::GuitarArpeggio,
                1024,
                1024,
                elapsed + 2048,
            );
            assert_eq!(
                restarted,
                vec![(elapsed + 2048, note_on(0, GUITAR_NOTES[0], VELOCITY))]
            );
        }
    }
}

#[test]
fn stop_retries_note_offs_that_do_not_fit_in_the_buffer() {
    let mut seq = Sequencer::new(48_000);
    run(&mut seq, SequencePattern::GuitarArpeggio, 30_001, 1024, 0);
    let mut buf = EventBuf::new();
    while buf.remaining() > 2 {
        assert!(buf.push(0));
    }
    seq.render(
        SequencePattern::Off,
        SequenceVelocity::default(),
        1024,
        &mut buf,
    );
    assert_eq!(
        &buf.as_slice()[254..],
        &[note_off(0, GUITAR_NOTES[0]), note_off(0, GUITAR_NOTES[1])]
    );
    buf.clear();
    seq.render(
        SequencePattern::Off,
        SequenceVelocity::default(),
        1024,
        &mut buf,
    );
    let expected: Vec<_> = GUITAR_NOTES[2..]
        .iter()
        .map(|&note| note_off(0, note))
        .collect();
    assert_eq!(buf.as_slice(), expected);
    buf.clear();
    seq.render(
        SequencePattern::Off,
        SequenceVelocity::default(),
        1024,
        &mut buf,
    );
    assert!(buf.as_slice().is_empty());
}

#[test]
fn same_pattern_restart_releases_hold_and_discards_remaining_wait() {
    for elapsed in [700, 2850] {
        let mut seq = Sequencer::new(1000);
        let mut buf = EventBuf::new();
        seq.render(
            SequencePattern::GuitarArpeggio,
            SequenceVelocity::V100,
            elapsed,
            &mut buf,
        );
        let sounding = seq.sounding;
        buf.clear();
        seq.restart();
        seq.render(
            SequencePattern::GuitarArpeggio,
            SequenceVelocity::V100,
            1,
            &mut buf,
        );
        for (note, active) in sounding.iter().enumerate() {
            if *active {
                assert!(buf.as_slice().contains(&note_off(0, note as u8)));
            }
        }
        assert_eq!(
            buf.as_slice().last(),
            Some(&note_on(0, GUITAR_NOTES[0], 100))
        );
        buf.clear();
        seq.render(SequencePattern::Off, SequenceVelocity::V100, 1, &mut buf);
        buf.clear();
        seq.restart();
        seq.render(SequencePattern::Off, SequenceVelocity::V100, 1000, &mut buf);
        assert!(!buf
            .as_slice()
            .iter()
            .any(|w| w & 0xfff0_0000 == 0x2090_0000));
    }
}
