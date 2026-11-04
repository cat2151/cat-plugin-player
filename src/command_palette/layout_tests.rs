use super::*;

fn draw(ctx: &egui::Context, size: egui::Vec2) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            ..Default::default()
        },
        |ctx| {
            let mut commands = vec![
                Command::new(
                    "Random patch",
                    "ランダムに音色を選択",
                    "音色",
                    "R",
                    &[Key::R],
                    true
                );
                23
            ];
            commands.push(Command::new(
                "Next velocity",
                "次のベロシティへ",
                "",
                "V",
                &[Key::V],
                true,
            ));
            let suspended = begin(ctx, commands, true);
            egui::CentralPanel::default().show(ctx, |_| {});
            end(ctx, suspended);
        },
    )
}

fn settle(ctx: &egui::Context, size: egui::Vec2, query: &str) -> egui::Rect {
    let mut palette = state(ctx);
    palette.open = true;
    palette.query = query.into();
    save(ctx, palette);
    for _ in 0..6 {
        draw(ctx, size);
    }
    ctx.memory(|m| m.area_rect(id(ctx)).unwrap())
}

#[test]
fn all_24_rows_fit_and_height_recovers_after_filtering_and_reopening() {
    let ctx = egui::Context::default();
    let size = egui::vec2(1200.0, 1000.0);
    let full = settle(&ctx, size, "");
    let query = ctx.read_response(id(&ctx).with("query")).unwrap().rect;
    // 24 normal rows plus their spacing must actually be allocated.
    assert!(full.height() > 24.0 * 20.0, "{full:?}");
    assert!(full.bottom() < size.y && full.top() < 25.0, "{full:?}");
    let output = draw(&ctx, size);
    let rows: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text.contains(" / ")
                    && (text.galley.job.text.starts_with("Random patch")
                        || text.galley.job.text.starts_with("Next velocity")) =>
            {
                Some((shape, text))
            }
            _ => None,
        })
        .collect();
    assert_eq!(rows.len(), 24);
    for (shape, text) in rows {
        let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
        assert!(
            shape.clip_rect.contains_rect(rect),
            "{rect:?} / {:?}",
            shape.clip_rect
        );
    }
    for _ in 0..3 {
        let one = settle(&ctx, size, "velocity");
        assert!(one.height() < full.height() / 2.0, "{one:?}");
        assert_eq!(one.min, full.min);
        assert_eq!(
            ctx.read_response(id(&ctx).with("query")).unwrap().rect,
            query
        );
        let empty = settle(&ctx, size, "no-such-command");
        assert!(empty.height() < full.height() / 2.0);
        assert_eq!(settle(&ctx, size, ""), full);
    }
    save(&ctx, State::default());
    assert_eq!(settle(&ctx, size, ""), full);
}

#[test]
fn list_fills_small_screen_and_recovers_when_screen_grows() {
    let ctx = egui::Context::default();
    let large = egui::vec2(1200.0, 1000.0);
    let full = settle(&ctx, large, "");
    for size in [egui::vec2(900.0, 500.0), egui::vec2(480.0, 320.0)] {
        let limited = settle(&ctx, size, "");
        assert!(limited.bottom() <= size.y, "{limited:?}");
        assert!(limited.bottom() >= size.y - 32.0, "{limited:?}");
        assert!(
            limited.left() >= 0.0 && limited.right() <= size.x,
            "{limited:?}"
        );
        settle(&ctx, size, "velocity");
        assert_eq!(settle(&ctx, size, ""), limited);
    }
    assert_eq!(settle(&ctx, large, ""), full);
}

#[test]
fn japanese_function_name_is_painted_and_searchable() {
    let ctx = egui::Context::default();
    let size = egui::vec2(1200.0, 1000.0);
    settle(&ctx, size, "次のベロシティ");
    let output = draw(&ctx, size);
    assert!(output.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Text(text)
            if text.galley.job.text == "Next velocity / 次のベロシティへ")
    }));
}
