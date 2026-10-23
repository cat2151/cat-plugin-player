use super::*;

fn key(key: Key, modifiers: egui::Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

/// A key held from an earlier frame is reported as a repeat, so each frame releases its keys.
fn released(mut events: Vec<Event>) -> Vec<Event> {
    let releases: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Key {
                key: k, modifiers, ..
            } => Some(Event::Key {
                key: *k,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: *modifiers,
            }),
            _ => None,
        })
        .collect();
    events.extend(releases);
    events
}

/// Runs one frame: the overlay first, then what a later handler still sees of Escape and Space.
fn frame(ctx: &egui::Context, events: Vec<Event>) -> (bool, bool) {
    let mut seen = (false, false);
    let _ = ctx.run(
        egui::RawInput {
            events: released(events),
            ..Default::default()
        },
        |ctx| {
            show(ctx, Page::Main);
            seen = ctx.input_mut(|input| {
                (
                    input.consume_key(egui::Modifiers::NONE, Key::Escape),
                    input.consume_key(egui::Modifiers::NONE, Key::Space),
                )
            });
        },
    );
    seen
}

#[test]
fn question_mark_and_f1_toggle_and_the_open_overlay_takes_every_key() {
    let ctx = egui::Context::default();
    let question = || {
        vec![
            key(Key::Questionmark, egui::Modifiers::SHIFT),
            Event::Text("?".into()),
        ]
    };
    assert_eq!(frame(&ctx, question()), (false, false));
    assert_eq!(
        frame(
            &ctx,
            vec![
                key(Key::Space, egui::Modifiers::NONE),
                key(Key::Escape, egui::Modifiers::NONE),
            ]
        ),
        (false, false),
        "open: Space is swallowed and Esc only closes the overlay"
    );
    assert_eq!(
        frame(&ctx, vec![key(Key::Space, egui::Modifiers::NONE)]),
        (false, true),
        "closed by Esc"
    );
    frame(&ctx, vec![key(Key::F1, egui::Modifiers::NONE)]);
    frame(&ctx, question());
    assert_eq!(
        frame(&ctx, vec![key(Key::Escape, egui::Modifiers::NONE)]),
        (true, false),
        "F1 opened and ? closed it"
    );
}

#[test]
fn question_mark_typed_into_a_text_field_is_left_alone() {
    let ctx = egui::Context::default();
    let mut text = String::new();
    for events in [
        vec![],
        vec![
            key(Key::Questionmark, egui::Modifiers::SHIFT),
            Event::Text("?".into()),
        ],
    ] {
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                show(ctx, Page::Main);
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.text_edit_singleline(&mut text).request_focus();
                });
            },
        );
    }
    assert_eq!(text, "?");
    assert!(
        frame(&ctx, vec![key(Key::Escape, egui::Modifiers::NONE)]).0,
        "the overlay stayed closed"
    );
}

fn menu_frame(ctx: &egui::Context, page: Page, events: Vec<Event>) -> (egui::FullOutput, bool) {
    use crate::main_menu::Menu;
    let mut action = false;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            events: released(events),
            ..Default::default()
        },
        |ctx| {
            show(ctx, page);
            crate::main_menu::keyboard(ctx, true);
            egui::CentralPanel::default().show(ctx, |ui| {
                for menu in [Menu::Settings, Menu::Analysis] {
                    menu.button(ui, |ui| {
                        ui.label("Menu contents");
                        action |= menu.take(ctx, Key::O) || menu.take(ctx, Key::P);
                    });
                }
            });
        },
    );
    (output, action)
}

fn has_text(output: &egui::FullOutput, expected: &str) -> bool {
    output.shapes.iter().any(|shape| {
        matches!(
            &shape.shape, egui::Shape::Text(text) if text.galley.job.text == expected
        )
    })
}

#[test]
fn main_and_smf_help_list_only_menu_entry_keys() {
    for page in [Page::Main, Page::Smf] {
        let ctx = egui::Context::default();
        menu_frame(&ctx, page, vec![key(Key::F1, egui::Modifiers::NONE)]);
        let output = menu_frame(&ctx, page, vec![]).0;
        assert!(has_text(&output, "Settings メニュー"));
        assert!(has_text(&output, "解析表示メニュー"));
        assert!(!has_text(&output, "お気に入りの並べ替え"));
        assert!(!has_text(&output, "解析ラベル"));
    }
}

#[test]
fn menu_help_is_local_and_escape_returns_to_the_same_menu() {
    use crate::main_menu::Menu;
    for (prefix, menu, toggle, own, other, action) in [
        (
            Key::D,
            Menu::Settings,
            Key::F1,
            "お気に入りの並べ替え",
            "解析ラベル",
            Key::O,
        ),
        (
            Key::A,
            Menu::Analysis,
            Key::Questionmark,
            "解析ラベル",
            "お気に入りの並べ替え",
            Key::P,
        ),
    ] {
        let ctx = egui::Context::default();
        menu_frame(&ctx, Page::Main, vec![key(prefix, egui::Modifiers::NONE)]);
        menu_frame(&ctx, Page::Main, vec![key(toggle, egui::Modifiers::NONE)]);
        let output = menu_frame(&ctx, Page::Main, vec![]).0;
        assert!(has_text(&output, own));
        assert!(!has_text(&output, other));
        assert!(!menu_frame(&ctx, Page::Main, vec![key(action, egui::Modifiers::NONE)]).1);
        menu_frame(
            &ctx,
            Page::Main,
            vec![key(Key::Escape, egui::Modifiers::NONE)],
        );
        assert!(Menu::current(&ctx) == Some(menu));
        assert!(menu_frame(&ctx, Page::Main, vec![key(action, egui::Modifiers::NONE)]).1);
        assert!(Menu::current(&ctx).is_none());
    }
}

#[test]
fn help_button_opens_the_menu_help() {
    let ctx = egui::Context::default();
    menu_frame(&ctx, Page::Main, vec![key(Key::D, egui::Modifiers::NONE)]);
    let output = menu_frame(&ctx, Page::Main, vec![]).0;
    let button = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Help (? / F1)" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .unwrap();
    for pressed in [true, false] {
        menu_frame(
            &ctx,
            Page::Main,
            vec![
                Event::PointerMoved(button),
                Event::PointerButton {
                    pos: button,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    let overlay_id = id(&ctx);
    assert!(
        ctx.data(|data| data.get_temp::<bool>(overlay_id))
            .unwrap_or(false),
        "Help button must open the overlay"
    );
    // A new egui Area measures itself on its first frame before painting.
    menu_frame(&ctx, Page::Main, vec![]);
    let output = menu_frame(&ctx, Page::Main, vec![]).0;
    assert!(has_text(&output, "お気に入りの並べ替え"));
    assert!(!has_text(&output, "解析ラベル"));
}
