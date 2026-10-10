//! Controlled preparation completions through the actual browser dispatcher.
use super::*;

#[test]
#[ignore = "requires compiled CLAP fixture and isolated S06_NATIVE_WORK_DIR"]
fn native_browser_latest_worker_close_heavy_and_background_keys() {
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    let mut app = app(&root.join("browser-worker/config.toml"));
    app.mml_input = Default::default();
    let normal = key("normal");
    let index = add(&mut app, &normal);
    app.load_plugin(index);
    wait(&mut app);
    let id = app.instrument_id().unwrap();
    app.host.load_state(id, &[4]).unwrap();
    let ctx = eframe::egui::Context::default();
    // Closed prevents the dispatcher starting the next fixture via the real catalog API;
    // tests explicitly supply controlled workers for fixture-only plugin identities.
    let mut a = prepared(&normal, &[5]).candidate;
    a.display = "A".into();
    let mut b = a.clone();
    b.display = "B".into();
    let mut c = a.clone();
    c.display = "C".into();
    app.patch_browser.requests.select(a.clone(), 1);
    app.patch_browser.requests.next().unwrap();
    let (release, wait_worker) = mpsc::channel();
    let (ready, completed) = mpsc::channel();
    app.patch_browser
        .preparation
        .start_with(a, ctx.clone(), move |_| {
            wait_worker.recv().unwrap();
            ready.send(()).unwrap();
            Ok(vec![5])
        });
    app.patch_browser.requests.select(b, 1);
    app.patch_browser.requests.select(c.clone(), 1);
    assert!(app.patch_browser.requests.next().is_none());
    release.send(()).unwrap();
    completed.recv_timeout(Duration::from_secs(5)).unwrap();
    drain(&mut app, &ctx);
    assert_eq!(
        app.host.save_state(id).unwrap(),
        [4],
        "prepared A never reaches live after C selected"
    );
    assert_eq!(
        app.patch_browser.requests.next().unwrap().candidate.display,
        "C"
    );
    app.patch_browser
        .preparation
        .start_with(c.clone(), ctx.clone(), |_| Ok(vec![6]));
    drain(&mut app, &ctx);
    wait(&mut app);
    assert_eq!(app.host.save_state(id).unwrap(), [6]);
    assert_eq!(
        app.patch_browser.requests.applied.as_ref().unwrap().display,
        "C"
    );
    assert!(app.status.contains("Browse patch:"));
    c.display = "Late".into();
    app.patch_browser.requests.select(c.clone(), 1);
    app.patch_browser.requests.next().unwrap();
    app.patch_browser
        .preparation
        .start_with(c.clone(), ctx.clone(), |_| Ok(vec![7]));
    app.patch_browser.close();
    drain(&mut app, &ctx);
    assert_eq!(app.host.save_state(id).unwrap(), [6]);
    c.display = "Normal-before-heavy".into();
    app.patch_browser.requests.select(c.clone(), 1);
    app.patch_browser.requests.next().unwrap();
    app.patch_browser
        .preparation
        .start_with(c.clone(), ctx.clone(), |_| Ok(vec![8]));
    c.display = "Heavy".into();
    c.measurement.sfz_sample_bytes = Some(64_000_000);
    app.patch_browser.requests.select(c, 1);
    app.patch_browser.requests.enter();
    drain(&mut app, &ctx);
    assert_eq!(
        app.host.save_state(id).unwrap(),
        [6],
        "normal completion cannot apply under an unconfirmed heavy selection"
    );
    assert!(app.patch_browser.requests.next().is_none());
    assert!(!app.patch_browser.preparation.busy());
    app.patch_browser.open = true;
    app.sequence_pattern = SequencePattern::Steps;
    let _ = ctx.run(
        eframe::egui::RawInput {
            events: vec![
                eframe::egui::Event::Key {
                    key: eframe::egui::Key::I,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                },
                eframe::egui::Event::Key {
                    key: eframe::egui::Key::Space,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                },
            ],
            ..Default::default()
        },
        |ctx| {
            app.mml_editor(ctx);
            eframe::egui::CentralPanel::default().show(ctx, |ui| app.sequence_controls(ui));
        },
    );
    assert!(!app.mml_input.open);
    assert_eq!(app.sequence_pattern, SequencePattern::Steps);
    app.open_mml_editor();
    assert!(!app.mml_input.open);
}

fn drain(app: &mut App, ctx: &eframe::egui::Context) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.patch_browser.preparation.busy() {
        assert!(Instant::now() < deadline);
        app.poll_browser_patch(ctx);
        std::thread::yield_now();
    }
}

#[test]
#[ignore = "requires compiled CLAP fixture and isolated S06_NATIVE_WORK_DIR"]
fn native_browser_child_viewport_focus_poll_close_and_reopen() {
    use eframe::egui;
    use std::{cell::RefCell, rc::Rc};
    let _library = crate::native_library::load().unwrap();
    let root = PathBuf::from(std::env::var_os("S06_NATIVE_WORK_DIR").unwrap());
    let mut app = app(&root.join("browser-viewport/config.toml"));
    let normal = key("normal");
    let index = add(&mut app, &normal);
    app.load_plugin(index);
    wait(&mut app);
    let id = app.instrument_id().unwrap();
    app.host.load_state(id, &[4]).unwrap();
    let ctx = egui::Context::default();
    ctx.set_embed_viewports(false);
    let closing = Rc::new(RefCell::new(false));
    let builders = Rc::new(RefCell::new(Vec::new()));
    egui::Context::set_immediate_viewport_renderer({
        let closing = closing.clone();
        let builders = builders.clone();
        move |ctx, mut viewport| {
            assert_ne!(viewport.ids.this, egui::ViewportId::ROOT);
            assert_eq!(viewport.ids.parent, egui::ViewportId::ROOT);
            builders.borrow_mut().push(viewport.builder.clone());
            let mut input = egui::RawInput {
                viewport_id: viewport.ids.this,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1920.0, 1080.0),
                )),
                ..Default::default()
            };
            let info = input.viewports.entry(viewport.ids.this).or_default();
            info.parent = Some(egui::ViewportId::ROOT);
            info.focused = Some(true);
            if *closing.borrow() {
                info.events.push(egui::ViewportEvent::Close);
            }
            let _ = ctx.run(input, |ctx| (viewport.viewport_ui_cb)(ctx));
        }
    });
    let frame = |app: &mut App| {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        };
        let info = input.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
        info.outer_rect = Some(egui::Rect::from_min_size(
            egui::pos2(120.0, 140.0),
            egui::vec2(900.0, 700.0),
        ));
        info.focused = Some(false);
        let _ = ctx.run(input, |ctx| {
            app.browser_ui(ctx);
            assert_eq!(ctx.viewport_id(), egui::ViewportId::ROOT);
            assert!(!ctx.input(|input| input.viewport().close_requested()));
            app.poll_browser_patch(ctx);
            app.mml_editor(ctx);
            egui::CentralPanel::default().show(ctx, |ui| app.sequence_controls(ui));
        });
    };
    app.patch_browser.open = true;
    let mut patch = prepared(&normal, &[6]).candidate;
    patch.display = "Child focus".into();
    app.patch_browser.requests.select(patch.clone(), 0);
    app.patch_browser.requests.next().unwrap();
    let (release, gate) = mpsc::channel();
    let (ready, completed) = mpsc::channel();
    app.patch_browser
        .preparation
        .start_with(patch.clone(), ctx.clone(), move |_| {
            gate.recv().unwrap();
            ready.send(()).unwrap();
            Ok(vec![6])
        });
    let geometry = app.window_config.clone();
    frame(&mut app);
    frame(&mut app);
    assert_eq!(builders.borrow()[0].maximized, Some(true));
    assert_eq!(
        builders.borrow()[0].position,
        Some(egui::pos2(120.0, 140.0))
    );
    release.send(()).unwrap();
    completed.recv_timeout(Duration::from_secs(5)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.patch_browser.preparation.busy() {
        assert!(Instant::now() < deadline);
        frame(&mut app);
    }
    wait(&mut app);
    assert_eq!(app.host.save_state(id).unwrap(), [6]);
    assert_eq!(
        app.patch_browser.requests.applied.as_ref().unwrap().display,
        "Child focus"
    );
    patch.display = "Late child close".into();
    app.patch_browser.requests.select(patch.clone(), 0);
    app.patch_browser.requests.next().unwrap();
    app.patch_browser
        .preparation
        .start_with(patch, ctx.clone(), |_| Ok(vec![7]));
    *closing.borrow_mut() = true;
    frame(&mut app);
    assert!(!app.patch_browser.open);
    drain(&mut app, &ctx);
    assert_eq!(app.host.save_state(id).unwrap(), [6]);
    assert_eq!(app.window_config.main, geometry.main);
    *closing.borrow_mut() = false;
    app.patch_browser.open = true;
    let before = builders.borrow().len();
    frame(&mut app);
    assert!(app.patch_browser.open && builders.borrow().len() > before);
    assert!(app.patch_browser.requests.desired.is_none());
    assert_eq!(app.instrument_id(), Some(id));
    app.patch_browser.close();
}
