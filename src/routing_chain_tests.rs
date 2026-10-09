use super::*;

#[test]
#[ignore = "requires installed Dexed, Dragonfly Hall/Room/Plate Reverb CLAP and audio device"]
fn native_ordered_effects_transaction_restore_and_dsp() {
    let _library = crate::native_library::load().unwrap();
    let mut app = App::prepare();
    app.restore = None;
    app.restore_effect.clear();
    app.output = None;
    app.deferred_scan = false;
    let dir = std::env::temp_dir().join(format!("cat-chain-{}", std::process::id()));
    assert!(!dir.exists());
    let path = dir.join("config.toml");
    app.config_path = Ok(path.clone());
    app.start_scan(false);
    settle(&mut app);
    let find = |name| {
        app.plugins
            .iter()
            .position(|p| p.name == name && p.format == "CLAP")
            .expect("required order-test plugin missing")
    };
    let source = find("Dexed");
    let hall = find("Dragonfly Hall Reverb");
    let room = find("Dragonfly Room Reverb");
    let plate = find("Dragonfly Plate Reverb");
    app.load_plugin(source);
    settle(&mut app);
    assert_eq!(plugin_action(&mut app, hall), "Load");
    for index in [hall, room, plate] {
        let expected = if app.effect_ids().is_empty() {
            "Load"
        } else {
            "Add"
        };
        assert_eq!(plugin_action(&mut app, index), expected);
        app.load_plugin(index);
        settle(&mut app);
        assert_eq!(plugin_action(&mut app, index), "Remove");
    }
    let effects = app.effect_ids();
    assert_eq!(effects.len(), 3);
    drag_effect(&mut app, 2, 0);
    assert_eq!(
        app.effect_ids(),
        [effects[2], effects[0], effects[1]],
        "vertical UI drop must commit the effect order"
    );
    assert!(app.reorder_effect(effects[2], effects[1], true));
    assert_eq!(app.effect_ids(), effects);
    app.load_plugin(hall);
    assert_eq!(
        app.effect_ids(),
        effects,
        "duplicate identity must not create another instance"
    );
    assert!(app.reorder_effect(effects[2], effects[0], false));
    assert_eq!(app.effect_ids(), [effects[2], effects[0], effects[1]]);
    let disk = crate::status::Status::load(&path).unwrap();
    assert_eq!(
        disk.effects.iter().map(|key| &key.id).collect::<Vec<_>>(),
        app.instances[1..]
            .iter()
            .map(|i| &i.plugin.id)
            .collect::<Vec<_>>()
    );
    // The middle effect is saved and removed, preserving both neighbors.
    assert!(app.remove_plugin(effects[0]));
    assert_eq!(app.effect_ids(), [effects[2], effects[1]]);
    app.load_plugin(hall);
    settle(&mut app);
    assert_eq!(app.effect_ids().len(), 3);
    let old = app.effect_ids();
    app.load_plugin(source);
    settle(&mut app);
    assert_eq!(
        app.effect_ids(),
        old,
        "source replacement keeps every effect"
    );
    let bytes = std::fs::read(crate::status::path(&path)).unwrap();
    let mut bad = app.plugins[source].clone();
    bad.kind = PluginKind::Effect;
    app.plugins.push(bad);
    app.load_plugin(app.plugins.len() - 1);
    settle(&mut app);
    assert_eq!(app.effect_ids(), old);
    assert!(app.status.contains("Could not connect"));
    assert_eq!(std::fs::read(crate::status::path(&path)).unwrap(), bytes);
    for id in app.instances.iter().map(|i| i.id).collect::<Vec<_>>() {
        app.save_plugin_state(id).unwrap();
    }
    let expected = crate::status::Status::load(&path).unwrap().effects;
    drop(app);
    let mut restored = restore_app(&path);
    restored.output = None;
    restored.restore_before_gui();
    settle(&mut restored);
    assert_eq!(
        restored.instances[1..]
            .iter()
            .map(|i| i.plugin.clone())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(!restored.restoring);
    // Native callbacks exercise stream lifetime boundaries for every mutation.
    let mut live = restore_app(&path);
    assert!(live.output.is_some(), "audio device unavailable");
    live.restore_before_gui();
    settle(&mut live);
    let ids = live.effect_ids();
    assert!(live.instances[0].voice.as_ref().unwrap().has_rendered());
    assert!(live.reorder_effect(ids[2], ids[0], false));
    assert!(live.remove_plugin(ids[1]));
    live.pause_audio();
    drop(live);
    restored.save_session();
    drop(restored);

    // Fresh hosts, identical saved states and events isolate order from tails.
    // Stereo modulated Hall and Room reverbs are order-sensitive DSP stages.
    let a = render_fresh(&path, false);
    let repeat = render_fresh(&path, false);
    let b = render_fresh(&path, true);
    let difference = |x: &[f32], y: &[f32]| {
        x.iter()
            .zip(y)
            .map(|(a, b)| f64::from(a - b).powi(2))
            .sum::<f64>()
    };
    let noise = difference(&a, &repeat);
    let order = difference(&a, &b);
    let energy = a.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
    eprintln!(
        "ordered DSP energy={energy}, same-order delta={noise}, reversed-order delta={order}"
    );
    assert!(energy > 0.001);
    assert!(
        order > noise * 10.0 + 0.000001,
        "order-sensitive DSP evidence unavailable"
    );

    // Failure after one restored effect discards ALL restored effects, keeps disk.
    let mut missing = crate::status::Status::load(&path).unwrap();
    missing.effects[1].id = "missing.middle.effect".into();
    missing.effects[1].bundle_path = "X:/missing/effect.clap".into();
    missing.save(&path).unwrap();
    let bytes = std::fs::read(crate::status::path(&path)).unwrap();
    let mut failed = restore_app(&path);
    failed.output = None;
    failed.restore_before_gui();
    failed.start_scan(false);
    settle(&mut failed);
    assert_eq!(failed.instances.len(), 1, "{}", failed.status);
    assert!(failed.effect_ids().is_empty() && !failed.restoring);
    assert!(failed
        .config_error
        .as_ref()
        .unwrap()
        .contains("Previous effect not found"));
    assert_eq!(std::fs::read(crate::status::path(&path)).unwrap(), bytes);
    drop(failed);
    std::fs::remove_dir_all(dir).unwrap();
}

fn render_fresh(path: &std::path::Path, reverse: bool) -> Vec<f32> {
    let mut app = restore_app(path);
    app.output = None;
    app.restore_before_gui();
    settle(&mut app);
    let mut effects = app.effect_ids();
    if reverse {
        effects.reverse();
    }
    let processor = app
        .host
        .create_chain(
            app.instrument_id().unwrap(),
            &effects,
            false,
            48_000,
            audio::MAX_BLOCK_FRAMES,
        )
        .unwrap();
    let mut renderer = unsafe { processor.audio_ref() };
    let mut block = [0.0; 2048];
    let mut samples = Vec::new();
    for i in 0..96 {
        let events = match i {
            0 => vec![crate::seq::note_on(0, 60, 100)],
            16 => vec![crate::seq::note_off(0, 60)],
            _ => Vec::new(),
        };
        assert!(renderer.process(&events, &mut block, 2));
        assert!(block.iter().all(|sample| sample.is_finite()));
        samples.extend_from_slice(&block);
    }
    drop(processor);
    drop(app);
    samples
}

fn drag_effect(app: &mut App, from: usize, to: usize) {
    use eframe::egui;
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 1000.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| app.routing_panel(ctx, false));
    let frame = ctx.run(input(), |ctx| app.routing_panel(ctx, false));
    let handles = frame
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == "::" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handles.len(),
        app.effect_ids().len(),
        "instrument has no drag handle"
    );
    let start = handles[from];
    let end = handles[to];
    let mut press = input();
    press.events = vec![
        egui::Event::PointerMoved(start),
        egui::Event::PointerButton {
            pos: start,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        },
    ];
    let _ = ctx.run(press, |ctx| app.routing_panel(ctx, false));
    let mut movement = input();
    movement.events = vec![egui::Event::PointerMoved(start + egui::vec2(0.0, -12.0))];
    let dragging = ctx.run(movement, |ctx| app.routing_panel(ctx, false));
    assert_eq!(
        *egui::DragAndDrop::payload::<i32>(&ctx).unwrap(),
        app.effect_ids()[from]
    );
    assert_eq!(
        dragging.platform_output.cursor_icon,
        egui::CursorIcon::Grabbing
    );
    let mut movement = input();
    movement.events = vec![egui::Event::PointerMoved(end)];
    let dragging = ctx.run(movement, |ctx| app.routing_panel(ctx, false));
    assert!(dragging.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::epaint::Shape::Text(text) if text.galley.job.text.starts_with("Dragging: "))));
    let mut release = input();
    release.events = vec![egui::Event::PointerButton {
        pos: end,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    }];
    let _ = ctx.run(release, |ctx| app.routing_panel(ctx, false));
    assert!(egui::DragAndDrop::payload::<i32>(&ctx).is_none());
}

fn plugin_action(app: &mut App, index: usize) -> String {
    use eframe::egui;
    app.favorites.show = false;
    app.filter = app.plugins[index].name.clone();
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 600.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| app.library_panel(ctx));
    let frame = ctx.run(input(), |ctx| app.library_panel(ctx));
    let action = frame
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if ["Load", "Add", "Remove"].contains(&text.galley.job.text.as_str()) =>
            {
                Some(text.galley.job.text.clone())
            }
            _ => None,
        })
        .expect("plugin action not rendered");
    app.filter.clear();
    action
}
