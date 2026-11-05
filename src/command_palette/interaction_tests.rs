use super::tests::{commands, key, open};
use super::*;

fn draw(ctx: &egui::Context, events: Vec<Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            let input = begin(ctx, commands(), true);
            egui::CentralPanel::default().show(ctx, |_| {});
            if input.is_none() {
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::V));
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::R));
            }
            end(ctx, input);
        },
    )
}

#[test]
fn clicking_only_selects_then_enter_runs_that_row_and_disabled_rows_are_not_painted() {
    let ctx = egui::Context::default();
    open(&ctx);
    let output = draw(&ctx, vec![]);
    let labels: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(!labels
        .iter()
        .any(|t| t.galley.job.text.contains("Disabled action")));
    assert!(labels.iter().any(|t| t.galley.job.text == "1 / 2"));
    assert!(!labels
        .iter()
        .any(|t| t.galley.job.text.contains("Enter / click")));
    let target = labels
        .iter()
        .find(|t| t.galley.job.text.contains("Next velocity"))
        .unwrap();
    let pos = target.pos + egui::vec2(12.0, 6.0);
    for pressed in [true, false] {
        draw(
            &ctx,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    assert!(state(&ctx).open);
    assert_eq!(state(&ctx).pending, None);
    assert_eq!(state(&ctx).selected, 1);
    draw(&ctx, vec![key(Key::Enter, Modifiers::NONE)]);
    assert!(!state(&ctx).open);
    assert_eq!(state(&ctx).pending, Some("Next velocity"));
    assert!(history::load(&ctx).rank("", "Main/next-velocity") == (2, 0));
    draw(&ctx, vec![]);
    assert_eq!(history::load(&ctx).rank("", "Main/next-velocity"), (0, 0));
}

#[test]
fn shortcut_uses_theme_color_monospace_and_sits_beside_label_in_both_themes() {
    for visuals in [egui::Visuals::dark(), egui::Visuals::light()] {
        let ctx = egui::Context::default();
        let mut visuals = visuals;
        let key_fill = egui::Color32::from_rgb(11, 22, 33);
        visuals.extreme_bg_color = key_fill;
        let key_color = visuals.text_color();
        ctx.set_theme(if visuals.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        ctx.set_visuals(visuals);
        ctx.style_mut(|s| s.animation_time = 0.0);
        open(&ctx);
        let output = draw(&ctx, vec![]);
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        let label = texts
            .iter()
            .find(|t| t.galley.job.text.contains("Random patch"))
            .unwrap();
        let shortcut = texts.iter().find(|t| t.galley.job.text == "R").unwrap();
        assert_eq!(
            shortcut.galley.job.sections[0].format.font_id.family,
            egui::FontFamily::Monospace
        );
        assert_eq!(shortcut.galley.job.sections[0].format.color, key_color);
        // Six pixels of key padding follow the reference's eight-pixel gap.
        assert!((shortcut.pos.x - label.pos.x - label.galley.size().x - 14.0).abs() < 1.0);
        assert!(output.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.fill == key_fill)));
    }
}

#[test]
fn a_palette_in_another_viewport_blocks_background_keys_and_a_second_palette() {
    let ctx = egui::Context::default();
    let child = egui::ViewportId::from_hash_of("child");
    claim_owner(&ctx, child);
    assert!(super::tests::frame(&ctx, vec![key(Key::V, Modifiers::NONE)], true).is_empty());
    assert!(super::tests::frame(
        &ctx,
        vec![key(Key::P, Modifiers::CTRL | Modifiers::SHIFT)],
        true
    )
    .is_empty());
    assert!(!state(&ctx).open);
    assert_eq!(owner(&ctx), Some(child));
}
