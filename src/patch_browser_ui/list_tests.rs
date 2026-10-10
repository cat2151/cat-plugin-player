use super::*;

#[test]
#[ignore = "read-only installed catalog and grid painting performance"]
fn installed_grid_virtualization_and_frame_timings() {
    let candidates = crate::random_patch_catalog::load().unwrap();
    let count = candidates.len();
    let mut browser = Browser {
        open: true,
        snapshot: Some(Snapshot::build(candidates, &[]).unwrap()),
        ..Default::default()
    };
    browser.filter(false);
    let labels = browser.row_labels.as_ptr();
    let snapshot = browser.snapshot.as_ref().unwrap() as *const Snapshot;
    for dimensions in [egui::vec2(1920.0, 1080.0), egui::vec2(360.0, 600.0)] {
        let ctx = egui::Context::default();
        let started = std::time::Instant::now();
        for _ in 0..10 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, dimensions)),
                    ..Default::default()
                },
                |ctx| browser.show(ctx, None, "idle"),
            );
        }
        let painted = ctx.data(|data| {
            data.get_temp::<std::ops::Range<usize>>(egui::Id::new("painted_patch_rows"))
                .unwrap()
        });
        assert!(!painted.is_empty() && painted.len() < count);
        assert_eq!(browser.list_columns == 1, dimensions.x < 400.0);
        assert_eq!(browser.row_labels.as_ptr(), labels);
        assert_eq!(
            browser.snapshot.as_ref().unwrap() as *const Snapshot,
            snapshot
        );
        assert!(browser.requests.desired.is_none());
        println!("{count} patches / {dimensions:?}: {} columns, {} visible cells, 10 frames {:?}; cached snapshot/names unchanged", browser.list_columns, painted.len(), started.elapsed());
    }
}

#[test]
fn filtering_scrolls_preserved_selection_to_its_new_position_without_reloading() {
    let ctx = egui::Context::default();
    let candidates = (0..1024)
        .map(|i| candidate("six", "Six Sines", &format!("Bank/Patch {i:03}"), None))
        .collect();
    let mut browser = Browser {
        open: true,
        snapshot: Some(Snapshot::build(candidates, &[]).unwrap()),
        ..Default::default()
    };
    browser.filter(false);
    browser.select(500);
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    let serial = browser.requests.selected.as_ref().unwrap().serial;
    browser.query = "-Patch.0".into();
    browser.filter(true);
    assert_eq!(browser.selected, Some(500));
    assert_eq!(browser.requests.selected.as_ref().unwrap().serial, serial);
    let position = browser
        .results
        .iter()
        .position(|&index| index == 500)
        .unwrap();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    assert!(ctx
        .data(|data| data
            .get_temp::<std::ops::Range<usize>>(egui::Id::new("painted_patch_rows"))
            .unwrap())
        .contains(&position));
}

#[test]
fn six_sines_sized_list_virtualizes_fixed_rows_and_scroll_keeps_identity() {
    let ctx = egui::Context::default();
    let candidates: Vec<_> = (0..216)
        .map(|index| candidate("six", "Six Sines", &format!("Bass/Patch {index:03}"), None))
        .collect();
    let mut browser = Browser {
        open: true,
        snapshot: Some(Snapshot::build(candidates, &[]).unwrap()),
        ..Default::default()
    };
    browser.filter(false);
    let draw = |browser: &mut Browser, dimensions, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, dimensions)),
                events,
                ..Default::default()
            },
            |ctx| browser.show(ctx, None, "test"),
        )
    };
    let wide = egui::vec2(1920.0, 600.0);
    draw(&mut browser, wide, vec![]);
    let output = draw(&mut browser, wide, vec![]);
    let columns = browser.list_columns;
    assert!(columns > 1);
    let range = ctx.data(|data| {
        data.get_temp::<std::ops::Range<usize>>(egui::Id::new("painted_patch_rows"))
            .unwrap()
    });
    assert!(
        range.len() > 100 && range.len() < 216,
        "not a dense visible-only list: {range:?}"
    );
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text().starts_with("Patch ") => {
                Some((text.pos, text.galley.text().to_owned()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), range.len());
    let stride = texts[columns].0.y - texts[0].0.y;
    assert!(stride < 30.0);
    for (cell, (pos, text)) in texts.iter().enumerate() {
        assert_eq!(text, &format!("Patch {:03}", cell + range.start));
        if cell % columns != 0 {
            assert!((pos.y - texts[cell - 1].0.y).abs() < 0.1);
            assert!(pos.x > texts[cell - 1].0.x);
        }
        if cell >= columns {
            assert!(((pos.y - texts[cell - columns].0.y) - stride).abs() < 0.1);
        }
    }
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().contains("Load:") || text.galley.text().contains("1 merged") || text.galley.text().contains("no category"))));
    browser.select(215);
    browser.scroll_to_selected = true;
    draw(&mut browser, wide, vec![]);
    draw(&mut browser, wide, vec![]);
    let range = ctx.data(|data| {
        data.get_temp::<std::ops::Range<usize>>(egui::Id::new("painted_patch_rows"))
            .unwrap()
    });
    assert!(
        range.contains(&215),
        "selected identity not scrolled into view: {range:?}"
    );
    draw(&mut browser, wide, vec![key(Key::ArrowLeft)]);
    assert_eq!(
        browser.requests.selected.as_ref().unwrap().candidate.path,
        browser.snapshot.as_ref().unwrap().candidates[214].path
    );
    assert_eq!(
        browser.requests.desired.as_ref().unwrap().candidate.display,
        "Bass/Patch 214"
    );
    let serial = browser.requests.selected.as_ref().unwrap().serial;
    let narrow = egui::vec2(360.0, 600.0);
    draw(&mut browser, narrow, vec![]);
    draw(&mut browser, narrow, vec![]);
    assert_eq!(browser.list_columns, 1);
    assert_eq!(browser.selected, Some(214));
    assert_eq!(browser.requests.selected.as_ref().unwrap().serial, serial);
    assert!(ctx
        .data(|data| data
            .get_temp::<std::ops::Range<usize>>(egui::Id::new("painted_patch_rows"))
            .unwrap())
        .contains(&214));
    draw(&mut browser, wide, vec![]);
    let output = draw(&mut browser, wide, vec![]);
    assert_eq!(browser.requests.selected.as_ref().unwrap().serial, serial);
    // A pointer in the scrolled grid still requests the painted candidate.
    let text = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "Patch 215" => Some(text),
            _ => None,
        })
        .unwrap();
    let point = text.pos + text.galley.size() * 0.5;
    for pressed in [true, false] {
        draw(
            &mut browser,
            wide,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
    assert_eq!(browser.selected, Some(215));
    assert_eq!(
        browser.requests.desired.as_ref().unwrap().candidate.path,
        browser.snapshot.as_ref().unwrap().candidates[215].path
    );
}

#[test]
fn partial_grid_row_has_no_phantom_cells_and_random_filter_keep_identity() {
    let ctx = egui::Context::default();
    let candidates = (0..7)
        .map(|i| candidate("six", "Six Sines", &format!("Bank/Patch {i}"), None))
        .collect();
    let mut browser = Browser {
        open: true,
        snapshot: Some(Snapshot::build(candidates, &[]).unwrap()),
        ..Default::default()
    };
    browser.filter(false);
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    assert_eq!(browser.list_columns, 2);
    click_text(&mut browser, &ctx, "Patch 6");
    assert_eq!(browser.selected, Some(6));
    let serial = browser.requests.selected.as_ref().unwrap().serial;
    frame(&mut browser, &ctx, vec![]);
    assert_eq!(browser.requests.selected.as_ref().unwrap().serial, serial);
    click_text(&mut browser, &ctx, "Random");
    let index = browser.selected.unwrap();
    assert_eq!(
        browser.requests.desired.as_ref().unwrap().candidate.path,
        browser.snapshot.as_ref().unwrap().candidates[index].path
    );
    browser.query = "Patch 6".into();
    browser.filter(true);
    assert_eq!(browser.results, [6]);
    assert_eq!(browser.selected, Some(6));
}

#[test]
fn pink_text_is_single_line_and_hover_shows_hidden_metadata() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
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
    let pink = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "Beta — Heavy" => Some(text),
            _ => None,
        })
        .unwrap();
    assert_eq!(pink.galley.rows.len(), 1);
    // The plugin prefix keeps its plugin tint; only the patch name turns pink.
    let colors: Vec<_> = pink
        .galley
        .job
        .sections
        .iter()
        .map(|section| {
            (
                &pink.galley.text()[section.byte_range.clone()],
                section.format.color,
            )
        })
        .collect();
    let dark = ctx.style().visuals.dark_mode;
    assert_eq!(
        colors,
        [
            ("Beta — ", super::row_colors::color(1, dark)),
            ("Heavy", PINK)
        ]
    );
    click_text(&mut browser, &ctx, "Beta — Heavy");
    assert!(browser.requests.desired.is_none());
    assert!(browser.requests.confirmation.is_none());
    frame(&mut browser, &ctx, vec![key(Key::Space)]);
    assert!(browser.requests.confirmation.is_some());
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![key(Key::Escape)]);
    assert!(browser.open && browser.requests.confirmation.is_none());
}
