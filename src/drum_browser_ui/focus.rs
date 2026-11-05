//! The active pane and its cursor breathe together; the other selection stays still.
use super::*;

fn pulse(ui: &egui::Ui) -> f32 {
    ui.input(|i| (0.5 + 0.5 * (i.time * std::f64::consts::TAU / 1.2).sin()) as f32)
}

fn stroke(ui: &egui::Ui) -> egui::Stroke {
    egui::Stroke::new(
        2.0_f32,
        ui.visuals()
            .selection
            .stroke
            .color
            .gamma_multiply(0.55 + 0.45 * pulse(ui)),
    )
}

pub(super) fn cursor(ui: &egui::Ui, rect: egui::Rect) {
    let fill = ui
        .visuals()
        .selection
        .bg_fill
        .lerp_to_gamma(ui.visuals().selection.stroke.color, 0.1 + 0.25 * pulse(ui));
    ui.painter().rect_filled(rect, 2.0, fill);
    ui.painter()
        .rect_stroke(rect, 2.0, stroke(ui), egui::StrokeKind::Inside);
}

pub(super) fn pane(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    title: &str,
    focused: bool,
    add: impl FnOnce(&mut egui::Ui),
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        let mut frame = egui::Frame::group(ui.style());
        if focused {
            frame.stroke = stroke(ui);
        }
        frame.show(ui, |ui| {
            ui.set_width((rect.width() - 16.0).max(1.0));
            ui.set_height((rect.height() - 16.0).max(1.0));
            ui.heading(title);
            add(ui);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glowing_rects(focused: usize, time: f64) -> Vec<(egui::Rect, egui::Stroke)> {
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0, 400.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    for (index, title) in ["Kits", "Notes"].iter().enumerate() {
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(10.0 + index as f32 * 340.0, 10.0),
                            egui::vec2(330.0, 370.0),
                        );
                        pane(ui, rect, title, focused == index, |ui| {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(250.0, 24.0), egui::Sense::hover());
                            if focused == index {
                                cursor(ui, rect);
                            }
                        });
                    }
                });
            },
        );
        fn collect(shape: &egui::Shape, result: &mut Vec<(egui::Rect, egui::Stroke)>) {
            match shape {
                egui::Shape::Rect(r) if r.stroke.width == 2.0 => result.push((r.rect, r.stroke)),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, result);
                    }
                }
                _ => {}
            }
        }
        let mut result = Vec::new();
        for shape in output.shapes {
            collect(&shape.shape, &mut result);
        }
        result
    }

    #[test]
    fn only_active_pane_and_cursor_glow_together_and_follow_focus() {
        for focused in 0..2 {
            let before = glowing_rects(focused, 0.0);
            let after = glowing_rects(focused, 0.3);
            assert_eq!(before.len(), 2);
            assert_eq!(after.len(), 2);
            assert_eq!(before[0].1, before[1].1);
            assert_eq!(after[0].1, after[1].1);
            assert_ne!(before[0].1.color, after[0].1.color);
            assert!(before
                .iter()
                .all(|(rect, _)| (rect.center().x > 340.0) == (focused == 1)));
        }
    }
}
