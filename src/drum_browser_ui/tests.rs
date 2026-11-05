use super::*;

fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(380.0, 330.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            if app.drum_browser.editing.is_none() {
                crate::help_overlay::show(ctx, crate::help_overlay::Page::Drum);
            }
            app.draw_drum_browser(ctx);
        },
    );
}

#[test]
fn native_drum_two_panes_keys_search_help_and_heavy_gate() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    app.drum_browser.kits = (0..25)
        .map(|n| {
            let mut c = crate::drum_browser::tests::candidate(&format!("plugin{n}"));
            c.measurement.sfz_sample_bytes = Some(64_000_000);
            c.measurement.drum_kit_notes = Some((0..=127).collect());
            c
        })
        .collect();
    app.drum_browser.filter();
    app.drum_browser.open = true;
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![key(Key::PageDown)]);
    assert_eq!(app.drum_browser.selected, Some(10));
    assert!(app.patch_browser.requests.desired.is_none());
    frame(&mut app, &ctx, vec![key(Key::L)]);
    assert_eq!(app.drum_browser.saved.focus, Focus::Notes);
    frame(&mut app, &ctx, vec![key(Key::PageDown)]);
    assert_eq!(
        app.drum_browser
            .saved
            .note(app.drum_browser.selected().unwrap()),
        Some(10)
    );
    assert!(app.patch_browser.requests.desired.is_none());
    frame(&mut app, &ctx, vec![key(Key::End)]);
    assert_eq!(
        app.drum_browser
            .saved
            .note(app.drum_browser.selected().unwrap()),
        Some(127)
    );
    frame(&mut app, &ctx, vec![key(Key::Tab)]);
    assert_eq!(app.drum_browser.saved.focus, Focus::Notes);
    frame(&mut app, &ctx, vec![key(Key::Space)]);
    assert!(app.patch_browser.requests.desired.is_some());
    assert!(app.patch_browser.requests.confirmation.is_none());
    frame(&mut app, &ctx, vec![key(Key::P), key(Key::ArrowLeft)]);
    assert!(app.drum_browser.awaiting.is_none());
    frame(&mut app, &ctx, vec![key(Key::ArrowLeft)]);
    assert_eq!(app.drum_browser.saved.focus, Focus::Kits);
    frame(&mut app, &ctx, vec![key(Key::V)]);
    assert!(app.drum_browser.compare_velocity);
    assert!(app.patch_browser.requests.desired.is_none());
    frame(&mut app, &ctx, vec![key(Key::Slash)]);
    frame(&mut app, &ctx, vec![egui::Event::Text("hlGP".into())]);
    assert!(app.drum_browser.open);
    assert_eq!(app.drum_browser.saved.focus, Focus::Kits);
    assert_eq!(app.drum_browser.saved.query, "hlGP");
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("編集中".into()))],
    );
    frame(&mut app, &ctx, vec![key(Key::Enter)]);
    assert!(app.drum_browser.editing.is_some());
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
    );

    frame(&mut app, &ctx, vec![key(Key::Escape)]);
    assert!(app.drum_browser.open);
    assert_eq!(app.drum_browser.saved.query, "");
    assert!(app.drum_browser.editing.is_none());
    frame(&mut app, &ctx, vec![key(Key::F1)]);
    let before = app.drum_browser.selected;
    frame(
        &mut app,
        &ctx,
        vec![key(Key::J), key(Key::L), key(Key::Space)],
    );
    assert_eq!(app.drum_browser.selected, before);
    assert_eq!(app.drum_browser.saved.focus, Focus::Kits);
    assert!(app.patch_browser.requests.desired.is_none());
    frame(&mut app, &ctx, vec![key(Key::Escape)]);
    assert!(app.drum_browser.open);
    let panes = ctx
        .data(|d| d.get_temp::<[egui::Rect; 2]>(egui::Id::new("drum_panes")))
        .unwrap();
    assert!(panes[0].max.x < panes[1].min.x);
    assert!(panes.iter().all(|p| p.width() > 0.0 && p.height() > 0.0));
    frame(&mut app, &ctx, vec![key(Key::Escape)]);
    assert!(!app.drum_browser.open);
}

#[test]
fn native_drum_open_starts_light_audition_but_heavy_waits_and_panes_favor_kit_names() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.pending = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    app.drum_browser.generation = Some(app.patch_catalog.generation);
    app.drum_browser.kits = vec![crate::drum_browser::tests::candidate("light")];
    app.drum_browser.filter();
    app.open_drum_browser();
    assert!(app.drum_browser.awaiting.is_some());
    assert!(app.drum_browser.loading_since.is_some());
    assert!(app.patch_browser.requests.desired.is_some());
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    let panes = ctx
        .data(|d| d.get_temp::<[egui::Rect; 2]>(egui::Id::new("drum_panes")))
        .unwrap();
    assert!(panes[0].width() > panes[1].width() * 1.7);
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let left = app.drum_left_width(ui, 1200.0);
            let wider = app.drum_left_width(ui, 1500.0);
            assert_eq!(
                left, wider,
                "extra window width must not stretch the kit pane"
            );
            assert!(left < 1200.0 * 0.8);
        });
    });
    app.close_drum_browser();
    assert!(app.drum_browser.loading_since.is_none());
    app.drum_browser.kits[0].measurement.sfz_sample_bytes = Some(64_000_000);
    app.open_drum_browser();
    assert!(app.drum_browser.awaiting.is_none());
    assert!(app.patch_browser.requests.desired.is_none());
    assert!(!app.drum_browser.opening_audition);
    assert_eq!(app.sequence_pattern, crate::SequencePattern::Off);
    assert_eq!(super::feedback::pitch(70), "A#4");
    assert_eq!(
        super::feedback::loading_color(0.0),
        super::feedback::loading_color(0.99)
    );
    assert_ne!(
        super::feedback::loading_color(1.1),
        super::feedback::loading_color(2.1)
    );
}

#[test]
fn native_drum_main_controls_exclude_melodic_transport_and_restore_for_normal_patch() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect.clear();
    app.deferred_scan = false;
    let kit = crate::drum_browser::tests::candidate("kit");
    app.patch_browser.requests.applied = Some(kit.clone());
    app.sequence_pattern = crate::SequencePattern::Off;
    assert!(app.drum_kit_active());
    let ctx = egui::Context::default();
    let _ = ctx.run(
        egui::RawInput {
            events: vec![key(Key::Space), key(Key::S)],
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.sequence_controls(ui));
        },
    );
    assert_eq!(app.sequence_pattern, crate::SequencePattern::Off);
    app.patch_browser
        .requests
        .applied
        .as_mut()
        .unwrap()
        .measurement
        .drum_kit = false;
    assert!(!app.drum_kit_active());
    app.patch_browser.requests.applied = None;
    app.drum_browser.saved.applied = Some(identity(&kit));
    app.patch_browser.restore_playing = Some(identity(&kit));
    assert!(app.drum_kit_active());
    app.invalidate_browser_live();
    assert!(!app.drum_kit_active());
}
