//! Hybrid frequency axis: logarithmic below 1 kHz, linear above it.
use crate::spectrum::{Display, FFT_SIZE, FLOOR_DB};
use eframe::egui;

const MIN_HZ: f32 = 20.0;
const SPLIT_HZ: f32 = 1_000.0;
const MAX_HZ: f32 = 20_000.0;

pub fn panel(ui: &mut egui::Ui, frame: Option<&Display>, has_audio: bool, plot_height: f32) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Spectrum | Output");
        ui.colored_label(egui::Color32::LIGHT_GREEN, "L");
        ui.colored_label(egui::Color32::LIGHT_BLUE, "R");
    });
    ui.label(if frame.is_some() {
        "Semilog | Log <= 1 kHz / Linear >= 1 kHz"
    } else if has_audio {
        "Waiting for audio..."
    } else {
        "Load an instrument with audio to view its spectrum"
    });
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), plot_height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(15, 20, 24));
    let plot = egui::Rect::from_min_max(
        rect.min + egui::vec2(36.0, 8.0),
        rect.max - egui::vec2(12.0, 23.0),
    );
    if !plot.is_positive() {
        return;
    }
    let max_hz = frame.map_or(MAX_HZ, |f| MAX_HZ.min(f.sample_rate as f32 / 2.0));
    if max_hz <= MIN_HZ {
        return;
    }
    let x = |hz| plot.left() + frequency_position(hz, max_hz) * plot.width();
    let y =
        |db: f32| plot.bottom() - (db.clamp(FLOOR_DB, 0.0) - FLOOR_DB) / -FLOOR_DB * plot.height();
    let grid = egui::Stroke::new(1.0_f32, egui::Color32::from_gray(45));
    let font = egui::FontId::proportional(10.0);
    for db in [0, -20, -40, -60, -80, -100] {
        painter.line_segment(
            [
                egui::pos2(plot.left(), y(db as f32)),
                egui::pos2(plot.right(), y(db as f32)),
            ],
            grid,
        );
        painter.text(
            egui::pos2(plot.left() - 4.0, y(db as f32)),
            egui::Align2::RIGHT_CENTER,
            db.to_string(),
            font.clone(),
            egui::Color32::GRAY,
        );
    }
    painter.text(
        rect.min + egui::vec2(2.0, 2.0),
        egui::Align2::LEFT_TOP,
        "dBFS",
        font.clone(),
        egui::Color32::GRAY,
    );
    let mut last_label = f32::NEG_INFINITY;
    for (hz, label) in [
        (20.0, "20"),
        (50.0, "50"),
        (100.0, "100"),
        (200.0, "200"),
        (500.0, "500"),
        (1_000.0, "1k"),
        (5_000.0, "5k"),
        (10_000.0, "10k"),
        (15_000.0, "15k"),
        (20_000.0, "20k"),
    ] {
        if hz > max_hz {
            continue;
        }
        let position = x(hz);
        painter.line_segment(
            [
                egui::pos2(position, plot.top()),
                egui::pos2(position, plot.bottom()),
            ],
            grid,
        );
        if position - last_label >= 28.0 {
            painter.text(
                egui::pos2(position, plot.bottom() + 3.0),
                egui::Align2::CENTER_TOP,
                label,
                font.clone(),
                egui::Color32::GRAY,
            );
            last_label = position;
        }
    }
    painter.text(
        egui::pos2(plot.right(), rect.bottom()),
        egui::Align2::RIGHT_BOTTOM,
        "Hz",
        font,
        egui::Color32::GRAY,
    );
    if let Some(frame) = frame {
        let painter = painter.with_clip_rect(plot);
        for (channel, color) in [egui::Color32::LIGHT_GREEN, egui::Color32::LIGHT_BLUE]
            .into_iter()
            .enumerate()
        {
            // Keep the largest bin per screen column so narrow harmonics remain
            // visible when the high-frequency linear region compresses bins.
            let mut points: Vec<egui::Pos2> = Vec::new();
            let mut previous_column = None;
            for (i, bin) in frame.bins.iter().enumerate().skip(1) {
                let hz = i as f32 * frame.sample_rate as f32 / FFT_SIZE as f32;
                if hz < MIN_HZ || hz > max_hz {
                    continue;
                }
                let position = egui::pos2(x(hz), y(bin[channel]));
                let column = position.x.floor() as i32;
                if previous_column == Some(column) {
                    let last = points.last_mut().unwrap();
                    last.y = last.y.min(position.y);
                } else {
                    points.push(position);
                    previous_column = Some(column);
                }
            }
            painter.add(egui::Shape::line(points, egui::Stroke::new(1.0_f32, color)));
        }
    }
}

fn frequency_position(hz: f32, max_hz: f32) -> f32 {
    if max_hz <= SPLIT_HZ {
        return (hz / MIN_HZ).ln() / (max_hz / MIN_HZ).ln();
    }
    if hz <= SPLIT_HZ {
        0.5 * (hz / MIN_HZ).ln() / (SPLIT_HZ / MIN_HZ).ln()
    } else {
        0.5 + 0.5 * (hz - SPLIT_HZ) / (max_hz - SPLIT_HZ)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_axis_has_logarithmic_bass_and_linear_treble() {
        assert_eq!(frequency_position(MIN_HZ, MAX_HZ), 0.0);
        assert_eq!(frequency_position(SPLIT_HZ, MAX_HZ), 0.5);
        assert_eq!(frequency_position(MAX_HZ, MAX_HZ), 1.0);
        let low_a = frequency_position(100.0, MAX_HZ) - frequency_position(50.0, MAX_HZ);
        let low_b = frequency_position(200.0, MAX_HZ) - frequency_position(100.0, MAX_HZ);
        assert!((low_a - low_b).abs() < 1e-6);
        let high_a = frequency_position(10_000.0, MAX_HZ) - frequency_position(5_000.0, MAX_HZ);
        let high_b = frequency_position(15_000.0, MAX_HZ) - frequency_position(10_000.0, MAX_HZ);
        assert!((high_a - high_b).abs() < 1e-6);
        assert!(frequency_position(999.0, MAX_HZ) < 0.5);
        assert!(frequency_position(1_001.0, MAX_HZ) > 0.5);
    }
}
