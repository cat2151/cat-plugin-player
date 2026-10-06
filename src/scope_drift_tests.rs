use super::*;
use crate::scope_boundary::{Measurement, Record};

fn record(epoch: u64, serial: u64, decision: u64, phase: Option<f64>) -> Record {
    Record {
        available_serial: serial,
        epoch,
        note: 48,
        channel: 0,
        measured: Measurement {
            phase,
            accepted: 4,
            rejected: 0,
            markers: vec![],
        },
        reference: None,
        shift: 0.0,
        reason: "boundary_aligned",
        decision_serial: decision,
        reference_serial: (epoch > 1).then_some(serial - 100),
    }
}

#[test]
fn gradual_drift_uses_sample_time_and_missing_landmarks_hold_position() {
    let mut drift = Drift::default();
    assert_eq!(drift.observe(&record(1, 100, 100, Some(0.2)), 1_000), 0.0);
    let shift = drift.observe(&record(1, 200, 100, Some(0.18)), 1_000);
    assert!((shift - 0.02 * (1.0 - (-0.2_f64).exp())).abs() < 1e-12);
    assert_eq!(drift.observe(&record(1, 300, 100, None), 1_000), shift);
    // Time since the previous completed window, not OS repaint time.
    let next = drift.observe(&record(1, 400, 100, Some(0.16)), 1_000);
    assert!((next - (shift + (0.04 - shift) * (1.0 - (-0.2_f64).exp()))).abs() < 1e-12);
}

#[test]
fn note_boundary_inherits_displayed_phase_and_keeps_the_position_target() {
    let mut drift = Drift::default();
    drift.observe(&record(1, 100, 100, Some(0.2)), 1_000);
    let old = drift.observe(&record(1, 200, 100, Some(0.18)), 1_000);
    let next = drift.observe(&record(2, 300, 300, Some(0.8)), 1_000);
    assert!(wrap(0.8 + next - 0.18 - old).abs() < 1e-12);
    assert_eq!(drift.target, Some(0.2));
    assert!(next.abs() <= 0.5);
    let mut gap = record(2, 500, 500, Some(0.7));
    gap.reference_serial = None;
    assert_eq!(drift.observe(&gap, 1_000), 0.0);
    assert_eq!(drift.target, Some(0.7));
}

#[test]
fn unreliable_first_window_does_not_enable_correction_later() {
    let mut drift = Drift::default();
    drift.observe(&record(1, 100, 100, None), 1_000);
    assert_eq!(drift.observe(&record(1, 200, 100, Some(0.4)), 1_000), 0.0);
    drift.reset();
    assert_eq!(drift.observe(&record(1, 300, 300, Some(0.1)), 1_000), 0.0);
}

#[test]
fn phase_wrap_is_small_and_channel_change_starts_a_new_target() {
    let mut drift = Drift::default();
    drift.observe(&record(1, 100, 100, Some(0.999)), 1_000);
    let shift = drift.observe(&record(1, 200, 100, Some(0.001)), 1_000);
    assert!(shift < 0.0 && shift.abs() < 0.002);
    let mut right = record(2, 300, 300, Some(0.6));
    right.channel = 1;
    assert_eq!(drift.observe(&right, 1_000), 0.0);
    assert_eq!(drift.target, Some(0.6));
}
