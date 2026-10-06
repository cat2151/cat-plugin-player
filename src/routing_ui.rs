//! Routing actions and analysis share the right sidebar.
use crate::App;
use eframe::egui;

impl App {
    pub(crate) fn routing_panel(&mut self, ctx: &egui::Context, analysis_on_right: bool) {
        // Apply actions only after the routing and analysis panes are drawn.
        let mut add_favorite = None;
        let mut show: Option<i32> = None;
        let mut hide: Option<i32> = None;
        let mut remove: Option<i32> = None;
        let mut set_bypass = None;
        egui::SidePanel::right("instances")
            .min_width(240.0)
            .default_width(320.0)
            .show(ctx, |ui| {
                if analysis_on_right {
                    egui::TopBottomPanel::bottom("audio_analysis_right")
                        .show_inside(ui, |ui| self.analysis_contents(ui, ctx, true));
                }
                egui::ScrollArea::vertical()
                    .id_salt("routing_scroll")
                    .show(ui, |ui| {
                        ui.heading("Routing");
                        ui.label(self.routing_label());
                        let mut bypass = self.effect_bypassed;
                        if ui
                            .add_enabled(
                                self.pending.is_none() && self.effect_id().is_some(),
                                egui::Checkbox::new(&mut bypass, "Bypass effect"),
                            )
                            .changed()
                        {
                            set_bypass = Some(bypass);
                        }
                        for instance in &self.instances {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    self.plugin_icons
                                        .draw(ui, &instance.plugin, &self.config_path);
                                    ui.label(format!(
                                        "{}: {}",
                                        instance.kind.label(),
                                        instance.label
                                    ));
                                });
                                if let Some((_, favorite_id)) = self
                                    .favorites
                                    .active
                                    .iter()
                                    .find(|(id, _)| *id == instance.id)
                                {
                                    if let Some(favorite) = self
                                        .favorites
                                        .library
                                        .entries
                                        .iter()
                                        .find(|f| &f.id == favorite_id)
                                    {
                                        ui.label(format!("Last favorite: {}", favorite.name));
                                    }
                                }
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .add_enabled(
                                            self.pending.is_none()
                                                && !self.restoring
                                                && self.favorites.error.is_none(),
                                            egui::Button::new("★ Add favorite"),
                                        )
                                        .clicked()
                                    {
                                        add_favorite = Some(instance.id);
                                    }
                                    let ui_visible = self.host.ui_is_visible(instance.id);
                                    if ui
                                        .button(if ui_visible { "Hide UI" } else { "Show UI" })
                                        .clicked()
                                    {
                                        if ui_visible {
                                            hide = Some(instance.id);
                                        } else {
                                            show = Some(instance.id);
                                        }
                                    }
                                    if ui
                                        .add_enabled(
                                            self.pending.is_none(),
                                            egui::Button::new("Remove"),
                                        )
                                        .clicked()
                                    {
                                        remove = Some(instance.id);
                                    }
                                });
                            });
                        }
                    });
            });

        if let Some(id) = add_favorite {
            self.add_favorite(id);
        }
        if let Some(bypass) = set_bypass {
            self.set_effect_bypass(bypass);
        }
        if let Some(id) = show {
            if let Err(code) = self.host.show_ui(id) {
                self.status = format!("Could not open the plugin UI (code {code})");
            }
        }
        if let Some(id) = hide {
            self.host.hide_ui(id);
        }
        if let Some(id) = remove {
            self.remove_plugin(id);
        }
    }
}
