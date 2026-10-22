//! Independent browser viewport. Native work is requested by identity, never by a painted row.
use crate::{patch_browser::Browser, App};
use cmrt_patch_select::{
    plugin_menu::{plugin_mode, toggle_plugin_term, PluginMenu, PluginMode},
    FilterGroup,
};
use eframe::egui::{self, Color32, Key};

pub(crate) use row_colors::RowTint;

const PINK: Color32 = Color32::from_rgb(255, 135, 190);

impl App {
    pub(crate) fn browser_ui(&mut self, ctx: &egui::Context) {
        if self.selecting_master() {
            self.patch_browser.open = false;
            return;
        }
        self.update_browser_snapshot(ctx);
        self.poll_main_random();
        let browser = &mut self.patch_browser;
        if browser.open && browser.reveal_pending && browser.snapshot.is_some() {
            browser.reveal_pending = false;
            browser.reveal_playing();
        }
        if self.patch_browser.open {
            let focus = self.take_browser_focus(crate::cli::Browser::Patch);
            let mut builder = egui::ViewportBuilder::default()
                .with_title("Browse patches")
                .with_inner_size([1100.0, 750.0])
                .with_min_inner_size([360.0, 420.0])
                .with_maximized(true);
            if focus {
                builder = builder.with_active(true);
            }
            if self.patch_browser.window_position.is_none() {
                self.patch_browser.window_position =
                    ctx.input(|input| input.viewport().outer_rect.map(|rect| rect.min));
            }
            if let Some(position) = self.patch_browser.window_position {
                builder = builder.with_position(position);
            }
            let selecting = self.selection.is_some();
            // Esc in the heavy-load confirmation belongs to the dialog, not the browser.
            let exits = (selecting || self.resident.is_some())
                && self.patch_browser.requests.confirmation.is_none();
            let selection_error = selecting.then(|| self.config_error.clone()).flatten();
            let browser_only = self
                .selection
                .as_ref()
                .is_some_and(|s| s.browser_only.is_some());
            let mut action = None;
            let mut exit_selection = false;
            ctx.show_viewport_immediate(browser_viewport(), builder, |ctx, _class| {
                self.catalog_startup.observe_activity(ctx);
                crate::help_overlay::show(ctx, crate::help_overlay::Page::Patch);
                if focus {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                crate::selection::show_error(ctx, selection_error.as_deref());
                action = crate::selection_ui::toolbar(ctx, browser_only, false);
                if exits && crate::selection::exit_key(ctx) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    exit_selection = true;
                    return;
                }
                self.patch_browser
                    .show(ctx, self.patch_catalog.notice(), &self.status);
                // The live Host dispatcher remains in the root main-thread update.
                // Child focus and worker progress must keep that update awake.
                ctx.request_repaint_after_for(
                    std::time::Duration::from_millis(50),
                    egui::ViewportId::ROOT,
                );
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            });
            if exit_selection {
                self.browser_exit_requested();
            }
            self.selection_browser_closed(crate::cli::Browser::Patch);
            self.selection_action(action, ctx);
            if std::mem::take(&mut self.patch_browser.favorite_requested) {
                if let Some(id) = self.instrument_id() {
                    self.add_favorite(id);
                }
            }
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
            crate::japanese_font::add(ctx);
            let response =
                egui::Modal::new(egui::Id::new("heavy_patch_confirmation")).show(ctx, |ui| {
                    ui.set_max_width((ctx.screen_rect().width() - 48.0).max(200.0));
                    ui.heading("Confirm sample load");
                    ui.label(format!(
                        "{} — {}",
                        ticket.candidate.plugin_name, ticket.candidate.display
                    ));
                    ui.colored_label(PINK, size(ticket.candidate.measurement.sfz_sample_bytes));
                    ui.label("この音色はロードに10秒以上あるいは1分以上かかる可能性があります");
                    ui.label("ENTER : ロードして演奏する");
                    ui.label("ESC : 演奏しない");
                    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Enter)) {
                        browser.requests.enter();
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
        if crate::playback_ui::take_favorite_shortcut(ctx, true) {
            self.count = None;
            self.favorite_requested = true;
            return;
        }
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

    pub(crate) fn select_random(&mut self) {
        if self.error.is_some() {
            return;
        }
        if let Some(index) =
            crate::patch_preparation::choose(self.results.iter().copied(), &mut rand::thread_rng())
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
