//! Independent browser viewport. Native work is requested by identity, never by a painted row.
use crate::{patch_browser::Browser, App};
use cmrt_patch_select::{
    plugin_menu::{plugin_mode, toggle_plugin_term, PluginMode},
    FilterGroup,
};
use eframe::egui::{self, Color32, Key};

pub(crate) use row_colors::RowTint;

const PINK: Color32 = Color32::from_rgb(255, 135, 190);

impl App {
    pub(crate) fn browser_ui(&mut self, ctx: &egui::Context) {
        self.update_browser_snapshot(ctx);
        let browser = &mut self.patch_browser;
        if browser.open && browser.reveal_pending && browser.snapshot.is_some() {
            browser.reveal_pending = false;
            browser.reveal_playing();
        }
        if self.patch_browser.open {
            let mut builder = egui::ViewportBuilder::default()
                .with_title("Browse patches")
                .with_inner_size([1100.0, 750.0])
                .with_min_inner_size([360.0, 420.0])
                .with_maximized(true);
            if self.patch_browser.window_position.is_none() {
                self.patch_browser.window_position =
                    ctx.input(|input| input.viewport().outer_rect.map(|rect| rect.min));
            }
            if let Some(position) = self.patch_browser.window_position {
                builder = builder.with_position(position);
            }
            ctx.show_viewport_immediate(browser_viewport(), builder, |ctx, _class| {
                self.patch_browser
                    .show(ctx, self.random_patch_catalog.notice(), &self.status);
                // The live Host dispatcher remains in the root main-thread update.
                // Child focus and worker progress must keep that update awake.
                ctx.request_repaint_after_for(
                    std::time::Duration::from_millis(50),
                    egui::ViewportId::ROOT,
                );
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            });
        }
    }
}

impl Browser {
    fn show(&mut self, ctx: &egui::Context, notice: Option<&str>, status: &str) {
        if !self.open {
            return;
        }
        if ctx.input(|input| input.viewport().close_requested()) {
            self.close();
            return;
        }
        let editing_before = editing_text(ctx);
        let browser = self;
        let confirming = browser.requests.confirmation.is_some();
        let heavy_progress = browser
            .requests
            .preparing
            .as_ref()
            .or(browser.requests.applying.as_ref())
            .is_some_and(|ticket| ticket.candidate.measurement.is_heavy_offline_load());
        let child =
            confirming || heavy_progress || browser.solo_draft.is_some() || browser.condition_menu;
        browser.body(ctx, notice, status, !child);
        if browser.solo_draft.is_some() {
            browser.solo_overlay(ctx);
            return;
        }
        if browser.condition_menu {
            browser.conditions_menu(ctx);
            return;
        }
        if confirming {
            let ticket = browser.requests.confirmation.as_ref().unwrap().clone();
            let response =
                egui::Modal::new(egui::Id::new("heavy_patch_confirmation")).show(ctx, |ui| {
                    ui.set_max_width((ctx.screen_rect().width() - 48.0).max(200.0));
                    ui.heading("Confirm sample load");
                    ui.label(format!(
                        "{} — {}",
                        ticket.candidate.plugin_name, ticket.candidate.display
                    ));
                    ui.colored_label(PINK, size(ticket.candidate.measurement.sfz_sample_bytes));
                    ui.label(
                        "Press Enter to prepare and apply. Esc cancels without changing sound.",
                    );
                    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Enter)) {
                        browser.requests.enter();
                    }
                    if ui.button("Cancel (Esc)").clicked() {
                        browser.requests.confirmation = None;
                    }
                });
            if response.should_close()
                || ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape))
            {
                browser.requests.confirmation = None;
            }
            return;
        } else if heavy_progress {
            egui::Modal::new(egui::Id::new("heavy_patch_progress")).show(ctx, |ui| {
                ui.heading("Loading confirmed patch");
                ui.spinner();
                ui.label(status);
                ui.label(
                    "This operation cannot be canceled. The browser returns when it finishes.",
                );
            });
            return;
        }
        if !browser.condition_menu
            && browser.solo_draft.is_none()
            && !editing_before
            && !editing_text(ctx)
        {
            browser.keys(ctx);
        }
        if !browser.condition_menu
            && browser.solo_draft.is_none()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape))
        {
            browser.close();
        }
    }
}

impl Browser {
    fn body(&mut self, ctx: &egui::Context, notice: Option<&str>, status: &str, enabled: bool) {
        self.status_panel(ctx, status);
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(notice) = notice {
                ui.label(notice);
            } else if let Some(error) = &self.build_error {
                ui.colored_label(Color32::YELLOW, error);
            } else if self.snapshot.is_none() {
                ui.label("Preparing Role / Preset catalog...");
            } else {
                ui.add_enabled_ui(enabled, |ui| self.controls(ui));
            }
        });
    }

    fn rebuild_conditions(&mut self) {
        self.snapshot = None;
        self.results.clear();
        self.selected = None;
        self.requests.invalidate();
        self.build_error = None;
    }

    fn keys(&mut self, ctx: &egui::Context) {
        if self.snapshot.is_none() {
            return;
        }
        let count = self.count;
        if let Some(count) = ctx.input_mut(|input| navigation::consume_count_digit(input, count)) {
            self.count = Some(count);
            return;
        }
        if ctx.input(|input| {
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Key { pressed: true, .. }))
        }) {
            self.count = None;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Enter)) {
            self.close();
            return;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::M)) {
            self.solo_draft = Some(self.query.clone());
            ctx.request_repaint();
            return;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Slash)) {
            // The typed "/" arrives in the same frame; the field gains focus only afterwards.
            ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("patch_browser_search")));
            return;
        }
        if self.error.is_some() {
            return;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::R)) {
            self.select_random();
        }
        if let Some((step, times)) = ctx.input_mut(navigation::consume_step) {
            if !self.results.is_empty() {
                let at = self
                    .selected
                    .and_then(|index| self.results.iter().position(|&row| row == index));
                let times = times.saturating_mul(count.unwrap_or(1));
                let next =
                    navigation::next_by(at, self.results.len(), self.list_columns, step, times);
                self.select(self.results[next]);
                self.scroll_to_selected = true;
            }
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Space)) {
            self.requests.space();
        }
    }

    fn select_random(&mut self) {
        if self.error.is_some() {
            return;
        }
        if let Some(index) =
            crate::random_patch::choose(self.results.iter().copied(), &mut rand::thread_rng())
        {
            self.select(index);
        }
    }
}

fn size(bytes: Option<u64>) -> String {
    bytes.map_or_else(
        || "Size: unmeasured".into(),
        |bytes| format!("Size: {:.1} MB samples", bytes as f64 / 1_000_000.0),
    )
}

pub(crate) fn editing_text(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| memory.focused())
        .is_some_and(|id| egui::text_edit::TextEditState::load(ctx, id).is_some())
}

fn compact_label(ui: &mut egui::Ui, text: String) {
    ui.add(egui::Label::new(&text).truncate())
        .on_hover_text(text);
}

fn browser_viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("patch_browser_window")
}

mod controls;
mod layout;
pub(crate) use layout::pane;
pub(crate) mod navigation;
pub(crate) mod row_colors;
mod rows;
mod status;
#[cfg(test)]
mod tests;
