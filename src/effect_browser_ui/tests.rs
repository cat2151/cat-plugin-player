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
    let _ = ctx.run(input, |ctx| browser.show(ctx, &view));
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
