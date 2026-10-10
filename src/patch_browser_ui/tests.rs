use super::*;
use crate::{
    patch_browser_catalog::{tests::candidate, Snapshot},
    patch_filter_store::Store,
};

fn browser() -> Browser {
    let normal = candidate("a", "Alpha", "Bass", Some("Bass"));
    let mut heavy = candidate("b", "Beta", "Heavy", None);
    heavy.measurement.sfz_sample_bytes = Some(64_000_000);
    let mut browser = Browser {
        open: true,
        snapshot: Some(Snapshot::build(vec![normal, heavy], &[]).unwrap()),
        ..Default::default()
    };
    browser.filter(false);
    browser
}

fn frame(browser: &mut Browser, ctx: &egui::Context, events: Vec<egui::Event>) {
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
fn headless_open_normal_arrow_heavy_space_enter_cancel_and_search_focus() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    assert!(
        browser.requests.desired.is_none(),
        "open alone does not apply"
    );
    frame(&mut browser, &ctx, vec![key(Key::ArrowDown)]);
    assert_eq!(
        browser.requests.desired.as_ref().unwrap().candidate.display,
        "Bass"
    );
    // Both patches share the first grid row.
    frame(&mut browser, &ctx, vec![key(Key::ArrowRight)]);
    assert_eq!(
        browser
            .requests
            .selected
            .as_ref()
            .unwrap()
            .candidate
            .display,
        "Heavy"
    );
    assert!(browser.requests.desired.is_none());
    // Space decides: a pink patch opens its confirmation instead of applying.
    frame(&mut browser, &ctx, vec![key(Key::Space)]);
    assert!(browser.requests.desired.is_none());
    assert!(browser.requests.confirmation.is_some());
    frame(&mut browser, &ctx, vec![]); // draw confirmation
    frame(&mut browser, &ctx, vec![key(Key::Escape)]);
    assert!(browser.requests.confirmation.is_none());
    assert!(browser.requests.desired.is_none());
    frame(&mut browser, &ctx, vec![key(Key::Space)]);
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![key(Key::Enter)]);
    assert_eq!(
        browser.requests.desired.as_ref().unwrap().candidate.display,
        "Heavy"
    );
    browser.requests.desired = None;
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("patch_browser_search")));
    frame(&mut browser, &ctx, vec![]);
    assert_eq!(
        ctx.memory(|memory| memory.focused()),
        Some(egui::Id::new("patch_browser_search"))
    );
    frame(
        &mut browser,
        &ctx,
        vec![
            key(Key::Space),
            egui::Event::Text(" ".into()),
            key(Key::Enter),
        ],
    );
    assert!(browser.requests.confirmation.is_none());
    assert!(browser.requests.desired.is_none());
}

#[test]
fn headless_invalid_zero_loading_failure_and_narrow_controls() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    browser.query = "[".into();
    browser.filter(true);
    assert!(browser.error.is_some());
    assert!(browser.requests.desired.is_none());
    frame(&mut browser, &ctx, vec![]);
    browser.query = "missing".into();
    browser.filter(true);
    assert!(browser.error.is_none() && browser.results.is_empty());
    browser.query = "plugin:alpha".into();
    browser.filter(true);
    assert_eq!(browser.results.len(), 1);
    assert_eq!(
        browser
            .requests
            .desired
            .as_ref()
            .unwrap()
            .candidate
            .plugin_id,
        "a"
    );
    browser.store = Some(Store::load(&Err("test config unavailable".into())));
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(360.0, 420.0),
            )),
            ..Default::default()
        },
        |ctx| browser.show(ctx, None, "test"),
    );
    browser.snapshot = None;
    frame(&mut browser, &ctx, vec![]);
    browser.build_error = Some("test catalog failure".into());
    frame(&mut browser, &ctx, vec![]);
}

#[test]
fn actual_mouse_selection_and_saved_condition_controls() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    let candidates = browser.snapshot.as_ref().unwrap().candidates.clone();
    let root = std::env::temp_dir().join(format!(
        "cat-browser-ui-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = Ok(root.join("config.toml"));
    browser.store = Some(Store::load(&config));
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    click_text(&mut browser, &ctx, "Alpha — Bass");
    assert_eq!(
        browser
            .requests
            .desired
            .as_ref()
            .unwrap()
            .candidate
            .plugin_id,
        "a"
    );
    browser.query = "bass".into();
    browser.filter(true);
    click_text(&mut browser, &ctx, "☰");
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    click_text(&mut browser, &ctx, "Save condition to Role");
    assert_eq!(
        Store::load(&config).presets,
        [("etc".into(), "bass".into())]
    );
    browser.snapshot =
        Some(Snapshot::build(candidates, &browser.store.as_ref().unwrap().presets).unwrap());
    browser.filter(false);
    frame(&mut browser, &ctx, vec![]);
    click_text(&mut browser, &ctx, "Etc / unknown");
    frame(&mut browser, &ctx, vec![]);
    click_text(&mut browser, &ctx, "Delete");
    assert!(Store::load(&config).presets.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

fn click_text(browser: &mut Browser, ctx: &egui::Context, text: &str) {
    click_text_n(browser, ctx, text, 0);
}

fn click_text_n(browser: &mut Browser, ctx: &egui::Context, text: &str, at: usize) {
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
    let point = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(shape)
                if if matches!(text, "Solo" | "Mute") {
                    shape.galley.text() == text
                } else {
                    shape.galley.text().starts_with(text)
                } =>
            {
                Some(shape.pos + shape.galley.size() * 0.5)
            }
            _ => None,
        })
        .nth(at)
        .unwrap_or_else(|| panic!("missing painted control {text}"));
    frame(
        browser,
        ctx,
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
        browser,
        ctx,
        vec![egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
    );
}

#[test]
fn long_paths_keep_virtual_rows_at_the_same_stride() {
    let ctx = egui::Context::default();
    let short = candidate("a", "Alpha", "Short", None);
    let long = candidate("b", "Beta", &"long-directory/".repeat(40), None);
    let mut browser = Browser {
        snapshot: Some(Snapshot::build(vec![short, long], &[]).unwrap()),
        ..Default::default()
    };
    browser.filter(false);
    let mut height = 0.0;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(260.0, 220.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(240.0);
                ui.set_max_height(200.0);
                let before = ui.cursor().min.y;
                browser.rows(ui);
                height = ui.cursor().min.y - before;
                assert!(
                    height <= 220.0,
                    "wrapped path expanded virtual rows: {height}"
                );
            });
        },
    );
    assert!(height > 0.0);
}

#[test]
fn framed_panes_use_window_and_child_close_invalidates_requests() {
    for dimensions in [egui::vec2(1920.0, 1080.0), egui::vec2(360.0, 420.0)] {
        let ctx = egui::Context::default();
        let mut browser = browser();
        for _ in 0..2 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, dimensions)),
                    ..Default::default()
                },
                |ctx| browser.show(ctx, None, "test"),
            );
        }
        let [role, preset, patches] = ctx.data(|data| {
            data.get_temp::<[egui::Rect; 3]>(egui::Id::new("browser_panes"))
                .unwrap()
        });
        assert!(role.max.y < preset.min.y);
        assert!(preset.max.x < patches.min.x);
        assert!(patches.width() > dimensions.x * 0.6);
        assert!(patches.max.y <= dimensions.y);
        assert!(patches.height() > dimensions.y * 0.5);
        browser.select(0);
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let _ = ctx.run(input, |ctx| browser.show(ctx, None, "test"));
        assert!(!browser.open);
        assert!(browser.requests.desired.is_none());
    }
}

#[test]
fn preset_last_row_is_reachable_by_pointer_scroll() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    let users: Vec<_> = (0..60)
        .map(|index| ("etc".into(), format!("fixture-{index:02}")))
        .collect();
    let candidates = browser.snapshot.as_ref().unwrap().candidates.clone();
    browser.snapshot = Some(Snapshot::build(candidates, &users).unwrap());
    browser.store = Some(Store::load(&Err("read-only fixture".into())));
    browser.role = FilterGroup::ALL
        .iter()
        .position(|role| role.role() == Some(cmrt_patches::PatchRole::Etc))
        .unwrap();
    browser.filter(false);
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![]);
    let (rect, _, content) = ctx.data(|data| {
        data.get_temp::<(egui::Rect, f32, f32)>(egui::Id::new("preset_scroll"))
            .unwrap()
    });
    assert!(content > rect.height());
    frame(
        &mut browser,
        &ctx,
        vec![
            egui::Event::PointerMoved(rect.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -10000.0),
                modifiers: Default::default(),
            },
        ],
    );
    for _ in 0..20 {
        frame(&mut browser, &ctx, vec![]);
    }
    let (rect, offset, content) = ctx.data(|data| {
        data.get_temp::<(egui::Rect, f32, f32)>(egui::Id::new("preset_scroll"))
            .unwrap()
    });
    assert!(
        offset + rect.height() >= content - 2.0,
        "last preset unreachable: {offset} + {} < {content}",
        rect.height()
    );
}

#[path = "controls_tests.rs"]
mod controls_tests;

#[path = "list_tests.rs"]
mod list_tests;
#[path = "reveal_tests.rs"]
mod reveal_tests;
#[path = "scroll_tests.rs"]
mod scroll_tests;

#[path = "status_tests.rs"]
mod status_tests;
