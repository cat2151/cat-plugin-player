use crate::seq::{note_on, EventBuf, Sequencer, NOTES};
use crate::sequence_modulation::{cc, cc1, MacroLanes, MACROS, MACRO_HALF_PERIOD_MS};
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
                seq.set_modulation(Some(M::Sweep));
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
    seq.set_modulation(Some(M::Fixed(127)));
    assert_eq!(
        render(&mut seq, P::Steps, 100, 100),
        vec![(0, cc1(127)), (0, note_on(0, NOTES[0], 100))]
    );
    seq.set_modulation(Some(M::Fixed(64)));
    assert_eq!(render(&mut seq, P::Steps, 10, 10), vec![(0, cc1(64))]);
    seq.set_modulation(Some(M::Fixed(0)));
    assert_eq!(render(&mut seq, P::Steps, 90, 90), vec![(0, cc1(0))]);
    assert!(render(&mut seq, P::Steps, 50, 50).is_empty());
    let next = render(&mut seq, P::Steps, 1, 1);
    assert!(next.contains(&(0, note_on(0, NOTES[1], 100))));
}

#[test]
fn sweep_stops_and_restarts_from_zero_and_choice_survives_serialization() {
    let mut seq = Sequencer::new(1000);
    seq.set_modulation(Some(M::Sweep));
    render(&mut seq, P::Steps, 2001, 64);
    let stopped = render(&mut seq, P::Off, 4000, 64);
    assert!(stopped.iter().all(|(_, w)| w & 0xFFFFFF00 != 0x20B00100));
    assert_eq!(render(&mut seq, P::Steps, 1, 1)[0], (0, cc1(0)));
    seq.set_modulation(Some(M::Fixed(127)));
    assert!(render(&mut seq, P::Off, 1, 1).contains(&(0, cc1(127))));
    let old: crate::status::Status = toml::from_str("sequence_pattern = 'steps'").unwrap();
    assert_eq!(old.sequence_modulation, None);
    assert_eq!(old.macro_modulation, [None; MACROS]);
    // A dragged value is saved and reaches the audio thread like a listed one.
    for choice in M::CHOICES.into_iter().chain([Some(M::Fixed(64))]) {
        assert_eq!(M::from_u8(M::to_u8(choice)), choice);
        let mut macro_modulation = [None; MACROS];
        macro_modulation[2] = choice;
        let config = crate::status::Status {
            sequence_modulation: choice,
            macro_modulation,
            ..Default::default()
        };
        let restored: crate::status::Status =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.sequence_modulation, choice);
        assert_eq!(restored.macro_modulation, macro_modulation);
    }
}

fn controller(events: &[(usize, u32)], cc: u32) -> Vec<(usize, u32)> {
    let status = 0x20B0_0000 | cc << 8;
    events
        .iter()
        .copied()
        .filter(|(_, w)| w & 0xFFFF_FF00 == status)
        .collect()
}

fn macros(lanes: &[(usize, u8, M)]) -> MacroLanes {
    let mut macros = [None; MACROS];
    for &(index, controller, choice) in lanes {
        macros[index] = Some((controller, choice));
    }
    macros
}

#[test]
fn macro_sweeps_each_take_their_own_period_on_their_own_controllers() {
    let rate = 1270;
    let total = rate as usize * 8 + 1;
    let mut seq = Sequencer::new(rate);
    seq.set_modulation(Some(M::Sweep));
    seq.set_macro_modulation(&macros(&[(0, 41, M::Sweep), (7, 48, M::Sweep)]));
    let events = render(&mut seq, P::Steps, total, 64);
    let mut last = [0; MACROS];
    for (index, controller_number) in [(0, 41), (7, 48)] {
        let half_period_ms = MACRO_HALF_PERIOD_MS[index];
        let sent: Vec<_> = (0u64..)
            .map(|step| {
                let phase = step % 254;
                let value = if phase <= 127 { phase } else { 254 - phase };
                let at = (step * half_period_ms * u64::from(rate)).div_ceil(127_000);
                (at as usize, cc(controller_number, value as u8))
            })
            .take_while(|&(at, _)| at < total)
            .collect();
        last[index] = (sent.last().unwrap().1 & 0x7F) as u8;
        assert_eq!(controller(&events, u32::from(controller_number)), sent);
    }
    assert_eq!(controller(&events, 1).len(), 509);
    let (_, cc1, macro_values) = seq.current_values();
    assert_eq!((cc1, macro_values), (0, last));
}

#[test]
fn macro_periods_differ_from_cc1_and_each_other() {
    let mut periods = MACRO_HALF_PERIOD_MS.to_vec();
    periods.push(2000);
    periods.sort_unstable();
    periods.dedup();
    assert_eq!(periods.len(), MACROS + 1);
}

#[test]
fn macro_lane_without_choice_or_controller_sends_nothing_and_controller_change_restarts() {
    let mut seq = Sequencer::new(1000);
    seq.set_macro_modulation(&macros(&[]));
    assert!(render(&mut seq, P::Off, 100, 100).is_empty());
    seq.set_macro_modulation(&macros(&[(0, 41, M::Fixed(127)), (1, 42, M::Fixed(0))]));
    assert_eq!(
        render(&mut seq, P::Off, 1, 1),
        vec![(0, cc(41, 127)), (0, cc(42, 0))]
    );
    assert!(render(&mut seq, P::Off, 100, 100).is_empty());
    seq.set_macro_modulation(&macros(&[(0, 20, M::Fixed(127)), (1, 42, M::Fixed(0))]));
    assert_eq!(render(&mut seq, P::Off, 1, 1), vec![(0, cc(20, 127))]);
    seq.set_macro_modulation(&macros(&[]));
    assert!(render(&mut seq, P::Steps, 1000, 64)
        .iter()
        .all(|(_, w)| w & 0xFFFF_0000 != 0x20B0_0000));
}

#[test]
fn dash_sends_nothing_and_a_patch_change_releases_only_fixed_values() {
    assert_eq!(M::label(None), "-");
    assert_eq!(M::adjacent(None, true), Some(M::Fixed(0)));
    assert_eq!(M::adjacent(None, false), Some(M::Sweep));
    assert_eq!(M::adjacent(Some(M::Sweep), true), None);
    for choice in M::CHOICES {
        assert_eq!(M::from_u8(M::to_u8(choice)), choice);
    }
    for (before, after, released) in [
        (Some(M::Fixed(0)), None, true),
        (Some(M::Fixed(127)), None, true),
        (Some(M::Sweep), Some(M::Sweep), false),
        (None, None, false),
    ] {
        let mut choice = before;
        assert_eq!(M::release_fixed(&mut choice), released);
        assert_eq!(choice, after);
    }
    let mut seq = Sequencer::new(1000);
    seq.set_modulation(Some(M::Fixed(127)));
    render(&mut seq, P::Off, 1, 1);
    seq.set_modulation(None);
    assert!(controller(&render(&mut seq, P::Steps, 100, 100), 1).is_empty());
}

#[test]
fn macro_toggle_sweeps_all_unless_every_shown_macro_already_sweeps() {
    let mut shown = [None; MACROS];
    shown[0] = Some(41);
    shown[2] = Some(43);
    let mut choices = [None; MACROS];
    choices[0] = Some(M::Sweep);
    choices[1] = Some(M::Fixed(64));
    M::toggle_sweep(&mut choices, &shown);
    assert_eq!(choices, [Some(M::Sweep); MACROS]);
    // A hidden macro set to "-" does not keep the shown sweeps from turning off.
    choices[5] = None;
    M::toggle_sweep(&mut choices, &shown);
    assert_eq!(choices, [None; MACROS]);
}
