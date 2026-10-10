//! Independent effect browser viewport, laid out like the patch browser.
use crate::{
    effect_browser::EffectBrowser,
    patch_browser_ui::{editing_text, navigation, pane},
    App,
};
use eframe::egui::{self, Color32, Key};

/// What the window needs from the rest of the app, read before it is drawn.
pub(crate) struct View<'a> {
    /// `(id, label)` of every connected effect, in chain order.
    pub slots: Vec<(i32, String)>,
    /// Plugins the target slot cannot use because another slot holds them.
    pub taken: Vec<String>,
    pub loading: bool,
    pub busy: bool,
    pub status: &'a str,
    /// Success text, so words in a preset name are not read as failures.
    pub success: Option<String>,
}

impl App {
    pub(crate) fn effect_browser_ui(&mut self, ctx: &egui::Context) {
        if !self.effect_browser.open {
            return;
        }
        let target = self.effect_browser.target;
        let slots: Vec<_> = self
            .instances
            .iter()
            .filter(|instance| instance.kind == crate::plugin_list::PluginKind::Effect)
            .map(|instance| (instance.id, instance.label.clone()))
            .collect();
        let taken = self
            .effect_browser
            .candidates
            .iter()
            .map(|candidate| candidate.plugin_id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|id| self.effect_plugin_taken(id, target))
            .collect();
        let view = View {
            slots,
            taken,
            loading: self.random_effect_catalog.loading(),
            busy: self.effect_browser.desired.is_some() || self.random_effect.busy(),
            status: &self.status,
            success: self
                .effect_browser
                .applied_in_target()
                .map(|applied| format!("Browse effect: {}", applied.preset.display)),
        };
        let mut builder = egui::ViewportBuilder::default()
            .with_title("Browse effects")
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([360.0, 420.0])
            .with_maximized(true);
        if self.effect_browser.window_position.is_none() {
            self.effect_browser.window_position =
                ctx.input(|input| input.viewport().outer_rect.map(|rect| rect.min));
        }
        if let Some(position) = self.effect_browser.window_position {
            builder = builder.with_position(position);
        }
        let browser = &mut self.effect_browser;
        ctx.show_viewport_immediate(viewport(), builder, |ctx, _class| {
            browser.show(ctx, &view);
            // The live Host dispatcher remains in the root main-thread update.
            ctx.request_repaint_after_for(
                std::time::Duration::from_millis(50),
                egui::ViewportId::ROOT,
            );
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        });
    }
}

impl EffectBrowser {
    fn show(&mut self, ctx: &egui::Context, view: &View) {
        if ctx.input(|input| input.viewport().close_requested()) {
            self.close();
            return;
        }
        let editing_before = editing_text(ctx);
        status_panel(ctx, view);
        egui::CentralPanel::default().show(ctx, |ui| {
            if view.loading {
                ui.label("Preparing effect catalog...");
            } else if self.candidates.is_empty() {
                ui.label("No installed CLAP effect presets were found.");
            } else {
                self.toolbar(ui, view);
                self.panes(ui, view);
            }
        });
        if !editing_before && !editing_text(ctx) {
            self.keys(ctx);
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            self.close();
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, view: &View) {
        let mut changed = false;
        ui.horizontal(|ui| {
            let height = 24.0;
            let target_width = 220.0_f32.min(ui.available_width() * 0.35);
            let random_width = 62.0;
            let search_width = (ui.available_width()
                - target_width
                - random_width
                - 2.0 * ui.spacing().item_spacing.x)
                .max(1.0);
            changed = ui
                .add_sized(
                    [search_width, height],
                    egui::TextEdit::singleline(&mut self.query)
                        .id(egui::Id::new("effect_browser_search"))
                        .hint_text("Search: hall -plate / surge")
                        .desired_width(search_width),
                )
                .changed();
            let label = |target: Option<i32>| {
                target
                    .and_then(|id| {
                        let at = view.slots.iter().position(|(slot, _)| *slot == id)?;
                        Some(format!("Replace {}: {}", at + 1, view.slots[at].1))
                    })
                    .unwrap_or_else(|| "Add new effect".into())
            };
            egui::ComboBox::from_id_salt("effect_browser_target")
                .width(target_width)
                .selected_text(label(self.target))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.target, None, label(None));
                    for &(id, _) in &view.slots {
                        ui.selectable_value(&mut self.target, Some(id), label(Some(id)));
                    }
                })
                .response
                .on_hover_text("Slot the selected preset is loaded into");
            if ui
                .add_enabled_ui(!self.results.is_empty(), |ui| {
                    ui.add_sized([random_width, height], egui::Button::new("Random"))
                })
                .inner
                .on_hover_text("Choose a random effect from the filtered results (R)")
                .clicked()
            {
                self.select_random();
            }
        });
        if changed {
            self.filter(true);
        }
        if let Some(error) = &self.error {
            ui.colored_label(Color32::YELLOW, format!("Invalid search: {error}"));
        }
    }

    fn panes(&mut self, ui: &mut egui::Ui, view: &View) {
        let area = ui.available_rect_before_wrap();
        let gap = ui.spacing().item_spacing.x;
        let left_width = (area.width() * 0.24).clamp(112.0, 240.0);
        let left = egui::Rect::from_min_size(area.min, egui::vec2(left_width, area.height()));
        let right = egui::Rect::from_min_max(egui::pos2(left.max.x + gap, area.min.y), area.max);
        let category_height = (area.height() * 0.42).min(210.0);
        let category =
            egui::Rect::from_min_size(left.min, egui::vec2(left.width(), category_height));
        let kind = egui::Rect::from_min_max(egui::pos2(left.min.x, category.max.y + gap), left.max);
        let mut chosen = None;
        pane(ui, category, "Category", |ui| {
            chosen = choice(ui, "effect_categories", &self.categories, self.category);
        });
        if let Some(index) = chosen {
            self.choose_category(index);
        }
        let mut chosen = None;
        pane(ui, kind, "Kind", |ui| {
            chosen = choice(ui, "effect_kinds", &self.kinds, self.kind);
        });
        if let Some(index) = chosen {
            self.choose_kind(index);
        }
        pane(ui, right, "Effects", |ui| self.rows(ui, view));
        ui.advance_cursor_after_rect(area);
    }

    fn keys(&mut self, ctx: &egui::Context) {
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
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Slash)) {
            // The typed "/" arrives in the same frame; the field gains focus only afterwards.
            ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("effect_browser_search")));
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
            }
        }
    }
}

/// A list pane of names; returns the clicked index when it differs from `current`.
fn choice(ui: &mut egui::Ui, id: &str, names: &[String], current: usize) -> Option<usize> {
    let mut chosen = None;
    egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(ui.available_height())
        .show(ui, |ui| {
            for (index, name) in names.iter().enumerate() {
                if ui.selectable_label(current == index, name).clicked() && current != index {
                    chosen = Some(index);
                }
            }
        });
    chosen
}

/// Loading progress and failures; ordinary success needs no footer.
fn status_panel(ctx: &egui::Context, view: &View) {
    let progress = ["Preparing effect:", "Loading effect:"]
        .iter()
        .any(|prefix| view.status.starts_with(prefix));
    let message = view
        .success
        .as_deref()
        .and_then(|success| view.status.strip_prefix(success))
        .unwrap_or(view.status)
        .to_ascii_lowercase();
    let warning = !progress
        && ["failed", "unavailable", "error", "no audio"]
            .iter()
            .any(|term| message.contains(term));
    if !view.busy && !warning {
        return;
    }
    egui::TopBottomPanel::bottom("effect_browser_status").show(ctx, |ui| {
        let text = if warning {
            egui::RichText::new(view.status).color(Color32::YELLOW)
        } else {
            egui::RichText::new(view.status)
        };
        ui.add(egui::Label::new(text).truncate())
            .on_hover_text(view.status);
    });
}

fn viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("effect_browser_window")
}

mod rows;
#[cfg(test)]
mod tests;
