const SILENCE: f64 = 0.000001;
const WINDOW_SAMPLES: usize = 240 * 2;

#[derive(Debug, serde::Serialize)]
pub(super) struct Comparison {
    pub matched: bool,
    pub max_db: f64,
    pub silence_mismatches: usize,
    pub audible_windows: usize,
}

pub(super) fn rms(samples: &[f32]) -> Vec<f64> {
    assert_eq!(samples.len(), 48_000 / 5 * 2);
    samples
        .as_chunks::<WINDOW_SAMPLES>()
        .0
        .iter()
        .map(|window| {
            assert!(window.iter().all(|x| x.is_finite()), "non-finite audio");
            (window.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / window.len() as f64).sqrt()
        })
        .collect()
}

pub(super) fn compare(first: &[f64], reference: &[f64]) -> Comparison {
    assert_eq!(first.len(), 40);
    assert_eq!(first.len(), reference.len());
    let mut result = Comparison {
        matched: false,
        max_db: 0.0,
        silence_mismatches: 0,
        audible_windows: 0,
    };
    for (&a, &b) in first.iter().zip(reference) {
        match (a > SILENCE, b > SILENCE) {
            (true, true) => {
                result.audible_windows += 1;
                result.max_db = result.max_db.max((20.0 * (a / b).log10()).abs());
            }
            (false, false) => {}
            _ => result.silence_mismatches += 1,
        }
    }
    result.matched = result.silence_mismatches == 0 && result.max_db <= 1.0;
    result
}

#[test]
fn envelope_contract_covers_threshold_silence_and_stereo_energy() {
    let reference = [0.1; 40];
    assert!(compare(&reference, &reference).matched);
    assert!(compare(&[0.1 * 10.0f64.powf(0.99 / 20.0); 40], &reference).matched);
    assert!(!compare(&[0.1 * 10.0f64.powf(1.01 / 20.0); 40], &reference).matched);
    assert!(!compare(&[0.0; 40], &reference).matched);
    assert!(compare(&[0.0; 40], &[0.0; 40]).matched);
    let mut quiet = reference;
    quiet[0] = SILENCE;
    assert_eq!(compare(&quiet, &reference).silence_mismatches, 1);
    assert!(compare(&quiet, &quiet).matched);
    let stereo: Vec<_> = (0..9600).flat_map(|_| [0.0, 1.0]).collect();
    assert!(rms(&stereo)
        .iter()
        .all(|x| (x - 0.5f64.sqrt()).abs() < 1e-12));
}
