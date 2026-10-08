use super::*;

#[test]
fn library_icon_clicks_work_with_thumbnails_and_placeholders_and_respect_disabled_state() {
    for thumbnail in [false, true] {
        for enabled in [false, true] {
            let ctx = egui::Context::default();
            let key = PluginKey {
                format: "CLAP".into(),
                id: "test-synth".into(),
                name: "Synth".into(),
                vendor: String::new(),
                bundle_path: String::new(),
            };
            let mut icons = PluginIcons::default();
            let handle = thumbnail.then(|| {
                texture(
                    &ctx,
                    &key,
                    &image::RgbaImage::from_pixel(30, 20, image::Rgba([80, 160, 240, 255])),
                )
            });
            icons.textures.insert(identity(&key), handle);
            let input = || egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 200.0),
                )),
                ..Default::default()
            };
            let mut position = egui::Pos2::ZERO;
            let mut clicks = 0;
            {
                let mut draw = |ctx: &egui::Context| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = ui
                            .add_enabled_ui(enabled, |ui| {
                                icons.button(ui, &key, &Err("unused".into()))
                            })
                            .inner;
                        position = response.rect.center();
                        clicks += usize::from(response.clicked());
                    });
                };
                let _ = ctx.run(input(), &mut draw);
                let _ = ctx.run(input(), &mut draw);
            }
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
                let _ = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        clicks += usize::from(
                            ui.add_enabled_ui(enabled, |ui| {
                                icons.button(ui, &key, &Err("unused".into()))
                            })
                            .inner
                            .clicked(),
                        );
                    });
                });
            }
            assert_eq!(
                clicks,
                usize::from(enabled),
                "thumbnail={thumbnail}, enabled={enabled}"
            );
        }
    }
}
