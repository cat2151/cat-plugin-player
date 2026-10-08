//! Compact stereo correlation bar beneath the Lissajous plot.
use eframe::egui;

pub fn panel(ui: &mut egui::Ui, correlation: Option<f32>, show_labels: bool) {
    if show_labels {
        ui.horizontal(|ui| {
            ui.label("Correlation");
            ui.monospace(correlation.map_or_else(|| "—".into(), |v| format!("{v:+.2}")));
        });
    }
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::hover());
    response.on_hover_text("Stereo correlation | 150 ms average\n+1: same phase / 0: weak correlation / -1: opposite phase\nNegative values indicate cancellation when mixed to mono.\n—: no audio or either channel is silent");
    let painter = ui.painter_at(rect);
    if rect.width() <= 2.0 {
        return;
    }
    let bar = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.top() + 10.0));
    let middle = bar.center().x;
    painter.rect_filled(bar, 2.0, egui::Color32::from_rgb(15, 20, 24));
    if let Some(value) = correlation {
        let x = middle + value.clamp(-1.0, 1.0) * bar.width() * 0.5;
        let color = if value < 0.0 {
            egui::Color32::LIGHT_RED
        } else {
            egui::Color32::LIGHT_GREEN
        };
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(middle.min(x), bar.top()),
                egui::pos2(middle.max(x), bar.bottom()),
            ),
            0.0,
            color,
        );
        let marker = x.clamp(bar.left() + 1.0, bar.right() - 1.0);
        painter.line_segment(
            [
                egui::pos2(marker, bar.top()),
                egui::pos2(marker, bar.bottom()),
            ],
            egui::Stroke::new(2.0_f32, color),
        );
    }
    painter.line_segment(
        [
            egui::pos2(middle, bar.top()),
            egui::pos2(middle, bar.bottom()),
        ],
        egui::Stroke::new(1.0_f32, egui::Color32::GRAY),
    );
    for (x, text, align) in [
        (bar.left(), "-1", egui::Align2::LEFT_TOP),
        (middle, "0", egui::Align2::CENTER_TOP),
        (bar.right(), "+1", egui::Align2::RIGHT_TOP),
    ] {
        painter.text(
            egui::pos2(x, bar.bottom() + 2.0),
            align,
            text,
            egui::FontId::proportional(10.0),
            egui::Color32::GRAY,
        );
    }
}
