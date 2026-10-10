use super::*;
use crate::patch_browser_catalog::tests::candidate;

#[test]
fn latest_only_a_b_c_discards_prepared_a_and_never_starts_b() {
    let mut requests = Requests::default();
    requests.select(candidate("a", "Alpha", "A", None), 1);
    assert_eq!(requests.next().unwrap().candidate.display, "A");
    requests.select(candidate("a", "Alpha", "B", None), 1);
    requests.select(candidate("a", "Alpha", "C", None), 1);
    assert!(requests.next().is_none());
    assert!(requests.ready().is_none());
    assert_eq!(requests.next().unwrap().candidate.display, "C");
    assert!(requests.ready().is_some());
    requests.finish(true);
    assert_eq!(requests.applied.as_ref().unwrap().display, "C");
    assert!(requests.next().is_none());
}

#[test]
fn normal_to_heavy_confirmation_cancel_close_and_generation_gate() {
    let mut requests = Requests::default();
    requests.select(candidate("a", "Alpha", "A", None), 1);
    requests.next().unwrap();
    let mut heavy = candidate("b", "Beta", "Heavy", None);
    heavy.measurement.sfz_sample_bytes = Some(64_000_000);
    requests.select(heavy.clone(), 1);
    assert!(requests.ready().is_none());
    requests.enter(); // Enter outside the confirmation cannot authorize heavy work.
    assert!(requests.next().is_none());
    requests.space();
    requests.confirmation = None; // Esc
    requests.enter();
    assert!(requests.next().is_none());
    requests.space();
    requests.select(heavy.clone(), 2); // rescan invalidates old confirmation
    requests.enter();
    assert!(requests.next().is_none());
    requests.space();
    requests.enter();
    requests.next().unwrap();
    requests.invalidate(); // close after confirmation, before prepared result
    assert!(requests.ready().is_none());
    assert!(requests.next().is_none());
    assert!(requests.applied.is_none());
    requests.select(heavy, 3);
    assert!(
        requests.next().is_none(),
        "reopening never reuses heavy authorization"
    );
}

#[test]
fn already_applying_finishes_and_failure_does_not_retry_or_replace_applied() {
    let mut requests = Requests::default();
    requests.select(candidate("a", "Alpha", "A", None), 1);
    requests.next();
    requests.ready();
    requests.select(candidate("a", "Alpha", "B", None), 1);
    assert!(requests.next().is_none());
    requests.finish(true);
    assert_eq!(requests.next().unwrap().candidate.display, "B");
    assert!(requests.failed_preparation());
    assert!(requests.next().is_none());
    assert_eq!(requests.applied.unwrap().display, "A");
}

#[test]
fn shared_heavy_boundary_unknown_and_unchanged_selection() {
    for (bytes, heavy) in [
        (None, false),
        (Some(63_999_999), false),
        (Some(64_000_000), true),
    ] {
        let mut candidate = candidate("a", "Alpha", "A", None);
        candidate.measurement.sfz_sample_bytes = bytes;
        let mut requests = Requests::default();
        requests.select(candidate.clone(), 1);
        let serial = requests.selected.as_ref().unwrap().serial;
        requests.select(candidate, 1);
        assert_eq!(requests.selected.as_ref().unwrap().serial, serial);
        assert_eq!(requests.desired.is_none(), heavy);
    }
}
