use super::*;
use std::time::Duration;

#[test]
fn worker_keeps_ui_responsive_rejects_duplicates_and_allows_retry() {
    let mut rebuild = Rebuild::default();
    let ctx = egui::Context::default();
    let (release, wait) = mpsc::channel();
    rebuild.start(&ctx, move || {
        wait.recv().unwrap();
        Err("test failure".into())
    });
    assert!(rebuild.busy());
    assert!(rebuild.poll().is_none());
    rebuild.start(&ctx, || panic!("duplicate rebuild"));
    release.send(()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let result = loop {
        if let Some(result) = rebuild.poll() {
            break result;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert_eq!(result.unwrap_err(), "test failure");
    assert!(!rebuild.busy());
    rebuild.start(&ctx, || Ok(Vec::new()));
    assert!(rebuild.busy());
}

#[test]
fn process_failure_and_missing_executable_are_reported() {
    let exe = std::env::current_exe().unwrap();
    run(Command::new(&exe).arg("--list")).unwrap();
    let error = run(Command::new(exe).arg("--invalid-catalog-test-option")).unwrap_err();
    assert!(error.contains("cmrt failed"), "{error}");
    let error = run(&mut Command::new("X:/missing/cmrt.exe")).unwrap_err();
    assert!(error.contains("Could not run cmrt"), "{error}");
}

#[test]
fn native_rebuild_result_refreshes_candidates_and_failure_keeps_old_catalog() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.restoring = false;
    app.deferred_scan = false;
    let ctx = egui::Context::default();
    let candidate = crate::patch_browser_catalog::tests::candidate(
        "nakst.Apricot",
        "Apricot",
        "Test.synthpatch",
        None,
    );
    app.plugins = vec![crate::ffi::PluginInfo {
        index: 0,
        name: "Apricot".into(),
        vendor: String::new(),
        format: "CLAP".into(),
        id: candidate.plugin_id.clone(),
        bundle_path: candidate.bundle_path.to_string_lossy().into(),
        kind: crate::plugin_list::PluginKind::Instrument,
    }];
    let (tx, rx) = mpsc::channel();
    app.patch_catalog_rebuild.result = Some(rx);
    // Paint the actual Settings menu while a rebuild is pending, without launching CMRT.
    let _ = ctx.run(egui::RawInput::default(), |ctx| app.library_panel(ctx));
    let key = egui::Event::Key {
        key: egui::Key::D,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = ctx.run(
        egui::RawInput {
            events: vec![key],
            ..Default::default()
        },
        |ctx| {
            crate::main_menu::keyboard(ctx, true);
            app.library_panel(ctx);
        },
    );
    let output = ctx.run(egui::RawInput::default(), |ctx| app.library_panel(ctx));
    assert!(output.shapes.iter().any(|shape| matches!(
        &shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Rebuild patch catalog (R)"
    )));
    tx.send(Ok(vec![candidate])).unwrap();
    let generation = app.patch_catalog.generation;
    app.poll_patch_catalog_rebuild(&ctx);
    assert!(app.patch_catalog.generation > generation);
    assert_eq!(
        app.patch_catalog.candidates().next().unwrap().plugin_name,
        "Apricot"
    );
    assert_eq!(app.status, "Patch catalog rebuilt");
    let generation = app.patch_catalog.generation;
    let (tx, rx) = mpsc::channel();
    app.patch_catalog_rebuild.result = Some(rx);
    tx.send(Err("test failure".into())).unwrap();
    assert!(!app.can_rebuild_patch_catalog());
    app.poll_patch_catalog_rebuild(&ctx);
    assert_eq!(app.patch_catalog.generation, generation);
    assert_eq!(app.patch_catalog.candidates().count(), 1);
    assert!(app.status.contains("test failure"));
    assert!(app.can_rebuild_patch_catalog());
}
