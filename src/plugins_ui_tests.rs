use super::*;

fn plugin(format: &str, kind: PluginKind) -> PluginInfo {
    PluginInfo {
        index: 0,
        name: "Synth".into(),
        vendor: "Vendor".into(),
        format: format.into(),
        id: format!("synth-{format}"),
        bundle_path: String::new(),
        kind,
    }
}

fn instance(id: i32, plugin: &PluginInfo) -> Instance {
    Instance {
        id,
        label: plugin.name.clone(),
        plugin: PluginKey::from_plugin(plugin),
        kind: plugin.kind,
        voice: None,
        sweep_cc1: false,
    }
}

fn click(row: &PluginRow<'_>, label: &str) -> Option<RowAction> {
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 200.0),
        )),
        ..Default::default()
    };
    let draw = |ui: &mut egui::Ui| row.draw(ui, true, true, |ui| ui.button("Icon"));
    let _ = ctx.run(input(), |ctx| {
        egui::CentralPanel::default().show(ctx, draw);
    });
    let frame = ctx.run(input(), |ctx| {
        egui::CentralPanel::default().show(ctx, draw);
    });
    if row.connected.iter().any(|i| i.plugin.format == "VST3") {
        assert!(frame.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text.starts_with("In use:"))));
    }
    let position = frame
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .expect("click target not rendered");
    let mut action = None;
    for pressed in [true, false] {
        let mut raw = input();
        raw.events = vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ];
        let _ = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                action = draw(ui).or(action.take());
            });
        });
    }
    action
}

#[test]
fn hidden_vst3_effect_is_selected_and_click_removes_its_actual_instance() {
    let catalog = [
        plugin("CLAP", PluginKind::Effect),
        plugin("VST3", PluginKind::Effect),
    ];
    let connected = [instance(42, &catalog[1])];
    let row = PluginRow::new(&catalog[0], &catalog, &connected, true);
    assert_eq!(row.connected.len(), 1);
    for target in ["Synth - Vendor [CLAP]", "Icon", "Remove"] {
        assert!(matches!(click(&row, target), Some(RowAction::Remove(42))));
    }
    let empty = PluginRow::new(&catalog[0], &catalog, &[], true);
    assert!(matches!(click(&empty, "Add"), Some(RowAction::Load)));
}

#[test]
fn instrument_stays_a_clap_load_and_dual_effect_buttons_choose_one_instance() {
    let catalog = [
        plugin("CLAP", PluginKind::Instrument),
        plugin("VST3", PluginKind::Instrument),
    ];
    let connected = [instance(42, &catalog[1])];
    let row = PluginRow::new(&catalog[0], &catalog, &connected, true);
    assert_eq!(row.connected.len(), 1);
    assert!(matches!(
        click(&row, "Synth - Vendor [CLAP]"),
        Some(RowAction::Load)
    ));
    let catalog = [
        plugin("CLAP", PluginKind::Effect),
        plugin("VST3", PluginKind::Effect),
    ];
    let connected = [instance(17, &catalog[0]), instance(42, &catalog[1])];
    let row = PluginRow::new(&catalog[0], &catalog, &connected, true);
    assert!(matches!(
        click(&row, "Remove CLAP"),
        Some(RowAction::Remove(17))
    ));
    assert!(matches!(
        click(&row, "Remove VST3"),
        Some(RowAction::Remove(42))
    ));
    assert!(click(&row, "Synth - Vendor [CLAP]").is_none());
}

#[test]
fn preference_off_and_unrelated_or_missing_metadata_do_not_alias_connections() {
    let catalog = [
        plugin("CLAP", PluginKind::Effect),
        plugin("VST3", PluginKind::Effect),
    ];
    let connected = [instance(42, &catalog[1])];
    assert!(PluginRow::new(&catalog[0], &catalog, &connected, false)
        .connected
        .is_empty());
    for change in ["vendor", "name", "kind", "empty", "format"] {
        let mut other = catalog.clone();
        match change {
            "vendor" => other[1].vendor = "Other".into(),
            "name" => other[1].name = "Other".into(),
            "kind" => other[1].kind = PluginKind::Instrument,
            "empty" => other[0].vendor.clear(),
            "format" => other[1].format = "AU".into(),
            _ => unreachable!(),
        }
        let connected = [instance(42, &other[1])];
        assert!(
            PluginRow::new(&other[0], &other, &connected, true)
                .connected
                .is_empty(),
            "{change}"
        );
    }
    let mut normalized = catalog.clone();
    normalized[1].name = " synth ".into();
    normalized[1].vendor = " vendor ".into();
    let connected = [instance(42, &normalized[1])];
    assert_eq!(
        PluginRow::new(&normalized[0], &normalized, &connected, true)
            .connected
            .len(),
        1
    );
}
