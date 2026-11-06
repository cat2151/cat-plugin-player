use super::*;

#[test]
fn resident_kit_browser_metadata_confirm_ack_and_cancel_keep_target_identity() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut receiver)) = app("resident kit");
    let open = Open::Kit {
        track: 1,
        layer: Some(9),
        song: 2,
        request: 3,
    };
    request(&mut app, &mut editor, open);
    assert!(app.drum_browser.open && !app.patch_browser.open);
    assert!(app.drum_browser.focus_window);
    let candidate = crate::drum_browser::tests::candidate("kit");
    app.plugins = vec![plugin("kit", crate::plugin_list::PluginKind::Instrument)];
    app.resident.as_mut().unwrap().preparation_target = Some(open);
    app.drum_browser.kits = vec![candidate.clone()];
    app.drum_browser.results = vec![0];
    app.drum_browser.selected = Some(0);
    app.patch_browser.requests.applied = Some(candidate.clone());
    app.drum_browser.prepared = Some(crate::patch_preparation::Prepared {
        candidate,
        state: vec![1, 2, 3],
    });
    let ctx = egui::Context::default();
    app.drum_browser.editing = Some(app.drum_browser.saved.query.clone());
    let mut ime = vec![egui::Event::Ime(egui::ImeEvent::Preedit("あ".into()))];
    ime.extend(drum_keys(egui::Key::Enter));
    drum_frame(&mut app, &ctx, ime);
    assert!(app.drum_browser.editing.is_some() && app.drum_browser.open);
    assert!(!app.resident_waiting());
    drum_frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
    );
    drum_frame(&mut app, &ctx, drum_keys(egui::Key::Escape));
    assert!(app.drum_browser.open && app.drum_browser.editing.is_none());
    drum_frame(&mut app, &ctx, drum_keys(egui::Key::Enter));
    assert!(app.resident_waiting());
    let (offered, bytes) = selected(&mut receiver);
    assert_eq!(offered.target, open);
    assert_eq!(bytes, [1, 2, 3]);
    let kit = offered.kit.unwrap();
    assert_eq!(kit.layer, 9);
    assert_eq!(
        kit.notes.iter().map(|n| n.note).collect::<Vec<_>>(),
        [0, 127]
    );
    assert_eq!(kit.notes[0].name, "Original name");
    assert_eq!(kit.notes[1].name, "");
    answer(&mut app, &mut editor, Ok(()));
    assert!(!app.drum_browser.open);
    assert_eq!(
        receiver.receive::<ToEditor>().unwrap(),
        Some(ToEditor::KitClosed { target: open })
    );
    let next = Open::Kit {
        track: 1,
        layer: Some(9),
        song: 2,
        request: 4,
    };
    request(&mut app, &mut editor, next);
    assert!(app.drum_browser.prepared.is_none());
    assert!(app.patch_browser.requests.applied.is_some());
    app.confirm_resident_kit();
    assert!(app.drum_browser.confirm && app.drum_browser.awaiting.is_some());
    assert!(app.patch_browser.requests.desired.is_some());
    assert!(!app.resident_waiting());
    assert!(app.patch_browser.requests.next().is_some());
    assert!(app.patch_browser.requests.ready().is_some());
    app.patch_browser.requests.finish(false);
    app.drum_browser.prepared = Some(crate::patch_preparation::Prepared {
        candidate: app.drum_browser.kits[0].clone(),
        state: vec![4, 5, 6],
    });
    app.drum_patch_finished(false);
    assert!(app.drum_browser.prepared.is_none() && !app.drum_browser.confirm);
    assert!(app.drum_browser.awaiting.is_none());
    assert!(!app.resident_waiting());
    drum_frame(&mut app, &ctx, drum_keys(egui::Key::Escape));
    assert_eq!(
        receiver.receive::<ToEditor>().unwrap(),
        Some(ToEditor::KitClosed { target: next })
    );
    assert!(!app.resident_waiting());
}

fn drum_keys(key: egui::Key) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .collect()
}
fn drum_frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
    let _ = ctx.run(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ctx| app.draw_drum_browser(ctx),
    );
}
