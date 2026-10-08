//! Central favorite/plugin tabs and the favorite naming dialog.
use crate::{plugin_list::PluginKind, App};
use eframe::egui;

#[cfg(test)]
#[path = "library_ui_tests.rs"]
mod tests;

// Paint behind the whole row without making its menu or metadata clickable.
pub(super) fn library_row<R>(
    ui: &mut egui::Ui,
    selected: bool,
    enabled: bool,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let background = ui.painter().add(egui::Shape::Noop);
    let row = ui.horizontal(|ui| {
        ui.set_min_width(ui.available_width());
        contents(ui)
    });
    let hovered = enabled && row.response.contains_pointer();
    let fill = if selected {
        let fill = ui.visuals().selection.bg_fill;
        if hovered {
            fill.linear_multiply(1.2)
        } else {
            fill
        }
    } else if hovered {
        ui.visuals().widgets.hovered.weak_bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().set(
        background,
        egui::Shape::rect_filled(row.response.rect, 4.0, fill),
    );
    row
}

impl App {
    pub(crate) fn library_panel(&mut self, ctx: &egui::Context) {
        let mut load_plugin = None;
        let mut load_favorite = None;
        let mut remove_effect = None;
        let mut delete = None;
        let mut rename = None;
        egui::CentralPanel::default().show(ctx, |ui| {
            // Reserve a fixed gutter so the scrollbar cannot cover row actions.
            ui.style_mut().spacing.scroll = egui::style::ScrollStyle {
                bar_width: 10.0,
                ..egui::style::ScrollStyle::solid()
            };
            let previous_tab = self.favorites.show;
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.favorites.show, true, "Favorites");
                ui.selectable_value(&mut self.favorites.show, false, "Plugins");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button("☰", |ui| {
                        ui.label("Settings");
                        let mut prefer_clap = self.prefer_clap;
                        if ui.checkbox(&mut prefer_clap, "Show only CLAP for duplicate plugins")
                            .on_hover_text("Hide VST3 when CLAP has the same name, vendor and kind. Turn off to show both formats.")
                            .changed()
                        {
                            let result = self.config_path.as_ref().map_err(Clone::clone)
                                .and_then(|path| crate::config::save_prefer_clap(path, prefer_clap));
                            match result {
                                Ok(()) => self.prefer_clap = prefer_clap,
                                Err(error) => self.status = format!("Could not save plugin display setting: {error}"),
                            }
                        }
                        ui.separator();
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
                    .on_hover_text("Settings");
                });
            });
            if self.favorites.show != previous_tab {
                // A tab switch during startup must preserve slots still restoring.
                self.config_error = self.config_path.as_ref().map_err(Clone::clone)
                    .and_then(|path| {
                        let mut status = crate::status::Status::load(path)?;
                        status.show_favorites = Some(self.favorites.show);
                        status.save(path)
                    })
                    .err()
                    .map(|error| format!("Could not save library tab: {error}"));
            }
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .desired_width(ui.available_width()),
                );
            });
            if !self.status.is_empty() {
                ui.label(&self.status);
            }
            if let Some(error) = &self.plugin_icons.error {
                ui.colored_label(egui::Color32::YELLOW, error);
            }
            ui.separator();
            let filter = self.filter.to_lowercase();
            let can_load = self.pending.is_none() && !self.scanning && !self.restoring;
            let has_instrument = self.instrument_id().is_some();
            if self.favorites.show {
                if let Some(error) = &self.favorites.error {
                    ui.colored_label(egui::Color32::YELLOW, error);
                } else if self.favorites.library.entries.is_empty() {
                    ui.label("Use 'Add favorite' on a loaded plugin to save its current sound.");
                }
                let mut matches = 0;
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                    for favorite in &self.favorites.library.entries {
                        let description = format!("{} | {} | {}", favorite.plugin.name,
                            favorite.plugin.format, if favorite.effect { "Effect" } else { "Instrument" });
                        if !format!("{} {description}", favorite.name).to_lowercase().contains(&filter) { continue; }
                        matches += 1;
                        let selected = self.favorites.active.iter().any(|(instance, id)| id == &favorite.id
                            && self.instances.iter().any(|i| i.id == *instance));
                        let connected_id = self.favorites.active.iter().find(|(instance, id)| favorite.effect && id == &favorite.id && self.instances.iter().any(|i| i.id == *instance && i.kind == PluginKind::Effect)).map(|(instance, _)| *instance);
                        let connected_effect = connected_id.is_some();
                        let kind = if favorite.effect { PluginKind::Effect } else { PluginKind::Instrument };
                        let enabled = can_load && (connected_effect || !favorite.effect || has_instrument);
                        let action = if connected_effect {
                            "Click again to remove the connected effect. The instrument and saved favorite are kept."
                        } else {
                            "Restore this snapshot. Edited sounds do not overwrite it."
                        };
                        let tooltip = format!("{}\n{}\n{action}", favorite.name, description);
                        let disabled_reason = if !can_load { "Wait for loading or scanning to finish" } else { "Load an instrument first" };
                        let mut clicked = false;
                        ui.push_id(&favorite.id, |ui| {
                            library_row(ui, selected, enabled, |ui| {
                                clicked |= ui.add_enabled_ui(enabled, |ui| {
                                    self.plugin_icons.button(ui, &favorite.plugin, &self.config_path)
                                }).inner.on_hover_text(&tooltip).on_disabled_hover_text(disabled_reason).clicked();
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
                                    enabled,
                                    |ui| ui.add_sized(
                                        [ui.available_width(), ui.spacing().interact_size.y],
                                        egui::Button::new(&favorite.name).frame(false).truncate(),
                                    ),
                                ).inner;
                                clicked |= response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(&tooltip).on_disabled_hover_text(disabled_reason).clicked();
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
                        if clicked {
                            if connected_effect { remove_effect = connected_id; } else { load_favorite = Some(favorite.id.clone()); }
                        }
                    }
                    if matches == 0 && !self.favorites.library.entries.is_empty() { ui.label("No matching favorites"); }
                });
            } else {
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                    let (plugin, effect) = self.plugins_list(ui, &filter, can_load);
                    load_plugin = plugin;
                    remove_effect = effect;
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
