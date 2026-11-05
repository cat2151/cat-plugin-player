use super::*;

fn frame(
    ctx: &egui::Context,
    rows: &[(i32, String, bool)],
    enabled: bool,
    trigger: bool,
    events: Vec<Event>,
) -> Option<i32> {
    let mut selected = None;
    let _ = ctx.run(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ctx| {
            selected = show(ctx, rows, enabled, trigger);
        },
    );
    selected
}

fn key(key: Key, repeat: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers,
    }
}

#[test]
fn single_effect_is_direct_and_multiple_effects_follow_chain_order() {
    let ctx = egui::Context::default();
    let rows = [(42, "First".into(), false), (7, "Second".into(), true)];
    assert_eq!(frame(&ctx, &[], true, true, vec![]), None);
    assert!(!is_open(&ctx));
    assert_eq!(frame(&ctx, &rows[..1], true, true, vec![]), Some(42));
    assert!(!is_open(&ctx));
    assert_eq!(frame(&ctx, &rows, true, true, vec![]), None);
    assert!(is_open(&ctx));
    assert_eq!(
        frame(
            &ctx,
            &rows,
            true,
            false,
            vec![key(Key::B, false, Modifiers::NONE)]
        ),
        Some(7)
    );
    assert!(!is_open(&ctx));
    let reordered = [rows[1].clone(), rows[0].clone()];
    frame(&ctx, &reordered, true, true, vec![]);
    assert_eq!(
        frame(
            &ctx,
            &reordered,
            true,
            false,
            vec![key(Key::A, false, Modifiers::NONE)]
        ),
        Some(7)
    );
}

#[test]
fn picker_isolates_keys_and_cancels_without_an_action() {
    let rows = [(42, "First".into(), false), (7, "Second".into(), true)];
    for event in [
        key(Key::A, true, Modifiers::NONE),
        key(Key::A, false, Modifiers::CTRL),
        key(Key::Z, false, Modifiers::NONE),
    ] {
        let ctx = egui::Context::default();
        let warmup = if matches!(event, Event::Key { repeat: true, .. }) {
            vec![key(Key::A, false, Modifiers::NONE)]
        } else {
            vec![]
        };
        frame(&ctx, &rows, true, true, warmup);
        assert_eq!(frame(&ctx, &rows, true, false, vec![event]), None);
        assert!(is_open(&ctx));
        assert!(ctx.input(|i| i.events.is_empty()));
        assert_eq!(
            frame(
                &ctx,
                &rows,
                true,
                false,
                vec![key(Key::Escape, false, Modifiers::NONE)]
            ),
            None
        );
        assert!(!is_open(&ctx));
        assert!(ctx.input(|i| i.events.is_empty()));
    }
    let ctx = egui::Context::default();
    frame(&ctx, &rows, true, true, vec![]);
    assert_eq!(
        frame(
            &ctx,
            &rows,
            false,
            false,
            vec![key(Key::A, false, Modifiers::NONE)]
        ),
        None
    );
    assert!(!is_open(&ctx));
}
