//! Responsive placement of sequence and random preset controls.
use crate::App;
use eframe::egui;

const SEQUENCE_MIN_WIDTH: f32 = 520.0;
const RANDOM_PATCH_WIDTH: f32 = 150.0;

impl App {
    /// Single-letter shortcuts of the main window; off while a child window or text field takes keys.
    pub(crate) fn main_shortcut(&self, ctx: &egui::Context, key: egui::Key) -> bool {
        let editing_text = ctx
            .memory(|memory| memory.focused())
            .is_some_and(|id| egui::text_edit::TextEditState::load(ctx, id).is_some());
        !self.mml_input.open
            && !self.patch_browser.open
            && !editing_text
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key))
    }

    pub(crate) fn playback_controls(&mut self, ui: &mut egui::Ui) {
        if self.main_shortcut(ui.ctx(), egui::Key::T) {
            self.patch_browser.open_window();
            ui.ctx().request_repaint();
        }
        let spacing = ui.spacing().item_spacing.x;
        let sequence_width = ui.available_width() - RANDOM_PATCH_WIDTH - spacing;
        let mut random = false;
        let mut effect = false;
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
                    |ui| (random, effect) = self.random_controls(ui),
                );
            });
        } else {
            self.sequence_controls(ui);
            (random, effect) = self.random_controls(ui);
        }
        if random {
            self.start_random_patch(ui.ctx());
        }
        if effect {
            self.start_random_effect(ui.ctx());
        }
    }

    fn random_controls(&mut self, ui: &mut egui::Ui) -> (bool, bool) {
        ui.group(|ui| {
            let patch = self.random_patch_catalog.button(ui, self.actions_busy());
            if ui.add_enabled(!self.mml_input.open && !self.patch_browser.open, egui::Button::new("Browse patches")).on_hover_text("Open patch browser (T)").clicked() {
                self.patch_browser.open_window();
            }
            let count = self.random_effect_catalog.installed(&self.plugins).count();
            let effect = ui.add_enabled(
                !self.actions_busy() && self.instrument_id().is_some() && count > 0,
                egui::Button::new("Random effect"),
            ).on_hover_text(format!(
                "Choose from {count} effect presets.\nReplace one randomly chosen connected effect; add the first effect when none is connected.\nKeeps effect order, bypass and playback selection."
            )).on_disabled_hover_text("Load an instrument and wait for scanning, catalog loading or preset preparation to finish. An eligible CLAP effect preset is required.").clicked();
            (patch, effect)
        })
            .inner
    }
}
