use super::*;
use crate::seq::{note_off, note_on, EventBuf, Sequencer};
use crate::{SequencePattern, SequenceVelocity};

#[test]
fn delivered_gates_ignore_timestamps_controllers_and_other_channels() {
    let meter = Meter::default();
    let mut tracker = Tracker::default();
    tracker.record(
        &[
            note_on(0, 60, 100),
            note_on(0, 67, 80),
            crate::seq::jr_timestamp(128),
            0x20b0_017f,
            note_on(1, 72, 100),
        ],
        &meter,
    );
    let notes = meter.snapshot();
    assert!(contains(notes, 60) && contains(notes, 67));
    assert!(!contains(notes, 1) && !contains(notes, 72));
    tracker.record(&[note_off(0, 60), note_on(0, 67, 0)], &meter);
    assert_eq!(meter.snapshot(), [0; 2]);
}

#[test]
fn switching_and_stopping_release_the_displayed_notes_for_builtin_and_custom() {
    for pattern in SequencePattern::TYPES
        .iter()
        .copied()
        .chain([SequencePattern::Custom])
    {
        let meter = Meter::default();
        let mut tracker = Tracker::default();
        let mut seq = Sequencer::new(48000);
        seq.set_phrase(Some(std::sync::Arc::new(
            crate::timed_sequence::Phrase::parse("ceg", 48000).unwrap(),
        )));
        let mut out = EventBuf::new();
        seq.render(pattern, SequenceVelocity::default(), 480, &mut out);
        tracker.record(out.as_slice(), &meter);
        assert_ne!(meter.snapshot(), [0; 2], "{pattern:?}");
        out.clear();
        seq.render(
            SequencePattern::Steps,
            SequenceVelocity::default(),
            480,
            &mut out,
        );
        tracker.record(out.as_slice(), &meter);
        assert_eq!(meter.snapshot(), [1 << 48, 0]);
        out.clear();
        seq.render(
            SequencePattern::Off,
            SequenceVelocity::default(),
            480,
            &mut out,
        );
        tracker.record(out.as_slice(), &meter);
        assert_eq!(meter.snapshot(), [0; 2]);
    }
}
