//! Compact toolbar and child overlays, with isolated input and batched conditions.
use super::*;

impl Browser {
    pub(super) fn controls(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        ui.horizontal(|ui| {
            let height = 24.0;
            let solo_width = 84.0;
            let random_width = 62.0;
            let menu_width = 28.0;
            let search_width = (ui.available_width()
                - solo_width
                - random_width
                - menu_width
                - 3.0 * ui.spacing().item_spacing.x)
                .max(1.0);
            let search = ui.add_sized(
                [search_width, height],
                egui::TextEdit::singleline(&mut self.query)
                    .id(egui::Id::new("patch_browser_search"))
                    .hint_text("Search: pad -bright / plugin:floe")
                    .desired_width(search_width),
            );
            changed = search.changed();
            // One target covers the complete instrument control group.
            let names = &self.snapshot.as_ref().unwrap().instrument_names;
            let active: Vec<_> = names
                .iter()
                .filter_map(|name| {
                    let mode = plugin_mode(&self.query, &cmrt_patches::plugin_slug(name))?;
                    Some(format!(
                        "{name} {}",
                        if mode == PluginMode::Solo {
                            "Solo"
                        } else {
                            "Mute"
                        }
                    ))
                })
                .collect();
            let summary = if active.is_empty() {
                "Instruments: Solo / Mute".into()
            } else {
                format!("Instruments: {}", active.join(", "))
            };
            let solo = ui
                .add_sized([solo_width, height], egui::Button::new("Solo/Mute"))
                .on_hover_text(format!("{summary}\nOpen Solo/Mute overlay (M)"));
            if solo.clicked() {
                self.solo_draft = Some(self.query.clone());
            }
            let random = ui
                .add_enabled_ui(self.error.is_none() && !self.results.is_empty(), |ui| {
                    ui.add_sized([random_width, height], egui::Button::new("Random"))
                })
                .inner
                .on_hover_text("Choose a random patch from the filtered results (R)");
            if random.clicked() {
                self.select_random();
            }
            let menu = ui
                .add_sized([menu_width, height], egui::Button::new("☰"))
                .on_hover_text("Saved conditions");
            if menu.clicked() {
                self.condition_menu = true;
            }
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("browser_toolbar"),
                    [search.rect, solo.rect, random.rect, menu.rect],
                )
            });
        });
        if let Some(error) = &self.error {
            ui.colored_label(Color32::YELLOW, format!("Invalid search: {error}"));
        }
        self.panes(ui, changed);
    }

    pub(super) fn solo_overlay(&mut self, ctx: &egui::Context) {
        // Taken before the widgets draw, so a focused Solo/Mute toggle never also sees Enter.
        let mut accept =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Enter));
        let response = egui::Modal::new(egui::Id::new("browser_solo_mute")).show(ctx, |ui| {
            ui.set_width((ctx.screen_rect().width() - 60.0).clamp(240.0, 550.0));
            ui.heading("Instruments: Solo / Mute");
            ui.label(
                "Click an active mode again to return to normal. OK (Enter) applies all changes.",
            );
            if let Some(snapshot) = &self.snapshot {
                // Names take only their own width so each mode stays next to its plugin.
                let font = egui::TextStyle::Body.resolve(ui.style());
                let longest = snapshot
                    .instrument_names
                    .iter()
                    .map(|name| {
                        ui.fonts(|fonts| {
                            fonts
                                .layout_no_wrap(name.clone(), font.clone(), Color32::PLACEHOLDER)
                                .size()
                                .x
                        })
                    })
                    .fold(0.0, f32::max);
                egui::ScrollArea::vertical()
                    .max_height((ctx.screen_rect().height() - 190.0).max(80.0))
                    .show(ui, |ui| {
                        for name in &snapshot.instrument_names {
                            ui.horizontal(|ui| {
                                let width = longest.min(ui.available_width() - 100.0);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(width, 22.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.set_min_width(width);
                                        ui.add(egui::Label::new(name).truncate());
                                    },
                                );
                                let slug = cmrt_patches::plugin_slug(name);
                                for (label, mode) in
                                    [("Solo", PluginMode::Solo), ("Mute", PluginMode::Mute)]
                                {
                                    let draft = self.solo_draft.as_mut().unwrap();
                                    if ui
                                        .selectable_label(
                                            plugin_mode(draft, &slug) == Some(mode),
                                            label,
                                        )
                                        .clicked()
                                    {
                                        *draft = toggle_plugin_term(draft, &slug, mode);
                                    }
                                }
                            });
                        }
                    });
            }
            accept |= ui.button("OK").clicked();
        });
        if response.should_close() {
            self.solo_draft = None;
        } else if accept {
            self.query = self.solo_draft.take().unwrap();
            self.filter(true);
        }
    }

    pub(super) fn conditions_menu(&mut self, ctx: &egui::Context) {
        let response = egui::Modal::new(egui::Id::new("browser_conditions_menu")).show(ctx, |ui| {
            ui.heading("Saved conditions");
            ui.label("ALL saves to Etc. Favorites / History remain separate.");
            let writable = self.store.as_ref().is_some_and(|store| store.writable());
            if ui
                .add_enabled(
                    writable && self.error.is_none() && !self.query.trim().is_empty(),
                    egui::Button::new("Save condition to Role"),
                )
                .clicked()
            {
                let role = FilterGroup::ALL[self.role]
                    .role()
                    .unwrap_or(cmrt_patches::PatchRole::Etc);
                let store = self.store.as_mut().unwrap();
                let mut presets = store.presets.clone();
                presets.push((role.key().into(), self.query.clone()));
                if store.replace(presets) {
                    self.rebuild_conditions();
                    self.condition_menu = false;
                }
            }
            if let Some(error) = self.store.as_ref().and_then(|store| store.error.as_ref()) {
                ui.colored_label(Color32::YELLOW, error);
            }
            ui.label("Saved presets can be deleted in the Preset pane. Esc returns to browsing.");
        });
        if response.should_close() {
            self.condition_menu = false;
        }
    }
}
