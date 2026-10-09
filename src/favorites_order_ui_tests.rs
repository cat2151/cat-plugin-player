//! Click the rendered controls with a native host but no installed plugins.
use super::*;
use crate::{config::PluginKey, favorites_store::Library, sequence_pattern::SequencePattern};

fn input() -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(900.0, 600.0),
        )),
        ..Default::default()
    }
}

fn frame(app: &mut App, ctx: &egui::Context) -> egui::FullOutput {
    let _ = ctx.run(input(), |ctx| app.library_panel(ctx));
    ctx.run(input(), |ctx| app.library_panel(ctx))
}

fn labels(output: &egui::FullOutput, label: &str) -> Vec<egui::Pos2> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .collect()
}

fn click(app: &mut App, ctx: &egui::Context, label: &str, occurrence: usize) {
    let output = frame(app, ctx);
    let position = labels(&output, label)[occurrence];
    for pressed in [true, false] {
        let mut input = input();
        input.events = vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ];
        let _ = ctx.run(input, |ctx| app.library_panel(ctx));
    }
}

fn ids(app: &App) -> Vec<String> {
    app.favorites
        .library
        .entries
        .iter()
        .map(|entry| entry.id.clone())
        .collect()
}

#[test]
#[ignore = "creates native host/audio device; no installed plugins required"]
fn native_reorder_controls_do_not_load_effects_and_respect_mode_boundaries_and_filter() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    assert!(
        !app.favorites.reorder_mode,
        "mode starts OFF independently of saved settings"
    );
    app.output = None;
    app.restore = None;
    app.restore_effect = Default::default();
    app.deferred_scan = false;
    let dir = std::env::temp_dir().join(format!(
        "cat-favorite-order-ui-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = dir.join("config.toml");
    app.config_path = Ok(path.clone());
    app.favorites.library = Library::default();
    app.favorites.show = true;
    app.favorites.show_history = false;
    app.favorites.error = None;
    app.filter.clear();
    app.status.clear();
    let mut saved = Vec::new();
    for (name, state) in [("Zulu", 1), ("Able", 2)] {
        saved.push(
            app.favorites
                .library
                .add(
                    &path,
                    PluginKey {
                        name: name.into(),
                        id: name.into(),
                        format: "CLAP".into(),
                        vendor: String::new(),
                        bundle_path: "X:/plugins/test.clap".into(),
                    },
                    true,
                    &[state],
                    SequencePattern::Off,
                )
                .unwrap(),
        );
    }
    let ctx = egui::Context::default();
    assert!(labels(&frame(&mut app, &ctx), "Up").is_empty());
    click(&mut app, &ctx, "☰", 0);
    click(&mut app, &ctx, "Reorder favorites", 0);
    assert!(app.favorites.reorder_mode);
    click(&mut app, &ctx, "☰", 0);
    let output = frame(&mut app, &ctx);
    assert_eq!(labels(&output, "Up").len(), 2);
    assert_eq!(labels(&output, "Down").len(), 2);
    let before = ids(&app);
    click(&mut app, &ctx, "Up", 0); // First row cannot move up.
    click(&mut app, &ctx, "Down", 1); // Last row cannot move down.
    assert_eq!(ids(&app), before);
    click(&mut app, &ctx, "Down", 0);
    assert_eq!(ids(&app), [saved[0].id.clone(), saved[1].id.clone()]);
    assert_eq!(app.status, "Favorite order saved");
    assert!(app.pending.is_none() && app.instances.is_empty());
    assert!(app.favorites.active.is_empty());
    assert_eq!(Library::load(&path).unwrap().entries[0].id, saved[0].id);
    let before = ids(&app);
    app.filter = "Zulu".into();
    click(&mut app, &ctx, "Down", 0); // Has a neighbor but filtered list must not move.
    assert_eq!(ids(&app), before);
    click(&mut app, &ctx, "☰", 0);
    click(&mut app, &ctx, "Sort by plugin name", 0);
    assert_eq!(ids(&app), before);
    click(&mut app, &ctx, "☰", 0);
    app.filter.clear();
    click(&mut app, &ctx, "☰", 0);
    click(&mut app, &ctx, "Sort by plugin name", 0);
    assert_eq!(ids(&app), [saved[1].id.clone(), saved[0].id.clone()]);
    assert_eq!(Library::load(&path).unwrap().entries[0].id, saved[1].id);
    click(&mut app, &ctx, "Up", 1); // Manual adjustment after one-shot sorting.
    assert_eq!(ids(&app), before);
    assert!(app.pending.is_none() && app.instances.is_empty());
    // History order still follows timestamp, independently of manual order.
    app.favorites.show_history = true;
    for entry in &mut app.favorites.library.entries {
        entry.history = true;
        entry.registered_at = if entry.id == saved[1].id { 2 } else { 1 };
    }
    let output = frame(&mut app, &ctx);
    assert!(labels(&output, "Up").is_empty());
    assert!(labels(&output, &saved[1].name)[0].y < labels(&output, &saved[0].name)[0].y);
    click(&mut app, &ctx, "☰", 0);
    click(&mut app, &ctx, "Sort by plugin name", 0);
    assert_eq!(ids(&app), before);
    drop(app);
    std::fs::remove_dir_all(dir).unwrap();
}
