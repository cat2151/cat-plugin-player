//! Stereo XY plot with equal axis scales and shared peak normalization.
use crate::lissajous::Display;
use eframe::egui;

pub fn panel(
    ui: &mut egui::Ui,
    frame: Option<&Display>,
    has_audio: bool,
    plot_height: f32,
    show_labels: bool,
) {
    if show_labels {
        ui.label("Lissajous | Output");
        ui.label(if frame.is_some() {
            "X: L / Y: R | Auto scale"
        } else if has_audio {
            "Waiting for audio..."
        } else {
            "Load an instrument with audio to view its stereo image"
        });
    }
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), plot_height),
        egui::Sense::hover(),
    );
    response.on_hover_text("Lissajous | X: left / Y: right | Shared auto scale\nSame phase: rising diagonal; opposite phase: falling diagonal");
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(15, 20, 24));
    let side = (rect.width().min(rect.height()) - 24.0).max(0.0);
    if side == 0.0 {
        return;
    }
    let plot = egui::Rect::from_center_size(rect.center(), egui::vec2(side, side));
    let grid = egui::Stroke::new(1.0_f32, egui::Color32::from_gray(45));
    for fraction in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let offset = fraction * side * 0.5;
        painter.line_segment(
            [
                egui::pos2(plot.center().x + offset, plot.top()),
                egui::pos2(plot.center().x + offset, plot.bottom()),
            ],
            grid,
        );
        painter.line_segment(
            [
                egui::pos2(plot.left(), plot.center().y + offset),
                egui::pos2(plot.right(), plot.center().y + offset),
            ],
            grid,
        );
    }
    let font = egui::FontId::proportional(10.0);
    painter.text(
        egui::pos2(plot.right() + 3.0, plot.center().y),
        egui::Align2::LEFT_CENTER,
        "L",
        font.clone(),
        egui::Color32::LIGHT_GREEN,
    );
    painter.text(
        egui::pos2(plot.center().x, plot.top() - 2.0),
        egui::Align2::CENTER_BOTTOM,
        "R",
        font,
        egui::Color32::LIGHT_BLUE,
    );
    if let Some(frame) = frame {
        let points = normalized_points(&frame.samples)
            .into_iter()
            .map(|[x, y]| plot.center() + egui::vec2(x, -y) * side * 0.46)
            .collect::<Vec<_>>();
        if let Some(point) = points.last() {
            painter.circle_filled(*point, 1.5, egui::Color32::LIGHT_GREEN);
        }
        if points.len() >= 2 {
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(1.0_f32, egui::Color32::LIGHT_GREEN),
            ));
        }
    }
}

fn normalized_points(samples: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let peak = samples
        .iter()
        .flatten()
        .copied()
        .filter(|v| v.is_finite())
        .map(f32::abs)
        .fold(0.0, f32::max);
    samples
        .iter()
        .map(|s| {
            s.map(|v| {
                if peak > 0.0 && v.is_finite() {
                    v / peak
                } else {
                    0.0
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_phase_and_balance_survive_auto_scale() {
        let mono = normalized_points(&[[-0.01, -0.01], [0.01, 0.01]]);
        assert_eq!(mono, vec![[-1.0, -1.0], [1.0, 1.0]]);
        let opposite = normalized_points(&[[-2.0, 1.0], [2.0, -1.0]]);
        assert_eq!(opposite, vec![[-1.0, 0.5], [1.0, -0.5]]);
        let quadrature = normalized_points(&[[0.0, 0.1], [0.1, 0.0], [0.0, -0.1], [-0.1, 0.0]]);
        assert!(quadrature.iter().all(|[x, y]| x * x + y * y == 1.0));
    }

    #[test]
    fn silence_and_invalid_audio_have_finite_coordinates() {
        assert_eq!(
            normalized_points(&[[0.0, f32::NAN], [f32::INFINITY, f32::NEG_INFINITY]]),
            vec![[0.0; 2]; 2]
        );
        assert!(normalized_points(&[]).is_empty());
    }
}
