use crate::{
    seq::*, sequence_modulation::SequenceModulation, sequence_pattern::SequencePattern,
    sequence_velocity::SequenceVelocity, timed_sequence::Phrase,
};
use std::sync::Arc;

fn collect(
    seq: &mut Sequencer,
    pattern: SequencePattern,
    start: usize,
    frames: usize,
) -> Vec<(usize, u32)> {
    let mut buf = EventBuf::new();
    seq.render(pattern, SequenceVelocity::V100, frames, &mut buf);
    let mut at = start;
    buf.as_slice()
        .iter()
        .filter_map(|&word| {
            if word & 0xffff_0000 == 0x0020_0000 {
                at += (word & 0xffff) as usize;
                None
            } else {
                Some((at, word))
            }
        })
        .collect()
}

#[test]
fn converted_tempo_rhythm_chords_and_trailing_rest_survive_block_slicing_and_delay() {
    for input in ["t120 o4 l8 crdt240 e4r4", "C F G C"] {
        let phrase = Arc::new(Phrase::parse(input, 48000).unwrap());
        assert!(phrase.notes > 1);
        let expected: Vec<_> = phrase
            .events
            .iter()
            .map(|(at, m)| {
                (
                    *at as usize + 4800,
                    if m[0] == 0x90 {
                        note_on(0, m[1], 100)
                    } else {
                        note_off(0, m[1])
                    },
                )
            })
            .collect();
        for block in [64, 257, 1024] {
            let mut seq = Sequencer::new(48000);
            seq.set_phrase(Some(phrase.clone()));
            seq.delay_start(std::time::Duration::from_millis(100));
            let limit = phrase.duration as usize + 4801;
            let mut actual = Vec::new();
            for start in (0..limit).step_by(block) {
                actual.extend(collect(
                    &mut seq,
                    SequencePattern::Custom,
                    start,
                    block.min(limit - start),
                ));
            }
            // Include the next loop's first sample only when no leading rest exists.
            let first_cycle: Vec<_> = actual.iter().copied().take(expected.len()).collect();
            assert_eq!(first_cycle, expected, "{input} block {block}");
            assert_eq!(
                actual
                    .iter()
                    .filter(|(at, w)| *at >= limit - 1 && w & 0xfff0_0000 == 0x2090_0000)
                    .count(),
                usize::from(phrase.events[0].0 == 0)
                    * phrase.events.iter().take_while(|(at, _)| *at == 0).count()
            );
        }
    }
}

#[test]
fn restart_and_stop_release_custom_notes_and_preserve_leading_rest() {
    let mut seq = Sequencer::new(1000);
    let phrase = Arc::new(Phrase::parse("t120 o4 r8 c1r2", 1000).unwrap());
    seq.set_phrase(Some(phrase.clone()));
    let notes = collect(&mut seq, SequencePattern::Custom, 0, 1000);
    let on = notes
        .iter()
        .find(|(_, w)| w & 0xfff0_0000 == 0x2090_0000)
        .unwrap()
        .1;
    seq.restart();
    let restarted = collect(&mut seq, SequencePattern::Custom, 0, 1000);
    assert_eq!(restarted[0], (0, note_off(0, ((on >> 8) & 127) as u8)));
    assert_eq!(
        restarted
            .iter()
            .find(|(_, w)| w & 0xfff0_0000 == 0x2090_0000)
            .unwrap()
            .0,
        phrase.events[0].0 as usize
    );
    let stopped = collect(&mut seq, SequencePattern::Off, 0, 1000);
    assert!(stopped.iter().any(|(_, w)| w & 0xfff0_0000 == 0x2080_0000));
    seq.restart();
    assert!(collect(&mut seq, SequencePattern::Off, 0, 1000)
        .iter()
        .all(|(_, w)| w & 0xfff0_0000 != 0x2090_0000));
}

#[test]
fn saturation_releases_every_emitted_note_and_can_restart() {
    let mut seq = Sequencer::new(1000);
    let mut events = Vec::new();
    for note in 0..128 {
        events.push((0, [0x90, note, 100]));
    }
    for note in 0..128 {
        events.push((1, [0x80, note, 0]));
    }
    seq.set_phrase(Some(Arc::new(Phrase {
        events,
        duration: 2,
        notes: 128,
    })));
    let output = collect(&mut seq, SequencePattern::Custom, 0, 1000);
    let mut sounding = [false; 128];
    for (_, word) in output {
        match word & 0xfff0_0000 {
            0x2090_0000 => sounding[((word >> 8) & 127) as usize] = true,
            0x2080_0000 => sounding[((word >> 8) & 127) as usize] = false,
            _ => {}
        }
    }
    assert!(!sounding.iter().any(|s| *s));
    assert!(collect(&mut seq, SequencePattern::Custom, 0, 1000).is_empty());
    seq.restart();
    assert!(collect(&mut seq, SequencePattern::Custom, 0, 1000)
        .iter()
        .any(|(_, w)| w & 0xfff0_0000 == 0x2090_0000));
}

#[test]
fn velocity_and_cc1_still_apply_without_changing_phrase_times() {
    let phrase = Arc::new(Phrase::parse("t120 c4d4r4", 1000).unwrap());
    let mut seq = Sequencer::new(1000);
    seq.set_phrase(Some(phrase));
    seq.set_modulation(SequenceModulation::Sweep);
    let first = collect(&mut seq, SequencePattern::Custom, 0, 1);
    assert!(first.iter().any(|(_, w)| *w == note_on(0, 60, 100)));
    let mut buf = EventBuf::new();
    seq.render(
        SequencePattern::Custom,
        SequenceVelocity::V127,
        600,
        &mut buf,
    );
    assert!(buf.as_slice().contains(&note_on(0, 62, 127)));
    assert!(buf
        .as_slice()
        .iter()
        .any(|w| w & 0xffff_ff00 == 0x20b0_0100));
}

#[test]
fn switching_to_custom_keeps_the_last_delivered_fixed_cc1_meter() {
    let mut seq = Sequencer::new(1000);
    seq.set_phrase(Some(Arc::new(Phrase::parse("c4r4", 1000).unwrap())));
    seq.set_modulation(SequenceModulation::Full);
    collect(&mut seq, SequencePattern::Steps, 0, 1);
    assert_eq!(seq.current_values().1, 127);
    let custom = collect(&mut seq, SequencePattern::Custom, 0, 1);
    assert!(custom
        .iter()
        .any(|(_, word)| word & 0xfff0_0000 == 0x2090_0000));
    assert!(!custom
        .iter()
        .any(|(_, word)| word & 0xffff_ff00 == 0x20b0_0100));
    assert_eq!(seq.current_values().1, 127);
}
