//! Bounded independent panes; the results own the remaining window area.
use super::*;

impl Browser {
    pub(super) fn panes(&mut self, ui: &mut egui::Ui, mut changed: bool) {
        if self.snapshot.is_none() {
            return;
        }
        let area = ui.available_rect_before_wrap();
        let gap = ui.spacing().item_spacing.x;
        let left_width = (area.width() * 0.24).clamp(112.0, 240.0);
        let left = egui::Rect::from_min_size(area.min, egui::vec2(left_width, area.height()));
        let right = egui::Rect::from_min_max(egui::pos2(left.max.x + gap, area.min.y), area.max);
        let role_height = (area.height() * 0.42).min(210.0);
        let role = egui::Rect::from_min_size(left.min, egui::vec2(left.width(), role_height));
        let preset = egui::Rect::from_min_max(egui::pos2(left.min.x, role.max.y + gap), left.max);
        pane(ui, role, "Role", |ui| changed |= self.roles(ui));
        pane(ui, preset, "Preset", |ui| changed |= self.presets(ui));
        if changed {
            self.filter(true);
        }
        pane(ui, right, "Patches", |ui| self.rows(ui));
        #[cfg(test)]
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new("browser_panes"), [role, preset, right])
        });
        ui.advance_cursor_after_rect(area);
    }
    pub(super) fn roles(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        egui::ScrollArea::vertical()
            .id_salt("patch_roles")
            .max_height(ui.available_height())
            .show(ui, |ui| {
                for (index, role) in FilterGroup::ALL.iter().enumerate() {
                    if ui
                        .selectable_label(self.role == index, role.label())
                        .clicked()
                        && self.role != index
                    {
                        self.role = index;
                        self.preset = 0;
                        changed = true;
                    }
                }
            });
        changed
    }

    pub(super) fn presets(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let mut delete = None;
        let output = egui::ScrollArea::vertical()
            .id_salt("patch_presets")
            .max_height(ui.available_height())
            .show(ui, |ui| {
                for (index, preset) in self.snapshot.as_ref().unwrap().presets[self.role]
                    .iter()
                    .enumerate()
                {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.preset == index, &preset.label)
                            .clicked()
                            && self.preset != index
                        {
                            self.preset = index;
                            changed = true;
                        }
                        if preset.is_user && ui.small_button("Delete").clicked() {
                            delete = Some((
                                preset.group.role().unwrap().key().to_owned(),
                                preset.pattern.clone().unwrap(),
                            ));
                        }
                    });
                }
            });
        #[cfg(test)]
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::new("preset_scroll"),
                (
                    output.inner_rect,
                    output.state.offset.y,
                    output.content_size.y,
                ),
            )
        });
        #[cfg(not(test))]
        let _ = output;
        if let Some(delete) = delete {
            let store = self.store.as_mut().unwrap();
            let presets = store
                .presets
                .iter()
                .filter(|preset| **preset != delete)
                .cloned()
                .collect();
            if store.replace(presets) {
                self.rebuild_conditions();
            }
        }
        changed
    }
}

fn pane(ui: &mut egui::Ui, rect: egui::Rect, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width((rect.width() - 16.0).max(1.0));
            ui.set_height((rect.height() - 16.0).max(1.0));
            ui.heading(title);
            add(ui);
        });
    });
}
