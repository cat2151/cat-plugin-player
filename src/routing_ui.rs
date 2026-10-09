//! Routing actions and analysis share the right sidebar.
use crate::App;
use eframe::egui;

impl App {
    pub(crate) fn routing_panel(&mut self, ctx: &egui::Context, analysis_on_right: bool) {
        // Apply actions only after the routing and analysis panes are drawn.
        let busy = self.actions_busy();
        let mut add_favorite = None;
        let mut show: Option<i32> = None;
        let mut hide: Option<i32> = None;
        let mut remove: Option<i32> = None;
        let mut set_bypass = None;
        let mut reorder = None;
        egui::SidePanel::right("instances")
            .min_width(240.0)
            .default_width(320.0)
            .show(ctx, |ui| {
                if analysis_on_right {
                    egui::TopBottomPanel::bottom("audio_analysis_right")
                        .show_inside(ui, |ui| self.analysis_contents(ui, ctx, true));
                }
                // Reserve a fixed gutter so the scrollbar cannot cover row actions.
                ui.style_mut().spacing.scroll = egui::style::ScrollStyle {
                    bar_width: 10.0,
                    ..egui::style::ScrollStyle::solid()
                };
                egui::ScrollArea::vertical()
                    .id_salt("routing_scroll")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Text selection competes with dragging the routing handles.
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(self.routing_label());
                        if !busy && self.effect_id().is_some() {
                            let mut bypass = self.effect_bypassed;
                            if ui.checkbox(&mut bypass, "Bypass effects").changed() {
                                set_bypass = Some(bypass);
                            }
                        }
                        for instance in &self.instances {
                            let group = ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    if instance.kind == crate::plugin_list::PluginKind::Effect {
                                        let handle = ui
                                            .add_enabled(
                                                !busy && !self.restoring,
                                                egui::Button::new("::")
                                                    .sense(egui::Sense::drag())
                                                    .min_size(egui::vec2(
                                                        28.0,
                                                        ui.spacing().interact_size.y,
                                                    )),
                                            )
                                            .on_hover_cursor(egui::CursorIcon::Grab)
                                            .on_hover_text("Drag vertically to reorder effects");
                                        handle.dnd_set_drag_payload(instance.id);
                                    }
                                    self.plugin_icons
                                        .draw(ui, &instance.plugin, &self.config_path);
                                    ui.add(
                                        egui::Label::new(format!(
                                            "{}: {}",
                                            instance.kind.label(),
                                            instance.label
                                        ))
                                        .wrap(),
                                    );
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
                                        ui.label(format!(
                                            "Last {}: {}",
                                            if favorite.favorite {
                                                "favorite"
                                            } else {
                                                "history"
                                            },
                                            favorite.name
                                        ));
                                    }
                                }
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .add_enabled(
                                            !busy
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
                                    if ui.add_enabled(!busy, egui::Button::new("Remove")).clicked()
                                    {
                                        remove = Some(instance.id);
                                    }
                                });
                            });
                            if instance.kind == crate::plugin_list::PluginKind::Effect
                                && !busy
                                && !self.restoring
                            {
                                if egui::DragAndDrop::payload::<i32>(ctx)
                                    .is_some_and(|id| *id == instance.id)
                                {
                                    ui.painter().rect_stroke(
                                        group.response.rect,
                                        4.0,
                                        ui.visuals().selection.stroke,
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                if group
                                    .response
                                    .dnd_hover_payload::<i32>()
                                    .is_some_and(|id| *id != instance.id)
                                {
                                    let after = ui.input(|input| {
                                        input
                                            .pointer
                                            .interact_pos()
                                            .is_some_and(|p| p.y > group.response.rect.center().y)
                                    });
                                    let y = if after {
                                        group.response.rect.bottom()
                                    } else {
                                        group.response.rect.top()
                                    };
                                    ui.painter().line_segment(
                                        [
                                            egui::pos2(group.response.rect.left(), y),
                                            egui::pos2(group.response.rect.right(), y),
                                        ],
                                        egui::Stroke::new(
                                            3.0_f32,
                                            ui.visuals().selection.stroke.color,
                                        ),
                                    );
                                }
                                if let Some(id) = group.response.dnd_release_payload::<i32>() {
                                    let after = ui.input(|input| {
                                        input
                                            .pointer
                                            .interact_pos()
                                            .is_some_and(|p| p.y > group.response.rect.center().y)
                                    });
                                    reorder = Some((*id, instance.id, after));
                                }
                            }
                        }
                    });
            });

        if let Some(id) = egui::DragAndDrop::payload::<i32>(ctx) {
            if let Some(instance) = self.instances.iter().find(|instance| instance.id == *id) {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                if let Some(pos) = ctx.pointer_interact_pos() {
                    egui::Area::new(egui::Id::new("routing_drag_preview"))
                        .order(egui::Order::Tooltip)
                        .interactable(false)
                        .fixed_pos(pos + egui::vec2(16.0, 16.0))
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(format!("Dragging: {}", instance.label))
                                        .selectable(false),
                                );
                            });
                        });
                }
            }
        }

        if let Some((id, target, after)) = reorder {
            self.reorder_effect(id, target, after);
        }
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
