//! Two bounded scrolling panes in the player's usual independent browser viewport.
use crate::{
    drum_browser::{identity, notes, same, Audition, Focus},
    App,
};
use eframe::egui::{self, Key, Modifiers};

mod feedback;
mod focus;
mod keyboard;
mod layout;
mod rows;
#[cfg(test)]
mod tests;

pub(crate) fn viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("drum_kit_browser")
}

impl App {
    pub(crate) fn drum_browser_ui(&mut self, ctx: &egui::Context) {
        if !self.drum_browser.open {
            return;
        }
        self.sync_drum_catalog();
        self.start_opening_drum_audition();
        let focus = crate::command_palette::take_focus(ctx, crate::command_palette::Screen::Drum);
        let commands = crate::command_palette::commands(self, crate::command_palette::Screen::Drum);
        let allowed = crate::command_palette::allowed(self, crate::command_palette::Screen::Drum);
        ctx.show_viewport_immediate(
            viewport(),
            egui::ViewportBuilder::default()
                .with_title("Drum kits")
                .with_inner_size([layout::INITIAL_WIDTH, 750.0])
                .with_min_inner_size([360.0, 300.0]),
            |ctx, _| {
                if focus {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                crate::japanese_font::add(ctx);
                let input = crate::command_palette::begin(
                    ctx,
                    commands.clone(),
                    allowed && !crate::help_overlay::is_open(ctx),
                );
                // A search owns all text, including '?' and shortcut letters.
                if self.drum_browser.editing.is_none() {
                    crate::help_overlay::show(ctx, crate::help_overlay::Page::Drum);
                }
                self.draw_drum_browser(ctx);
                crate::command_palette::end(ctx, input);
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
                ctx.request_repaint_after_for(
                    std::time::Duration::from_millis(50),
                    egui::ViewportId::ROOT,
                );
            },
        );
    }

    pub(crate) fn draw_drum_browser(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) {
            self.close_drum_browser();
            return;
        }
        self.drum_keyboard(ctx);
        if !self.drum_browser.open {
            return;
        }
        egui::TopBottomPanel::top("drum_search").show(ctx, |ui| {
            self.drum_feedback(ui);
            let mut compare = self.drum_browser.compare_velocity;
            if ui
                .checkbox(&mut compare, "Vel")
                .on_hover_ui(|ui| {
                    crate::command_palette::shortcut_hint(ui, "Velocity", "V");
                })
                .changed()
            {
                self.toggle_drum_velocity();
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("{} kits", self.drum_browser.results.len()));
                if self.drum_browser.editing.is_some() {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.drum_browser.saved.query)
                            .id_salt("drum_query")
                            .desired_width(ui.available_width()),
                    );
                    response.request_focus();
                    if response.changed() {
                        self.filter_drums();
                    }
                } else {
                    ui.label(format!("/ {}", self.drum_browser.saved.query));
                }
            });
        });
        egui::TopBottomPanel::bottom("drum_status").show(ctx, |ui| {
            if let Some(error) = &self.drum_browser.error {
                ui.colored_label(egui::Color32::YELLOW, error);
            }
            if let Some(notice) = self.patch_catalog.drum_notice() {
                ui.label(notice);
            }
            ui.add(egui::Label::new(&self.status).truncate())
                .on_hover_text(&self.status);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.fit_drum_window(ui);
            let area = ui.available_rect_before_wrap();
            let gap = ui.spacing().item_spacing.x;
            let width = self.drum_left_width(ui, (area.width() - gap).max(1.0));
            let left = egui::Rect::from_min_size(area.min, egui::vec2(width, area.height()));
            let right =
                egui::Rect::from_min_max(egui::pos2(left.max.x + gap, area.min.y), area.max);
            let titles = if self.drum_browser.saved.focus == Focus::Kits {
                ["▶ Kits", "Notes"]
            } else {
                ["Kits", "▶ Notes"]
            };
            focus::pane(
                ui,
                left,
                titles[0],
                self.drum_browser.saved.focus == Focus::Kits,
                |ui| self.drum_kit_rows(ui),
            );
            focus::pane(
                ui,
                right,
                titles[1],
                self.drum_browser.saved.focus == Focus::Notes,
                |ui| self.drum_note_rows(ui),
            );
            #[cfg(test)]
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("drum_panes"), [left, right]));
        });
        self.drum_browser.scroll = false;
    }

    fn filter_drums(&mut self) {
        let previous = self.drum_browser.selected;
        self.drum_browser.filter();
        if previous != self.drum_browser.selected {
            self.stop_drum_preview();
            if self
                .drum_browser
                .selected()
                .is_some_and(|c| !c.measurement.is_heavy_offline_load())
            {
                self.request_drum_audition(Audition::All, false);
            }
        }
        self.save_drum_cursor();
    }
}
