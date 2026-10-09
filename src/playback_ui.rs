//! Responsive placement of independent sequence and random patch panes.
use crate::App;
use eframe::egui;

const SEQUENCE_MIN_WIDTH: f32 = 520.0;
const RANDOM_PATCH_WIDTH: f32 = 150.0;

impl App {
    pub(crate) fn playback_controls(&mut self, ui: &mut egui::Ui) {
        if self.random_patch_catalog.candidates().next().is_none() {
            self.sequence_controls(ui);
            return;
        }

        let spacing = ui.spacing().item_spacing.x;
        let sequence_width = ui.available_width() - RANDOM_PATCH_WIDTH - spacing;
        let mut random = false;
        if sequence_width >= SEQUENCE_MIN_WIDTH {
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(sequence_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.sequence_controls(ui),
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(RANDOM_PATCH_WIDTH, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| random = self.random_patch_controls(ui),
                );
            });
        } else {
            self.sequence_controls(ui);
            random = self.random_patch_controls(ui);
        }
        if random {
            self.start_random_patch(ui.ctx());
        }
    }

    fn random_patch_controls(&self, ui: &mut egui::Ui) -> bool {
        ui.group(|ui| self.random_patch_catalog.button(ui, self.actions_busy()))
            .inner
    }
}
