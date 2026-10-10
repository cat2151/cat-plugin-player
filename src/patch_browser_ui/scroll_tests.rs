use super::*;

fn catalog(count: usize) -> Browser {
    let mut browser = Browser {
        open: true,
        snapshot: Some(
            Snapshot::build(
                (0..count)
                    .map(|i| candidate("six", "Six Sines", &format!("Patch {i:03}"), None))
                    .collect(),
                &[],
            )
            .unwrap(),
        ),
        ..Default::default()
    };
    browser.filter(false);
    browser
}

fn draw(browser: &mut Browser, ctx: &egui::Context) -> Vec<(String, egui::Pos2)> {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        },
        |ctx| browser.show(ctx, None, "test"),
    );
    output
        .shapes
        .into_iter()
        .filter_map(|shape| match shape.shape {
            egui::Shape::Text(text) if text.galley.text().starts_with("Patch ") => {
                Some((text.galley.text().to_owned(), text.pos))
            }
            _ => None,
        })
        .collect()
}

fn press(browser: &mut Browser, ctx: &egui::Context, keys: &[Key]) {
    let events = keys
        .iter()
        .map(|&key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .collect();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(900.0, 700.0),
        )),
        events,
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| browser.show(ctx, None, "test"));
}

#[test]
fn counts_and_pages_repeat_steps_and_enter_closes() {
    let ctx = egui::Context::default();
    let mut browser = catalog(200);
    draw(&mut browser, &ctx);
    let columns = browser.list_columns;
    browser.select(0);
    // One key per frame, as typed.
    let keys = |browser: &mut Browser, keys: &[Key]| {
        for &key in keys {
            press(browser, &ctx, &[key]);
        }
    };
    keys(&mut browser, &[Key::Num2, Key::J]);
    assert_eq!(browser.selected, Some(2 * columns));
    keys(&mut browser, &[Key::J]);
    assert_eq!(browser.selected, Some(3 * columns), "a count applies once");
    browser.select(0);
    keys(&mut browser, &[Key::Num1, Key::Num0, Key::ArrowDown]);
    assert_eq!(browser.selected, Some(10 * columns));
    browser.select(0);
    keys(&mut browser, &[Key::PageDown]);
    assert_eq!(browser.selected, Some(10 * columns));
    keys(&mut browser, &[Key::PageUp]);
    assert_eq!(browser.selected, Some(0));
    // Another key drops a pending count.
    keys(&mut browser, &[Key::Num3, Key::Space, Key::J]);
    assert_eq!(browser.selected, Some(columns));
    keys(&mut browser, &[Key::Enter]);
    assert!(!browser.open);
}

fn scroll(ctx: &egui::Context) -> (egui::Rect, f32) {
    ctx.data(|data| data.get_temp(egui::Id::new("patch_scroll")).unwrap())
}

#[test]
fn fitting_catalog_click_keeps_every_patch_in_place_on_each_frame() {
    let ctx = egui::Context::default();
    let mut browser = catalog(7);
    draw(&mut browser, &ctx);
    let before = draw(&mut browser, &ctx);
    assert_eq!(before.len(), 7);
    click_text(&mut browser, &ctx, "Patch 006");
    assert_eq!(browser.selected, Some(6));
    for _ in 0..3 {
        assert_eq!(draw(&mut browser, &ctx), before);
        assert_eq!(scroll(&ctx).1, 0.0);
    }
    // Random/programmatic selection and arrow navigation must also leave a fitting list still.
    browser.select(4);
    assert_eq!(draw(&mut browser, &ctx), before);
    frame(&mut browser, &ctx, vec![key(Key::ArrowRight)]);
    assert_eq!(browser.selected, Some(5));
    assert_eq!(draw(&mut browser, &ctx), before);
}

#[test]
fn long_catalog_visible_selection_preserves_position_and_last_row_is_valid_immediately() {
    let ctx = egui::Context::default();
    let mut browser = catalog(216);
    draw(&mut browser, &ctx);
    draw(&mut browser, &ctx);
    browser.select(100);
    let before = draw(&mut browser, &ctx);
    let offset = scroll(&ctx).1;
    assert!(offset > 0.0);
    click_text(&mut browser, &ctx, "Patch 100");
    assert_eq!(draw(&mut browser, &ctx), before);
    assert_eq!(scroll(&ctx).1, offset);
    browser.select(99);
    assert_eq!(draw(&mut browser, &ctx), before);
    assert_eq!(scroll(&ctx).1, offset);
    browser.select(215);
    let first = draw(&mut browser, &ctx);
    let (rect, _) = scroll(&ctx);
    let last = first
        .iter()
        .find(|(name, _)| name == "Patch 215")
        .unwrap()
        .1;
    assert!(
        last.y >= rect.top() && last.y < rect.bottom(),
        "{last:?} outside {rect:?}"
    );
    assert_eq!(
        draw(&mut browser, &ctx),
        first,
        "first reveal frame must already be clamped"
    );
    let previous_offset = scroll(&ctx).1;
    frame(
        &mut browser,
        &ctx,
        vec![
            egui::Event::PointerMoved(rect.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 120.0),
                modifiers: Default::default(),
            },
        ],
    );
    draw(&mut browser, &ctx);
    assert!(
        scroll(&ctx).1 < previous_offset,
        "manual scrolling must still work"
    );
    assert_eq!(browser.selected, Some(215));
    browser.query = "Patch.21[0-5]$".into();
    browser.filter(true);
    let filtered = draw(&mut browser, &ctx);
    assert_eq!(filtered.len(), 6);
    assert_eq!(scroll(&ctx).1, 0.0);
    assert_eq!(draw(&mut browser, &ctx), filtered);
}
