//! Central favorite/plugin tabs and the favorite naming dialog.
use crate::{plugin_list::PluginKind, App};
use eframe::egui;

#[cfg(test)]
#[path = "library_ui_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "favorites_order_ui_tests.rs"]
mod order_tests;

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
        let mut move_favorite = None;
        let mut sort_favorites = false;
        egui::CentralPanel::default().show(ctx, |ui| {
            // Reserve a fixed gutter so the scrollbar cannot cover row actions.
            ui.style_mut().spacing.scroll = egui::style::ScrollStyle {
                bar_width: 10.0,
                ..egui::style::ScrollStyle::solid()
            };
            let previous_tab = (self.favorites.show, self.favorites.show_history);
            ui.horizontal(|ui| {
                if ui.selectable_label(self.favorites.show && !self.favorites.show_history, "Favorites").clicked() {
                    self.favorites.show = true;
                    self.favorites.show_history = false;
                }
                if ui.selectable_label(self.favorites.show_history, "History").clicked() {
                    self.favorites.show = true;
                    self.favorites.show_history = true;
                }
                if ui.selectable_label(!self.favorites.show, "Plugins").clicked() {
                    self.favorites.show = false;
                    self.favorites.show_history = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button("☰", |ui| {
                        ui.label("Settings");
                        let favorites_tab = self.favorites.show && !self.favorites.show_history;
                        ui.add_enabled_ui(favorites_tab, |ui| {
                            ui.checkbox(&mut self.favorites.reorder_mode, "Reorder favorites");
                        });
                        let can_reorder = favorites_tab && self.filter.is_empty()
                            && self.favorites.error.is_none() && !self.actions_busy()
                            && !self.scanning && !self.restoring;
                        if ui.add_enabled(can_reorder,
                            egui::Button::new("Sort by plugin name"))
                            .on_hover_text("Sort Favorites once in ascending plugin name order and save. Clear the filter first.")
                            .clicked() {
                            sort_favorites = true;
                            ui.close_menu();
                        }
                        ui.separator();
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
                                !self.scanning && !self.actions_busy() && !self.restoring,
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
            if (self.favorites.show, self.favorites.show_history) != previous_tab {
                // A tab switch during startup must preserve slots still restoring.
                self.config_error = self.config_path.as_ref().map_err(Clone::clone)
                    .and_then(|path| {
                        let mut status = crate::status::Status::load(path)?;
                        status.show_favorites = Some(self.favorites.show);
                        status.show_history = self.favorites.show_history;
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
            let can_load = !self.actions_busy() && !self.scanning && !self.restoring;
            let has_instrument = self.instrument_id().is_some();
            if self.favorites.show {
                if let Some(error) = &self.favorites.error {
                    ui.colored_label(egui::Color32::YELLOW, error);
                } else if !self.favorites.show_history && !self.favorites.library.entries.iter().any(|entry| entry.favorite) {
                    ui.label("Use 'Add favorite' on a loaded plugin to save its current sound.");
                }
                let history = self.favorites.show_history;
                let mut entries: Vec<_> = self.favorites.library.entries.iter()
                    .filter(|entry| if history { entry.history } else { entry.favorite }).collect();
                if history {
                    entries.sort_by_key(|entry| std::cmp::Reverse(entry.registered_at));
                    ctx.request_repaint_after(std::time::Duration::from_secs(1));
                    if entries.is_empty() { ui.label("Sounds are recorded here when switching plugins."); }
                }
                let mut matches = 0;
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                    for (position, favorite) in entries.iter().enumerate() {
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
                                if !history {
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
                                    if self.favorites.reorder_mode {
                                        let can_move = self.filter.is_empty() && can_load
                                            && self.favorites.error.is_none();
                                        if ui.add_enabled(can_move && position + 1 < entries.len(), egui::Button::new("Down"))
                                            .on_hover_text("Move down. Clear the filter first.").clicked() {
                                            move_favorite = Some((favorite.id.clone(), false));
                                        }
                                        if ui.add_enabled(can_move && position > 0, egui::Button::new("Up"))
                                            .on_hover_text("Move up. Clear the filter first.").clicked() {
                                            move_favorite = Some((favorite.id.clone(), true));
                                        }
                                    }
                                }
                                if history {
                                    ui.label(crate::history::age(favorite.registered_at, crate::history::now()))
                                        .on_hover_text("Time since last history registration");
                                }
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
                    if matches == 0 && !self.favorites.library.entries.is_empty() { ui.label(if history { "No matching history" } else { "No matching favorites" }); }
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
        if sort_favorites || move_favorite.is_some() {
            let result = self
                .config_path
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|path| {
                    if let Some((id, up)) = move_favorite {
                        self.favorites.library.move_favorite(path, &id, up)
                    } else {
                        self.favorites.library.sort_favorites_by_plugin(path)
                    }
                });
            self.status = match result {
                Ok(()) => "Favorite order saved".into(),
                Err(error) => format!("Could not save favorite order: {error}"),
            };
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
                        !self.scanning && !self.actions_busy() && !self.restoring,
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
