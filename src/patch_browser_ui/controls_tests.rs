use super::*;

#[test]
fn random_key_uses_filtered_results_and_preserves_heavy_confirmation() {
    for (query, heavy) in [("plugin:alpha", false), ("plugin:beta", true)] {
        let ctx = egui::Context::default();
        let mut browser = browser();
        browser.query = query.into();
        browser.filter(false);
        frame(&mut browser, &ctx, vec![]);
        frame(&mut browser, &ctx, vec![key(Key::R)]);
        assert_eq!(browser.selected, Some(browser.results[0]));
        assert_eq!(browser.requests.desired.is_none(), heavy);
        assert!(browser.requests.confirmation.is_none());
    }
}

#[test]
fn letter_shortcuts_preserve_search_input_and_isolate_solo_overlay() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("patch_browser_search")));
    frame(&mut browser, &ctx, vec![]);
    frame(
        &mut browser,
        &ctx,
        vec![key(Key::R), key(Key::M), egui::Event::Text("rm".into())],
    );
    assert_eq!(browser.query, "rm");
    assert!(browser.solo_draft.is_none());
    assert!(browser.requests.selected.is_none());
    ctx.memory_mut(|memory| memory.surrender_focus(egui::Id::new("patch_browser_search")));
    browser.query = "[".into();
    browser.filter(false);
    frame(&mut browser, &ctx, vec![key(Key::R)]);
    assert!(browser.requests.selected.is_none());
    frame(&mut browser, &ctx, vec![key(Key::M)]);
    assert_eq!(browser.solo_draft.as_deref(), Some("["));
    frame(&mut browser, &ctx, vec![key(Key::R), key(Key::M)]);
    assert!(browser.requests.selected.is_none());
    frame(&mut browser, &ctx, vec![key(Key::Escape)]);
    assert!(browser.open && browser.solo_draft.is_none());
    browser.query = String::new();
    browser.filter(false);
    browser.condition_menu = true;
    frame(&mut browser, &ctx, vec![key(Key::R), key(Key::M)]);
    assert!(browser.solo_draft.is_none());
    assert!(browser.requests.selected.is_none());
}

#[test]
fn toolbar_is_one_line_with_bounded_controls_at_wide_and_minimum_width() {
    for width in [1920.0, 360.0] {
        let ctx = egui::Context::default();
        let mut browser = browser();
        for _ in 0..2 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 700.0),
                    )),
                    ..Default::default()
                },
                |ctx| browser.show(ctx, None, "test"),
            );
        }
        let rects = ctx.data(|data| {
            data.get_temp::<[egui::Rect; 4]>(egui::Id::new("browser_toolbar"))
                .unwrap()
        });
        assert!(rects[0].width() > 100.0);
        assert!(rects[1].width() <= 90.0);
        assert!(rects[3].right() <= width);
        for pair in rects.windows(2) {
            assert!(pair[0].right() < pair[1].left());
            assert!((pair[0].center().y - pair[1].center().y).abs() < 1.0);
        }
    }
}

#[test]
fn pointer_solo_mute_batches_ok_cancels_and_toggles_active_mode() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    browser.query = "-bright".into();
    browser.filter(false);
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    let original = browser.results.clone();
    click_text(&mut browser, &ctx, "Solo/Mute");
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    click_text_n(&mut browser, &ctx, "Solo", 0);
    click_text_n(&mut browser, &ctx, "Solo", 1);
    assert_eq!(browser.query, "-bright");
    assert_eq!(browser.results, original);
    assert!(browser.requests.desired.is_none());
    assert_eq!(
        browser.solo_draft.as_deref(),
        Some("-bright plugin:alpha plugin:beta")
    );
    frame(
        &mut browser,
        &ctx,
        vec![key(Key::ArrowDown), key(Key::J), key(Key::Space)],
    );
    assert!(browser.requests.selected.is_none());
    frame(&mut browser, &ctx, vec![key(Key::Enter)]);
    assert!(browser.solo_draft.is_none());
    assert!(browser.requests.confirmation.is_none());
    assert_eq!(browser.query, "-bright plugin:alpha plugin:beta");
    assert_eq!(
        browser.results,
        browser
            .snapshot
            .as_ref()
            .unwrap()
            .filter(browser.role, browser.preset, &browser.query)
            .unwrap()
    );
    let serial = browser
        .requests
        .desired
        .as_ref()
        .map(|ticket| ticket.serial);
    click_text(&mut browser, &ctx, "Solo/Mute");
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    click_text_n(&mut browser, &ctx, "Solo", 0);
    assert_eq!(
        plugin_mode(browser.solo_draft.as_deref().unwrap(), "alpha"),
        None
    );
    frame(&mut browser, &ctx, vec![key(Key::Escape)]);
    assert!(browser.open && browser.solo_draft.is_none());
    assert_eq!(browser.query, "-bright plugin:alpha plugin:beta");
    assert_eq!(
        browser
            .requests
            .desired
            .as_ref()
            .map(|ticket| ticket.serial),
        serial
    );
    click_text(&mut browser, &ctx, "Solo/Mute");
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    click_text_n(&mut browser, &ctx, "Mute", 1);
    click_text(&mut browser, &ctx, "OK");
    assert_eq!(browser.query, "-bright -plugin:beta");
    assert_eq!(browser.results.len(), 1);
    assert_eq!(
        browser.snapshot.as_ref().unwrap().candidates[browser.results[0]].plugin_id,
        "a"
    );
}

#[test]
fn child_outside_click_and_menu_escape_do_not_close_browser_or_select_rows() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    click_text(&mut browser, &ctx, "Solo/Mute");
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    let point = egui::pos2(5.0, 5.0);
    frame(
        &mut browser,
        &ctx,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ],
    );
    frame(
        &mut browser,
        &ctx,
        vec![egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
    );
    assert!(browser.open && browser.solo_draft.is_none());
    assert!(browser.requests.selected.is_none());
    click_text(&mut browser, &ctx, "☰");
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    frame(
        &mut browser,
        &ctx,
        vec![key(Key::ArrowDown), key(Key::Space), key(Key::Enter)],
    );
    assert!(browser.requests.selected.is_none());
    frame(&mut browser, &ctx, vec![key(Key::Escape)]);
    assert!(browser.open && !browser.condition_menu);
    frame(&mut browser, &ctx, vec![key(Key::Escape)]);
    assert!(!browser.open);
}

#[test]
fn slash_focuses_search_without_typing_and_overlay_modes_sit_beside_names() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(
        &mut browser,
        &ctx,
        vec![key(Key::Slash), egui::Event::Text("/".into())],
    );
    frame(&mut browser, &ctx, vec![egui::Event::Text("pad".into())]);
    assert_eq!(browser.query, "pad");
    ctx.memory_mut(|memory| memory.surrender_focus(egui::Id::new("patch_browser_search")));
    frame(&mut browser, &ctx, vec![key(Key::M)]);
    assert!(browser.solo_draft.is_some());
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
    let left = |text: &str| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(shape) if shape.galley.text() == text => {
                    Some((shape.pos.x, shape.pos.x + shape.galley.size().x))
                }
                _ => None,
            })
            .unwrap()
    };
    let (_, name_right) = left("Alpha");
    let (solo_left, _) = left("Solo");
    assert!(solo_left - name_right < 40.0, "{name_right} .. {solo_left}");
}
