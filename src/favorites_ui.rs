//! Central favorite/plugin tabs and the favorite naming dialog.
use crate::{
    plugin_list::{self, PluginKind},
    App,
};
use eframe::egui;

impl App {
    pub(crate) fn library_panel(&mut self, ctx: &egui::Context) {
        let mut load_plugin = None;
        let mut load_favorite = None;
        let mut remove_effect = None;
        let mut delete = None;
        let mut rename = None;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.favorites.show, true, "Favorites");
                ui.selectable_value(&mut self.favorites.show, false, "Plugins");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button("...", |ui| {
                        if ui
                            .add_enabled(
                                !self.scanning && self.pending.is_none() && !self.restoring,
                                egui::Button::new("Rescan plugins..."),
                            )
                            .clicked()
                        {
                            self.confirm_rescan = true;
                            ui.close_menu();
                        }
                    })
                    .response
                    .on_hover_text("Plugin list options");
                });
            });
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .desired_width(ui.available_width()),
                );
            });
            ui.label(&self.status);
            if let Some(error) = &self.plugin_icons.error {
                ui.colored_label(egui::Color32::YELLOW, error);
            }
            ui.separator();
            let filter = self.filter.to_lowercase();
            let can_load = self.pending.is_none() && !self.scanning && !self.restoring;
            let has_instrument = self.instrument_id().is_some();
            let effect_id = self.effect_id();
            if self.favorites.show {
                if let Some(error) = &self.favorites.error {
                    ui.colored_label(egui::Color32::YELLOW, error);
                } else if self.favorites.library.entries.is_empty() {
                    ui.label("Use 'Add favorite' on a loaded plugin to save its current sound.");
                }
                let mut matches = 0;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for favorite in &self.favorites.library.entries {
                        let description = format!("{} | {} | {}", favorite.plugin.name,
                            favorite.plugin.format, if favorite.effect { "Effect" } else { "Instrument" });
                        if !format!("{} {description}", favorite.name).to_lowercase().contains(&filter) { continue; }
                        matches += 1;
                        let selected = self.favorites.active.iter().any(|(instance, id)| id == &favorite.id
                            && self.instances.iter().any(|i| i.id == *instance));
                        let connected_effect = favorite.effect && self.favorites.active.iter().any(
                            |(instance, id)| id == &favorite.id && Some(*instance) == effect_id,
                        );
                        let kind = if favorite.effect { PluginKind::Effect } else { PluginKind::Instrument };
                        ui.push_id(&favorite.id, |ui| {
                            ui.horizontal(|ui| {
                                self.plugin_icons.draw(ui, &favorite.plugin, &self.config_path);
                                ui.colored_label(kind.color(ui.visuals().dark_mode), kind.label());
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                                egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.menu_button("...", |ui| {
                                    if ui.button("Rename").clicked() {
                                        rename = Some((favorite.id.clone(), favorite.name.clone()));
                                        ui.close_menu();
                                    }
                                    if ui.button("Delete").clicked() {
                                        delete = Some(favorite.id.clone());
                                        ui.close_menu();
                                    }
                                });
                                let width = ui.available_width();
                                let details_width = if width >= 300.0 { width * 0.35 } else { 0.0 };
                                if details_width > 0.0 {
                                    ui.add_sized(
                                        [details_width, ui.spacing().interact_size.y],
                                        egui::Label::new(egui::RichText::new(&description).small().weak()).truncate(),
                                    ).on_hover_text(&description);
                                }
                                let response = ui.add_enabled_ui(
                                    can_load && (connected_effect || !favorite.effect || has_instrument),
                                    |ui| ui.add_sized(
                                        [ui.available_width(), ui.spacing().interact_size.y],
                                        egui::Button::new(&favorite.name).selected(selected).frame(false).truncate(),
                                    ),
                                ).inner;
                                let action = if connected_effect {
                                    "Click again to remove the connected effect. The instrument and saved favorite are kept."
                                } else {
                                    "Restore this snapshot. Edited sounds do not overwrite it."
                                };
                                let tooltip = format!("{}\n{}\n{action}", favorite.name, description);
                                if response.on_hover_text(&tooltip).on_disabled_hover_text(&tooltip).clicked() {
                                    if connected_effect {
                                        remove_effect = effect_id;
                                    } else {
                                        load_favorite = Some(favorite.id.clone());
                                    }
                                }
                            });
                            });
                            if connected_effect {
                                ui.colored_label(kind.color(ui.visuals().dark_mode),
                                    if self.effect_bypassed {
                                        "Connected (bypassed) - click again to remove"
                                    } else {
                                        "Connected - click again to remove"
                                    });
                            }
                        });
                    }
                });
                if matches == 0 && !self.favorites.library.entries.is_empty() { ui.label("No matching favorites"); }
            } else {
                ui.horizontal(|ui| {
                    for kind in [PluginKind::Instrument, PluginKind::Effect, PluginKind::Unknown] {
                        ui.colored_label(kind.color(ui.visuals().dark_mode), kind.label());
                    }
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for i in plugin_list::ordered_indices(&self.plugins, self.restored.as_ref()) {
                        let plugin = &self.plugins[i];
                        if !plugin.name.to_lowercase().contains(&filter) { continue; }
                        ui.horizontal(|ui| {
                            self.plugin_icons.draw(ui, &crate::config::PluginKey::from_plugin(plugin), &self.config_path);
                            if ui.add_enabled(can_load && (plugin.kind != PluginKind::Effect || has_instrument),
                                egui::Button::new("Load")).on_disabled_hover_text("Load an instrument first").clicked() {
                                load_plugin = Some(i);
                            }
                            ui.colored_label(plugin.kind.color(ui.visuals().dark_mode),
                                format!("{} - {} [{}]", plugin.name, plugin.vendor, plugin.format)).on_hover_text(&plugin.id);
                        });
                    }
                });
            }
        });
        if let Some(i) = load_plugin {
            self.load_plugin(i);
        }
        if let Some(id) = load_favorite {
            self.load_favorite(&id);
        }
        if let Some(id) = remove_effect {
            self.remove_plugin(id);
        }
        if let Some(id) = delete {
            self.delete_favorite(&id);
        }
        if let Some(rename) = rename {
            self.favorites.rename = Some(rename);
        }
        self.favorite_name_dialog(ctx);
        self.rescan_confirmation(ctx);
    }

    fn rescan_confirmation(&mut self, ctx: &egui::Context) {
        if !self.confirm_rescan {
            return;
        }
        let mut confirm = false;
        let mut cancel = false;
        let response = egui::Modal::new(egui::Id::new("rescan_confirmation")).show(ctx, |ui| {
            ui.set_max_width(360.0);
            ui.heading("Rescan plugins?");
            ui.label("Refresh the installed plugin list. This may take some time.");
            ui.label("Plugins and favorites cannot be loaded while scanning.");
            ui.horizontal(|ui| {
                cancel = ui.button("Cancel").clicked();
                confirm = ui
                    .add_enabled(
                        !self.scanning && self.pending.is_none() && !self.restoring,
                        egui::Button::new("Rescan plugins"),
                    )
                    .clicked();
            });
        });
        if cancel || response.should_close() {
            self.confirm_rescan = false;
        } else if confirm {
            self.confirm_rescan = false;
            self.start_scan(true);
        }
    }

    fn favorite_name_dialog(&mut self, ctx: &egui::Context) {
        let Some((id, mut name)) = self.favorites.rename.clone() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        egui::Window::new("Rename favorite")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                let response = ui.text_edit_singleline(&mut name);
                save = response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !name.trim().is_empty();
                ui.horizontal(|ui| {
                    save |= ui
                        .add_enabled(!name.trim().is_empty(), egui::Button::new("Save"))
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        self.favorites.rename = Some((id.clone(), name.clone()));
        if !open || cancel || (save && self.rename_favorite(&id, &name)) {
            self.favorites.rename = None;
        }
    }
}
