use super::*;

fn key(key: egui::Key, repeat: bool, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers,
    }
}

#[test]
fn favorite_ignores_repeats_modifiers_and_disabled_actions() {
    for (repeat, modifiers, enabled, expected) in [
        (false, egui::Modifiers::NONE, true, true),
        (true, egui::Modifiers::NONE, true, false),
        (false, egui::Modifiers::CTRL, true, false),
        (false, egui::Modifiers::SHIFT, true, false),
        (false, egui::Modifiers::NONE, false, false),
    ] {
        let ctx = egui::Context::default();
        if repeat {
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![key(egui::Key::F, false, modifiers)],
                    ..Default::default()
                },
                |_| {},
            );
        }
        let _ = ctx.run(
            egui::RawInput {
                events: vec![key(egui::Key::F, repeat, modifiers)],
                ..Default::default()
            },
            |ctx| {
                assert_eq!(take_favorite_shortcut(ctx, enabled), expected);
                assert!(!take_favorite_shortcut(ctx, enabled));
            },
        );
    }
}

#[test]
#[ignore = "creates native host/audio device; no installed plugins required"]
fn native_main_slash_focuses_filter_without_inserting_slash() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect = Default::default();
    app.deferred_scan = false;
    let ctx = egui::Context::default();
    let input = |events| egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(900.0, 700.0),
        )),
        events,
        ..Default::default()
    };
    let _ = ctx.run(input(vec![]), |ctx| app.library_panel(ctx));
    let _ = ctx.run(
        input(vec![
            key(egui::Key::Slash, false, egui::Modifiers::NONE),
            egui::Event::Text("/".into()),
        ]),
        |ctx| app.library_panel(ctx),
    );
    assert_eq!(
        ctx.memory(|memory| memory.focused()),
        Some(egui::Id::new("library_filter"))
    );
    assert!(app.filter.is_empty());
    let _ = ctx.run(
        input(vec![
            key(egui::Key::F, false, egui::Modifiers::NONE),
            egui::Event::Text("f".into()),
        ]),
        |ctx| app.library_panel(ctx),
    );
    assert_eq!(app.filter, "f");
    assert!(app.favorites.library.entries.is_empty());
}
