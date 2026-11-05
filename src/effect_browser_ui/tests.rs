use super::*;
use crate::effect_browser::tests::browser;

fn frame(browser: &mut EffectBrowser, ctx: &egui::Context, events: Vec<egui::Event>) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(900.0, 700.0),
        )),
        events,
        ..Default::default()
    };
    let view = View {
        slots: vec![(10, "Surge [CLAP]".into())],
        taken: vec![],
        loading: false,
        busy: false,
        status: "",
        success: None,
    };
    let _ = ctx.run(input, |ctx| {
        let palette_input = crate::command_palette::begin(
            ctx,
            crate::command_palette::effect_commands(browser),
            true,
        );
        browser.show(ctx, &view);
        crate::command_palette::end(ctx, palette_input);
    });
}

#[test]
fn palette_cancel_keeps_browser_open_and_run_dispatches_column_width_once() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    let mut toggle = key(Key::P);
    if let egui::Event::Key { modifiers, .. } = &mut toggle {
        *modifiers = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    }
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![toggle.clone()]);
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![key(Key::Escape)]);
    assert!(browser.open);
    frame(&mut browser, &ctx, vec![toggle]);
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![egui::Event::Text("column".into())]);
    let before = browser.fit_columns;
    frame(&mut browser, &ctx, vec![key(Key::Enter)]);
    frame(&mut browser, &ctx, vec![]);
    assert_ne!(browser.fit_columns, before);
    assert!(browser.query.is_empty());
    assert!(browser.open);
    frame(&mut browser, &ctx, vec![]);
    assert_ne!(browser.fit_columns, before);
}

fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn favorite_does_not_apply_a_candidate_and_ignores_search() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![key(Key::F)]);
    assert!(std::mem::take(&mut browser.favorite_requested));
    assert!(browser.desired.is_none());
    let mut release = key(Key::F);
    if let egui::Event::Key { pressed, .. } = &mut release {
        *pressed = false;
    }
    frame(&mut browser, &ctx, vec![release]);
    frame(&mut browser, &ctx, vec![key(Key::Slash)]);
    frame(
        &mut browser,
        &ctx,
        vec![key(Key::F), egui::Event::Text("f".into())],
    );
    assert!(!browser.favorite_requested);
    assert_eq!(browser.query, "f");
}

fn selected(browser: &EffectBrowser) -> &str {
    &browser.candidates[browser.selected.unwrap()].name
}

#[test]
fn headless_grid_keys_count_search_focus_and_close() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    assert!(browser.desired.is_none(), "open alone does not apply");
    // 900px fits three columns: Big Hall, Tape Echo, Small Room / Plate.
    frame(&mut browser, &ctx, vec![key(Key::J)]);
    assert_eq!(selected(&browser), "Big Hall");
    frame(&mut browser, &ctx, vec![key(Key::L)]);
    assert_eq!(selected(&browser), "Tape Echo");
    assert_eq!(browser.desired.as_ref().unwrap().name, "Tape Echo");
    frame(&mut browser, &ctx, vec![key(Key::J)]);
    assert_eq!(selected(&browser), "Plate");
    // A count repeats the step: Plate -> Small Room -> Tape Echo.
    frame(&mut browser, &ctx, vec![key(Key::Num2)]);
    frame(&mut browser, &ctx, vec![key(Key::H)]);
    assert_eq!(selected(&browser), "Tape Echo");
    // "/" focuses search; typed letters then edit the query instead of moving.
    frame(&mut browser, &ctx, vec![key(Key::Slash)]);
    frame(
        &mut browser,
        &ctx,
        vec![key(Key::L), egui::Event::Text("room".into())],
    );
    assert_eq!(browser.query, "room");
    assert_eq!(selected(&browser), "Small Room");
    ctx.memory_mut(|memory| memory.stop_text_input());
    frame(&mut browser, &ctx, vec![key(Key::Escape)]);
    assert!(!browser.open);
    assert!(
        browser.desired.is_none(),
        "closing drops an unsent selection"
    );
}

#[test]
fn headless_w_toggles_columns_that_fit_every_label() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    let default = browser.list_columns;
    // Short labels fit narrower cells, so more columns appear; W again restores the default.
    frame(&mut browser, &ctx, vec![key(Key::W)]);
    frame(&mut browser, &ctx, vec![]);
    assert!(browser.list_columns > default, "{}", browser.list_columns);
    frame(&mut browser, &ctx, vec![key(Key::W)]);
    frame(&mut browser, &ctx, vec![]);
    assert_eq!(browser.list_columns, default);
}

#[test]
fn headless_enter_closes_and_random_stays_in_results() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    browser.choose_category(2);
    browser.desired = None;
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![key(Key::R)]);
    assert_eq!(selected(&browser), "Tape Echo");
    frame(&mut browser, &ctx, vec![key(Key::Enter)]);
    assert!(!browser.open);
}

#[test]
#[ignore = "creates native host/audio device; no installed plugins required"]
fn native_main_window_e_or_x_opens_effect_browser() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.output = None;
    app.restore = None;
    app.restore_effect = Default::default();
    app.deferred_scan = false;
    let ctx = egui::Context::default();
    for letter in [Key::X, Key::E] {
        app.effect_browser.close();
        let input = egui::RawInput {
            events: vec![key(letter)],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.playback_controls(ui));
        });
        assert!(
            app.effect_browser.open,
            "{letter:?} opens the effect browser"
        );
    }
}

// Match the patch browser regression: main-window-hidden frames arrive at 18 Hz.
fn hover_frame(
    browser: &mut EffectBrowser,
    ctx: &egui::Context,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ctx| {
            let view = View {
                slots: vec![],
                taken: vec!["id.Dragonfly".into()],
                loading: false,
                busy: false,
                status: "",
                success: None,
            };
            browser.show(ctx, &view);
        },
    )
}

fn effect_point(output: &egui::FullOutput, name: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text().contains(name) => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing effect {name}"))
}

fn cell_background(output: &egui::FullOutput, point: egui::Pos2) -> (Color32, egui::Stroke) {
    output
        .shapes
        .iter()
        .rev()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.rect.contains(point) => Some((rect.fill, rect.stroke)),
            _ => None,
        })
        .unwrap()
}

#[test]
fn hovering_effects_preserves_cell_background_and_selection_in_both_themes() {
    for dark in [true, false] {
        let ctx = egui::Context::default();
        ctx.set_visuals(if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        let mut browser = browser();
        browser.select(0);
        browser.desired = None;
        hover_frame(&mut browser, &ctx, 0.0, vec![]);
        let idle = hover_frame(&mut browser, &ctx, 0.1, vec![]);
        let selected = effect_point(&idle, "Big Hall");
        let other = effect_point(&idle, "Small Room");
        for point in [other, selected] {
            let hovered = hover_frame(
                &mut browser,
                &ctx,
                0.1,
                vec![egui::Event::PointerMoved(point)],
            );
            assert_eq!(
                cell_background(&idle, point),
                cell_background(&hovered, point)
            );
            assert_eq!(browser.selected, Some(0));
            assert!(browser.desired.is_none());
        }
    }
}

#[test]
fn effect_tooltip_returns_after_click_at_low_frame_rate_with_taken_details() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    hover_frame(&mut browser, &ctx, 0.0, vec![]);
    let output = hover_frame(&mut browser, &ctx, 0.01, vec![]);
    let hall = effect_point(&output, "Big Hall");
    let room = effect_point(&output, "Small Room");
    let mut time = 0.01;
    let mut next = |browser: &mut EffectBrowser, events| {
        time += 1.0 / 18.0;
        hover_frame(browser, &ctx, time, events)
    };
    let click = |pressed| egui::Event::PointerButton {
        pos: hall,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    next(
        &mut browser,
        vec![egui::Event::PointerMoved(hall), click(true)],
    );
    next(&mut browser, vec![click(false)]);
    assert_eq!(selected(&browser), "Big Hall");
    browser.desired = None;
    let moved = next(&mut browser, vec![egui::Event::PointerMoved(room)]);
    let tooltip = |output: &egui::FullOutput| {
        output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text)
            if text.galley.text().contains("X:/presets/Dragonfly/Small Room")
                && text.galley.text().contains("already connected in another slot"))
        })
    };
    assert!(!tooltip(&moved), "new cell must wait for its tooltip delay");
    let mut shown = false;
    for _ in 0..18 {
        shown |= tooltip(&next(&mut browser, vec![]));
    }
    assert!(shown);
    assert_eq!(selected(&browser), "Big Hall");
    assert!(browser.desired.is_none());
}
