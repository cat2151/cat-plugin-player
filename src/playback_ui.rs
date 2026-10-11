//! Responsive placement of sequence and browser controls.
use crate::App;
use eframe::egui;

const SEQUENCE_MIN_WIDTH: f32 = 520.0;
const BROWSE_WIDTH: f32 = 150.0;

impl App {
    /// A browser window takes the keys the main window would otherwise handle.
    pub(crate) fn browser_open(&self) -> bool {
        self.patch_browser.open || self.effect_browser.open
    }

    /// Single-letter shortcuts of the main window; off while a child window or text field takes keys.
    pub(crate) fn main_shortcut(&self, ctx: &egui::Context, key: egui::Key) -> bool {
        let editing_text = ctx
            .memory(|memory| memory.focused())
            .is_some_and(|id| egui::text_edit::TextEditState::load(ctx, id).is_some());
        !self.mml_input.open
            && !self.browser_open()
            && !editing_text
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key))
    }

    pub(crate) fn playback_controls(&mut self, ui: &mut egui::Ui) {
        if self.main_shortcut(ui.ctx(), egui::Key::T) {
            self.open_patch_browser();
            ui.ctx().request_repaint();
        }
        if self.main_shortcut(ui.ctx(), egui::Key::E) || self.main_shortcut(ui.ctx(), egui::Key::X)
        {
            self.open_effect_browser();
            ui.ctx().request_repaint();
        }
        let spacing = ui.spacing().item_spacing.x;
        let sequence_width = ui.available_width() - BROWSE_WIDTH - spacing;
        if sequence_width >= SEQUENCE_MIN_WIDTH {
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(sequence_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.sequence_controls(ui),
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(BROWSE_WIDTH, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.browse_controls(ui),
                );
            });
        } else {
            self.sequence_controls(ui);
            self.browse_controls(ui);
        }
    }

    fn browse_controls(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            if ui
                .add_enabled(
                    !self.mml_input.open && !self.patch_browser.open,
                    egui::Button::new("Browse patches"),
                )
                .on_hover_text("Open patch browser (T)")
                .clicked()
            {
                self.open_patch_browser();
            }
            if ui
                .add_enabled(
                    !self.mml_input.open && !self.effect_browser.open,
                    egui::Button::new("Browse effects"),
                )
                .on_hover_text("Open effect browser (E / X)")
                .clicked()
            {
                self.open_effect_browser();
            }
        });
    }
}
