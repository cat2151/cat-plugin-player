use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

fn candidate() -> Candidate {
    Candidate {
        format: "CLAP".into(),
        plugin_id: SURGE_ID.into(),
        plugin_name: "Surge XT".into(),
        display: "Bass/One.fxp".into(),
        path: "X:/patches/Bass/One.fxp".into(),
    }
}

fn plugin(format: &str, id: &str) -> PluginInfo {
    PluginInfo {
        index: 27,
        name: "Surge XT".into(),
        vendor: String::new(),
        format: format.into(),
        id: id.into(),
        bundle_path: String::new(),
        kind: crate::plugin_list::PluginKind::Instrument,
    }
}

fn poll(catalog: &mut Catalog) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(message) = catalog.update_with(false, || panic!("second load"), || {}) {
            return message;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn callback_gate_nonblocking_completion_repaint_and_one_shot() {
    let mut catalog = Catalog::default();
    catalog.update_with(false, || panic!("before callback"), || {});
    assert!(matches!(catalog.state, State::NotStarted));
    let (release, wait) = mpsc::channel();
    let (repainted, repaint) = mpsc::channel();
    let count = Arc::new(AtomicUsize::new(0));
    let worker_count = count.clone();
    assert!(catalog
        .update_with(
            true,
            move || {
                worker_count.fetch_add(1, Ordering::SeqCst);
                wait.recv().unwrap();
                Ok(vec![candidate()])
            },
            move || {
                repainted.send(()).unwrap();
            }
        )
        .is_none());
    // The loader remains blocked; a later GUI frame can still return immediately.
    assert!(catalog
        .update_with(true, || panic!("second load"), || {})
        .is_none());
    assert!(catalog.eligible.is_empty());
    release.send(()).unwrap();
    repaint.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(poll(&mut catalog).is_empty());
    catalog.update_with(false, || panic!("after stop"), || {});
    catalog.update_with(true, || panic!("after replacement"), || {});
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn scan_and_read_orders_use_format_and_id_and_reconcile_rescan() {
    for scan_first in [true, false] {
        let mut catalog = Catalog::default();
        let installed = [plugin("CLAP", SURGE_ID)];
        if scan_first {
            catalog.reconcile(&installed);
        }
        catalog.update_with(true, || Ok(vec![candidate()]), || {});
        poll(&mut catalog);
        catalog.reconcile(&installed);
        assert_eq!(catalog.eligible, [0]);
        for wrong in [plugin("VST3", SURGE_ID), plugin("CLAP", "other-id")] {
            catalog.reconcile(&[wrong]);
            assert!(catalog.eligible.is_empty());
        }
        catalog.reconcile(&installed);
        assert_eq!(catalog.eligible, [0]);
        catalog.reconcile(&[]);
        assert!(catalog.eligible.is_empty());
    }
}

#[test]
fn unavailable_empty_and_missing_target_stay_hidden_without_retry() {
    for error in ["missing", "invalid JSON", "unsupported version"] {
        let mut catalog = Catalog::default();
        catalog.update_with(true, move || Err(error.into()), || {});
        assert!(poll(&mut catalog).contains(error));
        catalog.reconcile(&[plugin("CLAP", SURGE_ID)]);
        assert!(catalog.eligible.is_empty());
        catalog.update_with(true, || panic!("retry"), || {});
    }
    let mut catalog = Catalog::default();
    catalog.update_with(true, || Ok(Vec::new()), || {});
    poll(&mut catalog);
    catalog.reconcile(&[plugin("CLAP", SURGE_ID)]);
    assert!(catalog.eligible.is_empty());
}

#[test]
fn conditional_ui_disables_each_busy_phase() {
    let ctx = egui::Context::default();
    let mut catalog = Catalog::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            assert!(catalog.button_response(ui, false).is_none());
        });
    });
    catalog.state = State::Ready(vec![candidate()]);
    catalog.reconcile(&[plugin("CLAP", SURGE_ID)]);
    for flags in [
        (false, false, false, false),
        (true, false, false, false),
        (false, true, false, false),
        (false, false, true, false),
        (false, false, false, true),
    ] {
        let blocked = busy(flags.0, flags.1, flags.2, flags.3);
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert_eq!(
                    catalog.button_response(ui, blocked).unwrap().enabled(),
                    !blocked
                );
            });
        });
    }
}

#[test]
#[ignore = "read-only installed catalog acceptance"]
fn installed_catalog_extracts_owned_surge_clap_paths() {
    let candidates = load().unwrap();
    assert!(!candidates.is_empty());
    for candidate in &candidates {
        assert_eq!(candidate.format, "CLAP");
        assert_eq!(candidate.plugin_id, SURGE_ID);
        assert!(candidate.path.is_absolute());
        assert!(!candidate.display.is_empty());
        assert!(!candidate.plugin_name.is_empty());
    }
    println!("owned Surge CLAP candidates: {}", candidates.len());
}
