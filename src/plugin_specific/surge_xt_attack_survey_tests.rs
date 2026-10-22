use super::*;

#[test]
fn survey_classification_does_not_exclude_failed_candidates() {
    let mut row = Row {
        patch: "test".into(),
        state_sha256: String::new(),
        audio_file: String::new(),
        observations: Vec::new(),
        error: None,
    };
    for (group, level) in [
        ("reference_a", 0.1),
        ("reference_b", 0.1),
        ("candidate_0", 0.05),
    ] {
        for repeat in 1..=30 {
            row.observations.push(Observation {
                group: group.into(),
                repeat,
                order: row.observations.len(),
                pre_note_frames: 0,
                before_rms: None,
                attack_rms: vec![level; 40],
                first_signal_frame: Some(0),
                energy: 1.0,
                peak: level,
            });
        }
    }
    assert_eq!(statistics::baseline(&row.observations), "equivalent");
    assert_eq!(
        statistics::candidate(&row.observations, 0),
        "outside_margin"
    );
    assert_eq!(classification(&row), ("unmeasured".into(), None));
    for repeat in 1..=30 {
        let mut o = row.observations[0].clone();
        o.group = "candidate_10".into();
        o.repeat = repeat;
        o.order = row.observations.len();
        row.observations.push(o);
    }
    assert_eq!(classification(&row), ("evaluable".into(), Some(10)));
    row.error = Some("read failure".into());
    assert_eq!(classification(&row), ("measurement_error".into(), None));
}
