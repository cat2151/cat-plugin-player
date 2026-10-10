use super::*;
use rand::{rngs::StdRng, SeedableRng};
use std::time::{Duration, Instant};

pub(super) fn candidate() -> Candidate {
    Candidate {
        format: "CLAP".into(),
        plugin_id: crate::random_patch_catalog::SURGE_ID.into(),
        plugin_name: "Surge XT".into(),
        display: "Bass/One.fxp".into(),
        path: "X:/patches/One.fxp".into(),
        bundle_path: "X:/plugins/Surge.clap".into(),
        entry: cmrt_patch_select::PatchCatalogEntry::from_display("Bass/One.fxp".into()),
        measurement: Default::default(),
    }
}

#[test]
fn selection_empty_single_multiple_and_repeat_are_valid() {
    let mut rng = StdRng::seed_from_u64(42);
    assert_eq!(choose(std::iter::empty::<usize>(), &mut rng), None);
    assert_eq!(choose([8].into_iter(), &mut rng), Some(8));
    for _ in 0..20 {
        assert!((0..3).contains(&choose(0..3, &mut rng).unwrap()));
    }
    assert_eq!(choose([8].into_iter(), &mut rng), Some(8));
}

#[test]
fn owned_preparation_is_nonblocking_rejects_second_request_and_clears_failure() {
    let mut preparing = Preparation::default();
    let (release, wait) = mpsc::channel();
    preparing.start_with(candidate(), Default::default(), move |_| {
        wait.recv().unwrap();
        Err("patch disappeared".into())
    });
    assert!(preparing.busy());
    assert!(preparing.poll().is_none());
    preparing.start_with(candidate(), Default::default(), |_| {
        panic!("second request queued")
    });
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = preparing.poll() {
            assert!(result.err().unwrap().contains("disappeared"));
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(!preparing.busy());
    assert!(preparing.poll().is_none());
}

#[test]
fn missing_patch_conversion_has_no_native_dependency() {
    let mut preparing = Preparation::default();
    preparing.start(candidate(), &Default::default());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = preparing.poll() {
            assert!(result.is_err());
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

mod native;

#[test]
fn empty_surge_backup_is_rejected_before_native_mutation() {
    let mut key = crate::config::PluginKey {
        format: "CLAP".into(),
        id: crate::random_patch_catalog::SURGE_ID.into(),
        name: "Surge XT".into(),
        vendor: String::new(),
        bundle_path: String::new(),
    };
    assert!(crate::random_patch_apply::backup(key.clone(), vec![]).is_err());
    assert!(crate::random_patch_apply::backup(key.clone(), vec![1]).is_ok());
    key.id = "another-plugin".into();
    assert!(crate::random_patch_apply::backup(key, vec![]).is_ok());
}
