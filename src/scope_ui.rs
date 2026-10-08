//! Oscilloscope controls and stereo drawing.
use crate::{scope, scope_trigger::Trigger, App};
use eframe::egui;

pub struct ScopeUi {
    cycles: u8,
    trigger: Trigger,
    guides: bool,
    show_labels: bool,
    pub(crate) on_right: bool,
}

impl Default for ScopeUi {
    fn default() -> Self {
        Self {
            cycles: 4,
            trigger: Trigger::Similarity,
            guides: false,
            show_labels: false,
            on_right: crate::status::Status::default().on_right,
        }
    }
}

impl ScopeUi {
    pub(crate) fn with_on_right(on_right: bool) -> Self {
        Self {
            on_right,
            ..Default::default()
        }
    }
}

impl App {
    pub(crate) fn scope_panel(&mut self, ctx: &egui::Context, on_right: bool) {
        if !on_right {
            egui::TopBottomPanel::bottom("audio_analysis_bottom")
                .show(ctx, |ui| self.analysis_contents(ui, ctx, false));
        }
    }

    pub(crate) fn analysis_contents(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        on_right: bool,
    ) {
        if self.scope_ui.show_labels {
            ui.heading("Audio analysis");
        }
        let scope = self
            .instances
            .iter()
            .find_map(|instance| instance.voice.as_ref().map(|voice| voice.scope()));
        let spectrum = scope.and_then(|scope| scope.spectrum());
        let lissajous = scope.and_then(|scope| scope.lissajous());
        let has_audio = scope.is_some();
        if on_right {
            crate::spectrum_ui::panel(
                ui,
                spectrum.as_deref(),
                has_audio,
                100.0,
                self.scope_ui.show_labels,
            );
            ui.separator();
            self.scope_contents(ui, ctx, 100.0);
            ui.separator();
            crate::lissajous_ui::panel(
                ui,
                lissajous.as_deref(),
                has_audio,
                150.0,
                self.scope_ui.show_labels,
            );
            crate::correlation_ui::panel(
                ui,
                lissajous.as_ref().and_then(|f| f.correlation),
                self.scope_ui.show_labels,
            );
        } else {
            ui.columns(3, |columns| {
                crate::spectrum_ui::panel(
                    &mut columns[0],
                    spectrum.as_deref(),
                    has_audio,
                    180.0,
                    self.scope_ui.show_labels,
                );
                self.scope_contents(&mut columns[1], ctx, 180.0);
                crate::lissajous_ui::panel(
                    &mut columns[2],
                    lissajous.as_deref(),
                    has_audio,
                    180.0,
                    self.scope_ui.show_labels,
                );
                crate::correlation_ui::panel(
                    &mut columns[2],
                    lissajous.as_ref().and_then(|f| f.correlation),
                    self.scope_ui.show_labels,
                );
            });
        }
        if has_audio {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    fn scope_contents(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, plot_height: f32) {
        ui.horizontal_wrapped(|ui| {
            if self.scope_ui.show_labels {
                ui.label("Oscilloscope | Output");
                ui.colored_label(egui::Color32::LIGHT_GREEN, "L");
                ui.colored_label(egui::Color32::LIGHT_BLUE, "R");
            }
            ui.menu_button("☰", |ui| {
                ui.checkbox(&mut self.scope_ui.show_labels, "Show analysis labels");
                if ui.checkbox(&mut self.scope_ui.on_right, "On right")
                    .on_hover_text(
                        "Move spectrum, oscilloscope and Lissajous between the right sidebar and bottom",
                    ).changed() {
                    self.save_session();
                }
                ui.separator();
                ui.label("Display cycles");
                ui.horizontal(|ui| {
                    for cycles in [1, 2, 4, 8] {
                        ui.selectable_value(
                            &mut self.scope_ui.cycles,
                            cycles,
                            format!("{cycles} cycles"),
                        );
                    }
                });
                ui.label("Trigger");
                egui::ComboBox::from_id_salt("scope_trigger")
                    .selected_text(match self.scope_ui.trigger {
                        Trigger::ZeroCross => "Zero cross",
                        Trigger::Similarity => "Similarity",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.scope_ui.trigger,
                            Trigger::ZeroCross,
                            "Zero cross",
                        );
                        ui.selectable_value(
                            &mut self.scope_ui.trigger,
                            Trigger::Similarity,
                            "Similarity",
                        );
                    });
                ui.separator();
                ui.checkbox(&mut self.scope_ui.guides, "Show red guides");
            })
            .response
            .on_hover_text("Analysis display settings");
        });
        let scope = self
            .instances
            .iter()
            .find_map(|instance| instance.voice.as_ref().map(|voice| voice.scope()));
        let frame = scope.and_then(|scope| {
            scope.configure(self.scope_ui.cycles, self.scope_ui.trigger);
            scope.display().filter(|f| {
                f.settings == scope::settings(self.scope_ui.cycles, self.scope_ui.trigger)
            })
        });
        if let Some(frame) = &frame {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(1.0 / 60.0));
            if self.scope_ui.show_labels {
                ui.label(format!(
                    "Note {} | {:.2} ms",
                    frame.note,
                    1000.0 * frame.samples.len() as f64 / f64::from(frame.sample_rate),
                ));
            }
        } else if self.scope_ui.show_labels {
            ui.label(if scope.is_some() {
                "Waiting for note and waveform..."
            } else {
                "Load an instrument with audio to view its waveform"
            });
        }
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), plot_height),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(15, 20, 24));
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            ],
            egui::Stroke::new(1.0_f32, egui::Color32::DARK_GRAY),
        );
        if let Some(frame) = frame {
            let peak = display_peak(&frame.samples);
            let x = |j: usize| {
                rect.left()
                    + rect.width()
                        * (j as f64 / (frame.samples.len() - 1) as f64
                            + frame.shift_samples / frame.samples.len() as f64)
                            as f32
            };
            for (ch, color) in [egui::Color32::LIGHT_GREEN, egui::Color32::LIGHT_BLUE]
                .into_iter()
                .enumerate()
            {
                let points = frame
                    .samples
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        egui::pos2(
                            x(i),
                            rect.center().y
                                - normalized_amplitude(s[ch], peak) * rect.height() * 0.4,
                        )
                    })
                    .collect();
                painter.add(egui::Shape::line(points, egui::Stroke::new(1.0_f32, color)));
            }
            if self.scope_ui.guides {
                for j in &frame.markers {
                    painter.line_segment(
                        [
                            egui::pos2(x(*j), rect.top()),
                            egui::pos2(x(*j), rect.bottom()),
                        ],
                        egui::Stroke::new(1.0_f32, egui::Color32::LIGHT_RED),
                    );
                }
            }
        }
    }
}

// Use one peak for the displayed window so stereo balance is preserved.
fn display_peak(samples: &[[f32; 2]]) -> f32 {
    samples
        .iter()
        .flatten()
        .copied()
        .filter(|sample| sample.is_finite())
        .map(f32::abs)
        .fold(0.0, f32::max)
}

fn normalized_amplitude(sample: f32, peak: f32) -> f32 {
    if peak > 0.0 && sample.is_finite() {
        sample / peak
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_stereo_uses_shared_peak_and_preserves_balance() {
        let samples = [[-0.01, 0.005], [0.01, -0.0025]];
        let peak = display_peak(&samples);
        assert_eq!(normalized_amplitude(samples[0][0], peak), -1.0);
        assert_eq!(normalized_amplitude(samples[1][0], peak), 1.0);
        assert_eq!(normalized_amplitude(samples[0][1], peak), 0.5);
        assert_eq!(normalized_amplitude(samples[1][1], peak), -0.25);
    }

    #[test]
    fn silence_and_non_finite_samples_stay_on_center_line() {
        let samples = [[0.0, f32::NAN], [f32::INFINITY, f32::NEG_INFINITY]];
        let peak = display_peak(&samples);
        assert_eq!(peak, 0.0);
        for sample in samples.iter().flatten() {
            assert_eq!(normalized_amplitude(*sample, peak), 0.0);
        }
    }

    #[test]
    fn over_full_scale_and_tiny_peaks_are_normalized_without_clipping() {
        for peak in [2.0, f32::MIN_POSITIVE / 2.0] {
            let samples = [[-peak, peak / 2.0], [peak, f32::NAN]];
            let measured = display_peak(&samples);
            assert_eq!(normalized_amplitude(-peak, measured), -1.0);
            assert_eq!(normalized_amplitude(peak / 2.0, measured), 0.5);
            assert_eq!(normalized_amplitude(f32::NAN, measured), 0.0);
        }
    }
}
