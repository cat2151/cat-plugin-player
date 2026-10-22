use super::Observation;

const SILENCE: f64 = 1e-6;

// Two tails of Binomial(30, 1/2), through 4, times 9 groups x 40 windows.
pub(super) fn failure_bound() -> f64 {
    360.0 * 2.0 * (1 + 30 + 435 + 4060 + 27405) as f64 / 2.0_f64.powi(30)
}

pub(super) fn compare(left: &[&Observation], right: &[&Observation]) -> &'static str {
    assert_eq!(left.len(), 30);
    assert_eq!(right.len(), 30);
    let mut silent = 0;
    let mut outside = false;
    let mut uncertain = false;
    for window in 0..40 {
        let values = |group: &[&Observation]| {
            let mut values: Vec<_> = group.iter().map(|o| o.attack_rms[window]).collect();
            assert!(values.iter().all(|v| v.is_finite() && *v >= 0.0));
            values.sort_by(f64::total_cmp);
            values
        };
        let a = values(left);
        let b = values(right);
        let audible_a = a.iter().filter(|&&x| x > SILENCE).count();
        let audible_b = b.iter().filter(|&&x| x > SILENCE).count();
        if audible_a == 0 && audible_b == 0 {
            silent += 1;
        } else if audible_a != 30 || audible_b != 30 {
            uncertain = true;
        } else {
            let lo = 20.0 * a[4].log10() - 20.0 * b[25].log10();
            let hi = 20.0 * a[25].log10() - 20.0 * b[4].log10();
            outside |= lo > 1.0 || hi < -1.0;
            uncertain |= lo < -1.0 || hi > 1.0;
        }
    }
    if silent == 40 {
        return "excluded_silent";
    }
    if left
        .iter()
        .chain(right)
        .any(|o| o.first_signal_frame.is_none() || o.attack_rms.iter().all(|&v| v <= SILENCE))
    {
        return "indeterminate_missing";
    }
    if outside {
        "outside_margin"
    } else if uncertain {
        "indeterminate"
    } else {
        "equivalent"
    }
}

pub(super) fn baseline(observations: &[Observation]) -> &'static str {
    let group = |name| {
        observations
            .iter()
            .filter(|o| o.group == name)
            .collect::<Vec<_>>()
    };
    compare(&group("reference_a"), &group("reference_b"))
}

pub(super) fn candidate(observations: &[Observation], delay: u64) -> &'static str {
    let name = format!("candidate_{delay}");
    let candidate: Vec<_> = observations.iter().filter(|o| o.group == name).collect();
    if candidate.len() != 30 {
        return "unmeasured";
    }
    let a = compare(
        &candidate,
        &observations
            .iter()
            .filter(|o| o.group == "reference_a")
            .collect::<Vec<_>>(),
    );
    let b = compare(
        &candidate,
        &observations
            .iter()
            .filter(|o| o.group == "reference_b")
            .collect::<Vec<_>>(),
    );
    if a == "equivalent" && b == "equivalent" {
        "equivalent"
    } else if a == "outside_margin" || b == "outside_margin" {
        "outside_margin"
    } else {
        "indeterminate"
    }
}

#[test]
fn simultaneous_median_contract_and_missing_attacks() {
    assert!(failure_bound() <= 0.05);
    let make = |level| Observation {
        group: "reference_a".into(),
        repeat: 1,
        order: 0,
        pre_note_frames: 48128,
        before_rms: None,
        attack_rms: vec![level; 40],
        first_signal_frame: (level > 0.0).then_some(0),
        energy: 1.0,
        peak: level,
    };
    let a = vec![make(0.1); 30];
    fn refs(g: &[Observation]) -> Vec<&Observation> {
        g.iter().collect()
    }
    assert_eq!(compare(&refs(&a), &refs(&a)), "equivalent");
    let b = vec![make(0.1 * 10.0_f64.powf(1.01 / 20.0)); 30];
    assert_eq!(compare(&refs(&a), &refs(&b)), "outside_margin");
    let zero = vec![make(0.0); 30];
    assert_eq!(compare(&refs(&zero), &refs(&zero)), "excluded_silent");
    assert_eq!(compare(&refs(&zero), &refs(&a)), "indeterminate_missing");
    let mut mixed = a.clone();
    mixed[0].attack_rms[0] = 0.0;
    assert_eq!(compare(&refs(&mixed), &refs(&a)), "indeterminate");
    for (i, o) in mixed.iter_mut().enumerate() {
        o.attack_rms = vec![0.1 * 10.0_f64.powf(i as f64 / 20.0); 40];
    }
    assert_eq!(compare(&refs(&mixed), &refs(&mixed)), "indeterminate");
}
