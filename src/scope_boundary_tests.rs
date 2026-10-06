use super::*;

fn wave(phase: f64) -> Vec<[f32; 2]> {
    (0..320)
        .map(|i| {
            let value = (std::f64::consts::TAU * (i as f64 / 80.0 + phase)).sin() as f32;
            [value, value * 0.5]
        })
        .collect()
}

#[test]
fn transient_harmonic_peak_is_rejected_without_inventing_a_fourth_marker() {
    let mut samples = vec![[0.0; 2]; 980];
    for j in [177, 303, 548, 793] {
        samples[j] = [1.0; 2];
    }
    let measured = measure(&samples, 0);
    assert_eq!(measured.accepted, 3);
    assert_eq!(measured.rejected, 1);
    assert!((measured.phase.unwrap() - 58.0 / 245.0).abs() < 1e-12);
    assert!(measure(&vec![[0.0; 2]; 320], 0).phase.is_none());
}

#[test]
fn boundary_uses_last_completed_reference_and_holds_offset_during_shape_change() {
    let mut tracker = Boundary::default();
    tracker.observe(1, 48, 320, &wave(0.0));
    let previous = tracker.observe(1, 48, 640, &wave(0.02));
    let first = tracker.observe(2, 55, 960, &wave(0.2));
    assert_eq!(first.reason, "boundary_aligned");
    assert_eq!(first.reference_serial, Some(640));
    assert_eq!(first.reference, previous.measured.phase);
    assert_eq!(first.decision_serial, 960);
    assert!(first.shift.abs() > 0.1);
    let changed = tracker.observe(2, 55, 1280, &wave(0.35));
    assert_eq!(first.shift, changed.shift);
    assert_eq!(first.decision_serial, changed.decision_serial);
    assert_ne!(first.measured.phase, changed.measured.phase);
}

#[test]
fn insufficient_first_window_never_causes_a_late_jump_and_gap_discards_reference() {
    let mut tracker = Boundary::default();
    tracker.observe(1, 48, 320, &wave(0.0));
    let quiet = tracker.observe(2, 55, 640, &vec![[0.0; 2]; 320]);
    assert_eq!(quiet.reason, "first_window_unreliable_hold");
    assert_eq!(quiet.shift, 0.0);
    let later = tracker.observe(2, 55, 960, &wave(0.3));
    assert_eq!(later.shift, 0.0);
    assert_eq!(later.decision_serial, 640);
    tracker.reset();
    let reset = tracker.observe(3, 60, 1500, &wave(0.4));
    assert_eq!(reset.reason, "no_previous_reference_hold");
    assert_eq!(reset.reference_serial, None);
}

#[test]
fn arbitrary_future_suffix_cannot_change_any_prefix_decision() {
    let mut prefix = Boundary::default();
    prefix.observe(1, 48, 320, &wave(0.0));
    let mut a = prefix.clone();
    let mut b = prefix.clone();
    let first_a = a.observe(2, 55, 640, &wave(0.2));
    let first_b = b.observe(2, 55, 640, &wave(0.2));
    assert_eq!(first_a, first_b);
    let a_later = a.observe(2, 55, 960, &wave(0.4));
    let b_later = b.observe(2, 55, 960, &wave(-0.2));
    assert_eq!(a_later.shift, b_later.shift);
    assert_eq!(a_later.decision_serial, first_a.available_serial);
    assert!(first_a.reference_serial.unwrap() < first_a.available_serial);
}

#[test]
fn circular_reference_does_not_jump_when_phase_crosses_zero() {
    let mut tracker = Boundary::default();
    for (i, phase) in [0.26, 0.24, 0.25].into_iter().enumerate() {
        tracker.observe(1, 48, (i as u64 + 1) * 320, &wave(phase));
    }
    let record = tracker.observe(2, 55, 1280, &wave(0.25));
    assert!(record.shift.abs() <= 0.02);
}

#[test]
fn overtaking_harmonic_peak_does_not_replace_the_note_boundary_reference() {
    let peaks = |primary, harmonic| {
        let mut samples = vec![[0.0; 2]; 320];
        for cycle in 0..4 {
            samples[cycle * 80 + 20] = [primary; 2];
            samples[cycle * 80 + 60] = [harmonic; 2];
        }
        samples
    };
    let mut tracker = Boundary::default();
    let first = tracker.observe(1, 67, 320, &peaks(1.0, 0.5));
    let last = tracker.observe(1, 67, 640, &peaks(0.5, 1.0));
    assert_eq!(measure(&peaks(0.5, 1.0), 0).phase, Some(0.75));
    assert_eq!(last.measured.phase, first.measured.phase);
    assert_eq!(last.measured.phase, Some(0.25));
    let missing = tracker.observe(1, 67, 800, &peaks(0.0, 1.0));
    assert_eq!(missing.measured.phase, None);
    assert_eq!(missing.shift, first.shift);
    let next = tracker.observe(2, 48, 960, &wave(0.1));
    assert_eq!(next.reference, Some(0.25));
    assert_eq!(next.reference_serial, Some(640));
}
