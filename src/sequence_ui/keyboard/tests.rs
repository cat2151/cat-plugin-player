use super::*;

#[test]
fn plan_contains_all_phrase_notes_and_excludes_gaps() {
    let steps = Plan::new(SequencePattern::Steps, None);
    assert_eq!(steps.range, Some((48, 67)));
    for note in 0..128 {
        assert_eq!(contains(steps.used, note), [48, 55, 60, 67].contains(&note));
    }
    let phrase = Arc::new(Phrase {
        events: vec![
            (0, [0x90, 61, 100]),
            (10, [0x80, 61, 0]),
            (20, [0x90, 76, 100]),
            (30, [0x80, 76, 0]),
        ],
        duration: 40,
        notes: 2,
    });
    let plan = Plan::new(SequencePattern::Custom, Some(&phrase));
    assert_eq!(plan.range, Some((61, 76)));
    assert!(contains(plan.used, 61) && contains(plan.used, 76));
    assert!(!contains(plan.used, 62));
    assert!(plan.matches(SequencePattern::Custom, Some(&phrase)));
    assert!(!plan.matches(SequencePattern::Custom, Some(&Arc::new((*phrase).clone()))));
    assert_eq!(Plan::new(SequencePattern::Custom, None).range, None);
}

#[test]
fn planned_notes_match_a_full_rendered_loop_for_every_builtin() {
    for &pattern in SequencePattern::TYPES {
        let mut seq = crate::seq::Sequencer::new(1000);
        let mut out = crate::seq::EventBuf::new();
        let mut played = [0; 2];
        for _ in 0..2000 {
            out.clear();
            seq.render(pattern, crate::SequenceVelocity::default(), 10, &mut out);
            for &word in out.as_slice() {
                if word & 0xffff_0000 == 0x2090_0000 {
                    let note = ((word >> 8) & 127) as usize;
                    played[note / 64] |= 1 << (note % 64);
                }
            }
        }
        assert_eq!(Plan::new(pattern, None).used, played, "{pattern:?}");
    }
}

#[test]
fn keyboard_is_read_only_and_handles_single_black_key_and_full_midi_range() {
    for (first, last) in [(61, 61), (0, 127), (48, 67)] {
        let ctx = egui::Context::default();
        let phrase = Arc::new(Phrase {
            events: vec![(0, [0x90, first, 100]), (1, [0x90, last, 100])],
            duration: 2,
            notes: 2,
        });
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, SequencePattern::Custom, Some(&phrase), [0; 2]);
            });
        });
        assert!(!output.shapes.is_empty());
        assert!(!ctx.wants_keyboard_input());
        assert!(!ctx.wants_pointer_input());
    }
}
