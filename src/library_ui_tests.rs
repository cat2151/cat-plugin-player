use super::*;

fn settle(app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while app.pending.is_some() || app.scanning {
        assert!(
            std::time::Instant::now() < deadline,
            "native operation timed out"
        );
        app.host.pump_startup();
        app.handle_events();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

// A fresh context keeps hover highlighting out of the active-row assertion.
fn row(app: &mut App, label: &str, selected: bool, target: Option<&str>) {
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(900.0, 600.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| app.library_panel(ctx));
    let frame = ctx.run(input(), |ctx| app.library_panel(ctx));
    let name = frame
        .shapes
        .iter()
        .rev()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .expect("row name not rendered");
    let background = frame
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.rect.contains(name)
                    && rect.rect.width() > 600.0
                    && rect.rect.height() < ctx.style().spacing.interact_size.y * 2.0 =>
            {
                Some(rect)
            }
            _ => None,
        })
        .expect("full-width row background not rendered");
    assert_eq!(
        background.fill,
        if selected {
            ctx.style().visuals.selection.bg_fill
        } else {
            egui::Color32::TRANSPARENT
        }
    );
    let Some(target) = target else {
        return;
    };
    let position = match target {
        "name" => name,
        "icon" => egui::pos2(
            background.rect.left() + ctx.style().spacing.interact_size.y * 0.75,
            name.y,
        ),
        "menu" => frame
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.job.text == "..." && background.rect.contains(text.pos) =>
                {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            })
            .expect("favorite menu not rendered"),
        _ => panic!("unknown click target"),
    };
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
    settle(app);
}

#[test]
#[ignore = "requires installed Dexed and Dragonfly Hall/Plate Reverb CLAP plugins"]
fn native_library_rows_toggle_by_name_and_icon_and_highlight_all_connected_plugins() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.restore = None;
    app.restore_effect = Default::default();
    app.output = None;
    app.deferred_scan = false;
    let dir = std::env::temp_dir().join(format!("cat-library-ui-{}", std::process::id()));
    assert!(!dir.exists());
    app.config_path = Ok(dir.join("config.toml"));
    app.initialize_favorites();
    app.start_scan(false);
    settle(&mut app);
    let find = |name: &str| {
        app.plugins
            .iter()
            .position(|p| p.name == name && p.format == "CLAP")
            .unwrap()
    };
    let synth = find("Dexed");
    let hall = find("Dragonfly Hall Reverb");
    let plate = find("Dragonfly Plate Reverb");
    let label = |index: usize| {
        let p = &app.plugins[index];
        format!("{} - {} [{}]", p.name, p.vendor, p.format)
    };
    let synth_label = label(synth);
    let hall_label = label(hall);
    let plate_label = label(plate);
    app.favorites.show = false;
    app.filter = "Dragonfly Hall Reverb".into();
    // Effects are disabled until an instrument is loaded.
    row(&mut app, &hall_label, false, Some("icon"));
    assert!(app.effect_ids().is_empty());
    assert!(app.pending.is_none());
    app.filter = "Dexed".into();
    row(&mut app, &synth_label, false, Some("name"));
    let source = app.instrument_id().unwrap();
    row(&mut app, &synth_label, true, None);
    app.filter = "Dragonfly Hall Reverb".into();
    row(&mut app, &hall_label, false, Some("name"));
    app.filter = "Dragonfly Plate Reverb".into();
    row(&mut app, &plate_label, false, Some("icon"));
    assert_eq!(app.effect_ids().len(), 2);
    row(&mut app, &plate_label, true, None);
    app.filter = "Dragonfly Hall Reverb".into();
    row(&mut app, &hall_label, true, None);
    app.effect_bypassed = true;
    row(&mut app, &hall_label, true, Some("icon"));
    assert_eq!(app.effect_ids().len(), 1);
    assert!(app
        .connected_effect(&crate::config::PluginKey::from_plugin(&app.plugins[plate]))
        .is_some());
    row(&mut app, &hall_label, false, Some("icon"));
    assert_eq!(app.effect_ids().len(), 2);
    let effect = app
        .connected_effect(&crate::config::PluginKey::from_plugin(&app.plugins[hall]))
        .unwrap();
    app.add_favorite(effect);
    let favorite = app.favorites.library.entries[0].clone();
    app.favorites.show = true;
    app.filter = favorite.name.clone();
    // The menu must not invoke the row's remove action.
    row(&mut app, &favorite.name, true, Some("menu"));
    assert_eq!(app.effect_ids().len(), 2);
    row(&mut app, &favorite.name, true, Some("icon"));
    assert_eq!(app.effect_ids().len(), 1);
    row(&mut app, &favorite.name, false, Some("icon"));
    assert_eq!(app.effect_ids().len(), 2);
    assert_eq!(app.instrument_id(), Some(source));
    assert_eq!(app.favorites.library.entries.len(), 1);
    drop(app);
    std::fs::remove_dir_all(dir).unwrap();
}
