//! Keyboard prefixes and anchored popups for the two main-window menus.
use eframe::egui::{self, Event, Key};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Menu {
    Settings,
    Analysis,
}

impl Menu {
    pub(crate) fn current(ctx: &egui::Context) -> Option<Self> {
        [Self::Settings, Self::Analysis]
            .into_iter()
            .find(|menu| ctx.memory(|m| m.is_popup_open(menu.id())))
    }

    pub(crate) fn help_page(self) -> crate::help_overlay::Page {
        match self {
            Self::Settings => crate::help_overlay::Page::Settings,
            Self::Analysis => crate::help_overlay::Page::Analysis,
        }
    }

    fn id(self) -> egui::Id {
        egui::Id::new(match self {
            Self::Settings => "main_settings_menu",
            Self::Analysis => "main_analysis_menu",
        })
    }

    pub(crate) fn take(self, ctx: &egui::Context, key: Key) -> bool {
        let id = self.id().with("key");
        if ctx.data(|d| d.get_temp::<Key>(id)) == Some(key) {
            ctx.data_mut(|d| d.remove::<Key>(id));
            ctx.memory_mut(|m| m.close_popup());
            true
        } else {
            false
        }
    }

    pub(crate) fn button(self, ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
        let hover = match self {
            Self::Settings => "Settings (D)",
            Self::Analysis => "Analysis display settings (A)",
        };
        let response = ui.button("☰").on_hover_text(hover);
        if response.clicked() {
            ui.memory_mut(|m| m.toggle_popup(self.id()));
        }
        egui::popup_below_widget(
            ui,
            self.id(),
            &response,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                if self == Self::Settings {
                    // The button sits in a narrow right-aligned allocation, not the menu body.
                    ui.set_width(
                        360.0_f32
                            .min(ui.ctx().screen_rect().width() - 24.0)
                            .max(1.0),
                    );
                }
                contents(ui);
                ui.separator();
                if ui.button("Help (? / F1)").clicked() {
                    crate::help_overlay::open(ui.ctx(), self.help_page());
                }
            },
        );
    }
}

/// Run before transport, SMF exit, and all other main-window handlers.
/// Keys in a menu belong only to that menu, even on the frame that closes it.
pub(crate) fn keyboard(ctx: &egui::Context, enabled: bool) {
    let menus = [Menu::Settings, Menu::Analysis];
    for menu in menus {
        ctx.data_mut(|d| d.remove::<Key>(menu.id().with("key")));
    }
    let mut open = Menu::current(ctx);
    if !enabled {
        if open.is_some() {
            ctx.memory_mut(|m| m.close_popup());
        }
        return;
    }
    let editing = crate::patch_browser_ui::editing_text(ctx);
    let mut captured = open.is_some();
    let mut opened = None;
    let mut cancelled = false;
    let mut action = None;
    ctx.input_mut(|input| {
        input.events.retain(|event| match event {
            Event::Key {
                key,
                pressed,
                repeat,
                modifiers,
                ..
            } => {
                if !captured && !editing && *pressed && !repeat && modifiers.is_none() {
                    open = match key {
                        Key::D => Some(Menu::Settings),
                        Key::A => Some(Menu::Analysis),
                        _ => None,
                    };
                    if let Some(menu) = open {
                        opened = Some(menu);
                        captured = true;
                        return false;
                    }
                }
                if captured && *pressed && !repeat && modifiers.is_none() {
                    if *key == Key::Escape {
                        cancelled = true;
                        open = None;
                        action = None;
                    } else if let Some(menu) = open.take() {
                        action = Some((menu, *key));
                    }
                }
                !captured
            }
            Event::Text(_) if captured => false,
            _ => true,
        });
    });
    if let Some(menu) = opened {
        ctx.memory_mut(|m| m.open_popup(menu.id()));
    }
    if cancelled {
        ctx.memory_mut(|m| m.close_popup());
    } else if let Some((menu, key)) = action {
        ctx.data_mut(|d| d.insert_temp(menu.id().with("key"), key));
    }
}
