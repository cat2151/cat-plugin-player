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
