use super::*;

/// Center of the painted row label that starts with `text`.
fn row_point(browser: &mut Browser, ctx: &egui::Context, text: &str) -> egui::Pos2 {
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
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(shape) if shape.galley.text().starts_with(text) => {
                Some(shape.pos + shape.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing row {text}"))
}

fn press(button: egui::PointerButton, pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Default::default(),
    }
}

#[test]
fn right_click_on_a_row_chooses_a_random_result() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    assert_eq!(browser.selected, None);
    let point = row_point(&mut browser, &ctx, "Alpha");
    let right = egui::PointerButton::Secondary;
    frame(
        &mut browser,
        &ctx,
        vec![egui::Event::PointerMoved(point), press(right, point, true)],
    );
    assert_eq!(browser.selected, None, "pressing alone does nothing");
    frame(&mut browser, &ctx, vec![press(right, point, false)]);
    assert!(browser.results.contains(&browser.selected.unwrap()));
}

#[test]
fn w_widens_columns_to_the_longest_label_and_back() {
    let ctx = egui::Context::default();
    let short = candidate("a", "Alpha", "Short", None);
    let long = candidate("a", "Alpha", &"long name ".repeat(30), None);
    let mut browser = Browser {
        open: true,
        snapshot: Some(Snapshot::build(vec![short, long], &[]).unwrap()),
        ..Default::default()
    };
    browser.filter(false);
    frame(&mut browser, &ctx, vec![]);
    assert!(browser.list_columns > 1);
    frame(&mut browser, &ctx, vec![key(Key::W)]);
    frame(&mut browser, &ctx, vec![]);
    assert_eq!(browser.list_columns, 1);
    frame(&mut browser, &ctx, vec![key(Key::W)]);
    frame(&mut browser, &ctx, vec![]);
    assert!(browser.list_columns > 1);
}

#[test]
fn home_and_end_select_the_first_and_last_result() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![key(Key::End)]);
    assert_eq!(browser.selected, browser.results.last().copied());
    frame(&mut browser, &ctx, vec![key(Key::Home)]);
    assert_eq!(browser.selected, browser.results.first().copied());
}

#[test]
fn sample_size_switches_to_gigabytes() {
    assert_eq!(sample_size(None), "Size: unmeasured");
    assert_eq!(sample_size(Some(64_000_000)), "Size: 64.0 MB samples");
    assert_eq!(sample_size(Some(2_500_000_000)), "Size: 2.5 GB samples");
}

/// Frames 1/18 s apart: too slow for egui to see pointer movement, as in a hidden-root browser.
fn slow_frame(
    browser: &mut Browser,
    ctx: &egui::Context,
    time: f64,
    events: Vec<egui::Event>,
) -> Vec<String> {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ctx| browser.show(ctx, None, "test"),
    );
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn tooltip_returns_after_a_click_even_at_a_low_frame_rate() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    let alpha = row_point(&mut browser, &ctx, "Alpha");
    let beta = row_point(&mut browser, &ctx, "Beta");
    let primary = egui::PointerButton::Primary;
    let mut time = 0.0;
    let mut next = |browser: &mut Browser, events| {
        time += 1.0 / 18.0;
        slow_frame(browser, &ctx, time, events)
    };
    next(
        &mut browser,
        vec![
            egui::Event::PointerMoved(alpha),
            press(primary, alpha, true),
        ],
    );
    next(&mut browser, vec![press(primary, alpha, false)]);
    assert!(browser.selected.is_some());
    next(&mut browser, vec![egui::Event::PointerMoved(beta)]);
    let tooltip = |texts: &[String]| texts.iter().any(|text| text.contains("X:/patches/b/"));
    let mut shown = false;
    for _ in 0..18 {
        shown |= tooltip(&next(&mut browser, vec![]));
    }
    assert!(shown, "the hovered patch shows its details after the delay");
}
