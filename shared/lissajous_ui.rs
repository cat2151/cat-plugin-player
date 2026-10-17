//! Stereo vectorscope rotated 45 degrees with shared peak normalization.
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
            "45° rotated | Auto scale"
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
    response.on_hover_text("Lissajous | 45-degree stereo vectorscope | Shared auto scale\nSame phase: vertical; opposite phase: horizontal\nLeft only: upper-left diagonal; right only: upper-right diagonal");
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(15, 20, 24));
    let side = (rect.width().min(rect.height()) - 24.0).max(0.0);
    if side == 0.0 {
        return;
    }
    let plot = egui::Rect::from_center_size(rect.center(), egui::vec2(side, side));
    let grid = egui::Stroke::new(1.0_f32, egui::Color32::from_gray(45));
    let grid_point = |stereo| {
        let [x, y] = rotated_point(stereo);
        plot.center() + egui::vec2(x, -y) * side * 0.5
    };
    for fraction in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        painter.line_segment(
            [grid_point([fraction, -1.0]), grid_point([fraction, 1.0])],
            grid,
        );
        painter.line_segment(
            [grid_point([-1.0, fraction]), grid_point([1.0, fraction])],
            grid,
        );
    }
    painter.line_segment([plot.center_top(), plot.center_bottom()], grid);
    painter.line_segment([plot.left_center(), plot.right_center()], grid);
    let font = egui::FontId::proportional(10.0);
    painter.text(
        grid_point([1.0, 0.0]) + egui::vec2(-3.0, -3.0),
        egui::Align2::RIGHT_BOTTOM,
        "L",
        font.clone(),
        egui::Color32::LIGHT_GREEN,
    );
    painter.text(
        grid_point([0.0, 1.0]) + egui::vec2(3.0, -3.0),
        egui::Align2::LEFT_BOTTOM,
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
            rotated_point(s.map(|v| {
                if peak > 0.0 && v.is_finite() {
                    v / peak
                } else {
                    0.0
                }
            }))
        })
        .collect()
}

fn rotated_point([left, right]: [f32; 2]) -> [f32; 2] {
    // A 45-degree rotation, uniformly reduced to fit the original plot bounds.
    [(right - left) * 0.5, (left + right) * 0.5]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_phase_and_balance_survive_auto_scale() {
        let mono = normalized_points(&[[-0.01, -0.01], [0.01, 0.01]]);
        assert_eq!(mono, vec![[0.0, -1.0], [0.0, 1.0]]);
        let anti_phase = normalized_points(&[[-0.01, 0.01], [0.01, -0.01]]);
        assert_eq!(anti_phase, vec![[1.0, 0.0], [-1.0, 0.0]]);
        let opposite = normalized_points(&[[-2.0, 1.0], [2.0, -1.0]]);
        assert_eq!(opposite, vec![[0.75, -0.25], [-0.75, 0.25]]);
        assert_eq!(normalized_points(&[[1.0, 0.0]]), vec![[-0.5, 0.5]]);
        assert_eq!(normalized_points(&[[0.0, 1.0]]), vec![[0.5, 0.5]]);
        let quadrature = normalized_points(&[[0.0, 0.1], [0.1, 0.0], [0.0, -0.1], [-0.1, 0.0]]);
        assert!(quadrature.iter().all(|[x, y]| x * x + y * y == 0.5));
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
