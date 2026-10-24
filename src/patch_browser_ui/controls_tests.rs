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

fn shifted(key: Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn solo_overlay_letters_solo_lower_and_mute_upper_then_apply_at_once() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![key(Key::M)]);
    // Letters naming no plugin, and Ctrl, leave the overlay open and the draft unchanged.
    let mut release = shifted(Key::B, egui::Modifiers::CTRL);
    if let egui::Event::Key { pressed, .. } = &mut release {
        *pressed = false;
    }
    frame(
        &mut browser,
        &ctx,
        vec![key(Key::Q), shifted(Key::B, egui::Modifiers::CTRL), release],
    );
    assert_eq!(browser.solo_draft.as_deref(), Some(""));
    frame(&mut browser, &ctx, vec![key(Key::B)]);
    assert!(browser.solo_draft.is_none());
    assert_eq!(browser.query, "plugin:beta");
    frame(&mut browser, &ctx, vec![key(Key::M)]);
    frame(
        &mut browser,
        &ctx,
        vec![shifted(Key::A, egui::Modifiers::SHIFT)],
    );
    assert!(browser.solo_draft.is_none());
    assert_eq!(browser.query, "-plugin:alpha");
}

#[test]
fn main_random_waits_for_catalog_then_uses_the_filter_or_reports_why_not() {
    use crate::patch_browser::MainRandom;
    let mut browser = browser();
    browser.open = false;
    let snapshot = browser.snapshot.take();
    browser.main_random = MainRandom::Catalog;
    assert_eq!(browser.poll_main_random(), None);
    assert_eq!(browser.main_random, MainRandom::Catalog);
    browser.snapshot = snapshot;
    browser.query = "plugin:alpha".into();
    browser.filter(false);
    assert_eq!(browser.poll_main_random(), None);
    assert_eq!(browser.main_random, MainRandom::Start);
    assert_eq!(
        browser.requests.desired.as_ref().unwrap().candidate.display,
        "Bass"
    );
    // A live change that drops the request also ends the closed browser's dispatch.
    browser.requests.invalidate();
    assert_eq!(browser.poll_main_random(), None);
    assert_eq!(browser.main_random, MainRandom::Idle);

    browser.query = "plugin:beta".into();
    browser.filter(false);
    browser.main_random = MainRandom::Catalog;
    let status = browser.poll_main_random().unwrap();
    assert!(status.contains("press Space"), "{status}");
    assert_eq!(browser.main_random, MainRandom::Idle);

    browser.query = "nothing-matches".into();
    browser.filter(false);
    browser.main_random = MainRandom::Catalog;
    let status = browser.poll_main_random().unwrap();
    assert!(status.contains("no patch matches"), "{status}");
}

#[test]
fn main_random_overlay_paints_before_selection_and_lasts_through_apply() {
    use crate::patch_browser::MainRandom;
    let mut browser = browser();
    browser.open = false;
    browser.query = "plugin:alpha".into();
    browser.filter(false);
    let ctx = egui::Context::default();
    browser.begin_main_random(&ctx);
    assert!(
        browser.requests.desired.is_none(),
        "selection must wait for paint"
    );
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        browser.show_main_random_overlay(ctx);
    });
    assert!(
        output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::epaint::Shape::Text(text) if text.galley.job.text == "Changing patch..."
        )),
        "the very first frame must contain the overlay text"
    );
    assert_eq!(browser.main_random, MainRandom::Catalog);
    assert_eq!(browser.poll_main_random(), None);
    browser.requests.next().unwrap();
    browser.main_random = MainRandom::Idle;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        browser.show_main_random_overlay(ctx);
    });
    assert!(browser.main_random_overlay);
    browser.requests.ready().unwrap();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        browser.show_main_random_overlay(ctx);
    });
    assert!(browser.main_random_overlay);
    browser.requests.finish(true);
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        browser.show_main_random_overlay(ctx);
    });
    assert!(!browser.main_random_overlay);
    assert!(output.shapes.is_empty());
}

#[test]
fn main_random_overlay_closes_when_no_patch_loads_or_preparation_fails() {
    let ctx = egui::Context::default();
    for query in ["nothing-matches", "[", "plugin:beta", "plugin:alpha"] {
        let mut browser = browser();
        browser.query = query.into();
        browser.filter(false);
        browser.begin_main_random(&ctx);
        browser.poll_main_random();
        if query == "plugin:alpha" {
            browser.requests.next().unwrap();
            browser.main_random = crate::patch_browser::MainRandom::Idle;
            assert!(browser.requests.failed_preparation());
        }
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            browser.show_main_random_overlay(ctx);
        });
        assert!(!browser.main_random_overlay, "{query}");
    }
}

#[test]
fn main_random_waits_for_presented_frame_even_when_sizing_discards_a_pass() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    browser.query = "plugin:alpha".into();
    browser.filter(false);
    let mut passes = 0;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        if passes == 0 {
            browser.begin_main_random(ctx);
            ctx.request_discard("test sizing pass");
        }
        passes += 1;
        assert!(browser.main_random_waiting_for_paint(ctx));
        assert!(browser.requests.desired.is_none());
        browser.show_main_random_overlay(ctx);
    });
    assert_eq!(passes, 2);
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        assert!(!browser.main_random_waiting_for_paint(ctx));
        browser.poll_main_random();
        assert!(browser.requests.desired.is_some());
    });
}
