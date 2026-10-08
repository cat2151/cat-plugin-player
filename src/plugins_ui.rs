//! Plugin rows represent hidden VST3 connections without changing load identities.
use crate::{
    config::PluginKey, ffi::PluginInfo, plugin_list, plugin_list::PluginKind, App, Instance,
};
use eframe::egui;

pub(super) enum RowAction {
    Load,
    Remove(i32),
}

struct PluginRow<'a> {
    plugin: &'a PluginInfo,
    connected: Vec<&'a Instance>,
}

impl<'a> PluginRow<'a> {
    fn new(
        plugin: &'a PluginInfo,
        catalog: &[PluginInfo],
        instances: &'a [Instance],
        prefer_clap: bool,
    ) -> Self {
        let connected = instances
            .iter()
            .filter(|instance| {
                instance.kind == plugin.kind
                    && ((instance.plugin.format == plugin.format
                        && instance.plugin.id == plugin.id)
                        || (prefer_clap
                            && instance.plugin.find(catalog).is_some_and(|index| {
                                plugin_list::represents_hidden_vst3(plugin, &catalog[index])
                            })))
            })
            .collect();
        Self { plugin, connected }
    }

    fn remove_targets(&self) -> Vec<&Instance> {
        if self.plugin.kind == PluginKind::Effect {
            self.connected.clone()
        } else {
            Vec::new()
        }
    }

    fn draw(
        &self,
        ui: &mut egui::Ui,
        enabled: bool,
        has_effect: bool,
        icon: impl FnOnce(&mut egui::Ui) -> egui::Response,
    ) -> Option<RowAction> {
        let targets = self.remove_targets();
        let action = if !targets.is_empty() {
            "Remove"
        } else if self.plugin.kind == PluginKind::Effect && has_effect {
            "Add"
        } else {
            "Load"
        };
        let mut result = None;
        let single_target = targets.first().filter(|_| targets.len() == 1);
        let tooltip = if let Some(target) = single_target {
            format!(
                "Remove {} [{}]\n{}",
                target.plugin.name, target.plugin.format, target.plugin.id
            )
        } else if targets.len() > 1 {
            "Choose a format's Remove button".into()
        } else {
            format!(
                "{action} {} [{}]\n{}",
                self.plugin.name, self.plugin.format, self.plugin.id
            )
        };
        let disabled_reason = if self.plugin.kind == PluginKind::Effect && !enabled {
            "Load an instrument first, or wait for loading or scanning to finish"
        } else {
            "Wait for loading or scanning to finish"
        };
        super::favorites_ui::library_row(ui, !self.connected.is_empty(), enabled, |ui| {
            let mut clicked = ui
                .add_enabled_ui(enabled, icon)
                .inner
                .on_hover_text(&tooltip)
                .on_disabled_hover_text(disabled_reason)
                .clicked();
            if targets.len() > 1 {
                for target in &targets {
                    if ui
                        .add_enabled(
                            enabled,
                            egui::Button::new(format!("Remove {}", target.plugin.format)),
                        )
                        .on_hover_text(format!(
                            "Remove {} [{}]\n{}",
                            target.plugin.name, target.plugin.format, target.plugin.id
                        ))
                        .on_disabled_hover_text(disabled_reason)
                        .clicked()
                    {
                        result = Some(RowAction::Remove(target.id));
                    }
                }
            } else {
                clicked |= ui
                    .add_enabled(enabled, egui::Button::new(action))
                    .on_hover_text(&tooltip)
                    .on_disabled_hover_text(disabled_reason)
                    .clicked();
            }
            clicked |= ui
                .add_enabled(
                    enabled,
                    egui::Button::new(
                        egui::RichText::new(format!(
                            "{} - {} [{}]",
                            self.plugin.name, self.plugin.vendor, self.plugin.format
                        ))
                        .color(self.plugin.kind.color(ui.visuals().dark_mode)),
                    )
                    .frame(false)
                    .truncate(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(&tooltip)
                .on_disabled_hover_text(disabled_reason)
                .clicked();
            if self
                .connected
                .iter()
                .any(|instance| instance.plugin.format != self.plugin.format)
            {
                let formats: std::collections::BTreeSet<_> = self
                    .connected
                    .iter()
                    .map(|instance| instance.plugin.format.as_str())
                    .collect();
                ui.label(format!(
                    "In use: {}",
                    formats.into_iter().collect::<Vec<_>>().join(", ")
                ));
            }
            if clicked && targets.len() <= 1 {
                result = Some(
                    single_target.map_or(RowAction::Load, |target| RowAction::Remove(target.id)),
                );
            }
        });
        result
    }
}

impl App {
    pub(crate) fn plugins_list(
        &mut self,
        ui: &mut egui::Ui,
        filter: &str,
        can_load: bool,
    ) -> (Option<usize>, Option<i32>) {
        let mut load = None;
        let mut remove = None;
        let has_instrument = self.instrument_id().is_some();
        let has_effect = self.effect_id().is_some();
        for i in
            plugin_list::visible_indices(&self.plugins, self.restored.as_ref(), self.prefer_clap)
        {
            let plugin = &self.plugins[i];
            if !plugin.name.to_lowercase().contains(filter) {
                continue;
            }
            let row = PluginRow::new(plugin, &self.plugins, &self.instances, self.prefer_clap);
            let key = PluginKey::from_plugin(plugin);
            let enabled = can_load && (plugin.kind != PluginKind::Effect || has_instrument);
            let action = ui
                .push_id((plugin.format.as_str(), plugin.id.as_str()), |ui| {
                    row.draw(ui, enabled, has_effect, |ui| {
                        self.plugin_icons.button(ui, &key, &self.config_path)
                    })
                })
                .inner;
            match action {
                Some(RowAction::Load) => load = Some(i),
                Some(RowAction::Remove(id)) => remove = Some(id),
                None => {}
            }
        }
        (load, remove)
    }
}

#[cfg(test)]
#[path = "plugins_ui_tests.rs"]
mod tests;
