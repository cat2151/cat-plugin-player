use crate::seq::{note_on, EventBuf, Sequencer, NOTES};
use crate::sequence_modulation::cc1;
use crate::{SequenceModulation as M, SequencePattern as P, SequenceVelocity as V};

fn render(seq: &mut Sequencer, pattern: P, total: usize, block: usize) -> Vec<(usize, u32)> {
    let mut events = Vec::new();
    let mut done = 0;
    while done < total {
        let frames = block.min(total - done);
        let mut buf = EventBuf::new();
        seq.render(pattern, V::V100, frames, &mut buf);
        let mut at = done;
        for &word in buf.as_slice() {
            if word >> 16 == 0x0020 {
                at += (word & 0xFFFF) as usize;
            } else {
                assert!(at < done + frames);
                events.push((at, word));
            }
        }
        done += frames;
    }
    events
}

#[test]
fn sweep_has_exact_endpoints_and_repeats_independent_of_blocks_and_notes() {
    for rate in [44_100, 48_000] {
        for block in [64, 1024, 30_000] {
            for &pattern in P::TYPES {
                let mut seq = Sequencer::new(rate);
                seq.set_modulation(M::Sweep);
                let events = render(&mut seq, pattern, rate as usize * 8 + 1, block);
                let cc: Vec<_> = events
                    .iter()
                    .copied()
                    .filter(|(_, w)| w & 0xFFFFFF00 == 0x20B00100)
                    .collect();
                let expected: Vec<_> = (0..=508u64)
                    .map(|step| {
                        let phase = step % 254;
                        let value = if phase <= 127 { phase } else { 254 - phase };
                        (
                            ((step * 2 * u64::from(rate)).div_ceil(127)) as usize,
                            cc1(value as u8),
                        )
                    })
                    .collect();
                assert_eq!(cc, expected, "rate {rate}, block {block}, {pattern:?}");
                let mut baseline = Sequencer::new(rate);
                let notes = render(&mut baseline, pattern, rate as usize * 8 + 1, block);
                assert_eq!(
                    events
                        .into_iter()
                        .filter(|(_, w)| w & 0xFFFFFF00 != 0x20B00100)
                        .collect::<Vec<_>>(),
                    notes
                );
            }
        }
    }
}

#[test]
fn fixed_values_switch_immediately_without_restarting_notes() {
    let mut seq = Sequencer::new(1000);
    seq.set_modulation(M::Full);
    assert_eq!(
        render(&mut seq, P::Steps, 100, 100),
        vec![(0, cc1(127)), (0, note_on(0, NOTES[0], 100))]
    );
    seq.set_modulation(M::Zero);
    assert_eq!(render(&mut seq, P::Steps, 100, 100), vec![(0, cc1(0))]);
    assert!(render(&mut seq, P::Steps, 50, 50).is_empty());
    let next = render(&mut seq, P::Steps, 1, 1);
    assert!(next.contains(&(0, note_on(0, NOTES[1], 100))));
}

#[test]
fn sweep_stops_and_restarts_from_zero_and_choice_survives_serialization() {
    let mut seq = Sequencer::new(1000);
    seq.set_modulation(M::Sweep);
    render(&mut seq, P::Steps, 2001, 64);
    let stopped = render(&mut seq, P::Off, 4000, 64);
    assert!(stopped.iter().all(|(_, w)| w & 0xFFFFFF00 != 0x20B00100));
    assert_eq!(render(&mut seq, P::Steps, 1, 1)[0], (0, cc1(0)));
    seq.set_modulation(M::Full);
    assert!(render(&mut seq, P::Off, 1, 1).contains(&(0, cc1(127))));
    let old: crate::status::Status = toml::from_str("sequence_pattern = 'steps'").unwrap();
    assert_eq!(old.sequence_modulation, M::Zero);
    for &choice in M::TYPES {
        let config = crate::status::Status {
            sequence_modulation: choice,
            ..Default::default()
        };
        let restored: crate::status::Status =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.sequence_modulation, choice);
    }
}
