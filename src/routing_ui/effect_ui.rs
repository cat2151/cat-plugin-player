//! One-key editor toggle for a single effect, letter picker for an effect chain.
use crate::App;
use eframe::egui::{self, Event, Key, Modifiers};

const KEYS: [Key; 26] = [
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
];

fn id() -> egui::Id {
    egui::Id::new("effect_ui_picker")
}

pub(crate) fn is_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(id())).unwrap_or(false)
}

impl App {
    pub(crate) fn effect_ui_picker(&mut self, ctx: &egui::Context) {
        let enabled = !self.actions_busy()
            && !self.browser_open()
            && !self.mml_input.open
            && self.favorites.rename.is_none()
            && !self.confirm_rescan
            && !self.first_instrument.dialog_open()
            && !self.patch_browser.main_random_overlay
            && !crate::help_overlay::is_open(ctx)
            && crate::main_menu::Menu::current(ctx).is_none()
            && self.resident.is_none()
            && !self.selection.as_ref().is_some_and(|s| !s.main_visible);
        let fresh_press = ctx.input(|input| {
            input.events.iter().any(|event| {
                matches!(event, Event::Key {
                    key: Key::O, pressed: true, repeat: false, modifiers, ..
                } if *modifiers == Modifiers::NONE)
            })
        });
        let trigger = enabled && !is_open(ctx) && fresh_press && self.main_shortcut(ctx, Key::O);
        if !trigger && !is_open(ctx) {
            return;
        }
        let rows: Vec<_> = self
            .instances
            .iter()
            .filter(|i| i.kind == crate::plugin_list::PluginKind::Effect)
            .map(|i| (i.id, i.label.clone(), self.host.ui_is_visible(i.id)))
            .collect();
        if let Some(id) = show(ctx, &rows, enabled, trigger) {
            if self.host.ui_is_visible(id) {
                self.host.hide_ui(id);
            } else {
                self.open_editor(id);
            }
        }
    }
}

fn show(
    ctx: &egui::Context,
    rows: &[(i32, String, bool)],
    enabled: bool,
    trigger: bool,
) -> Option<i32> {
    let was_open = is_open(ctx);
    let mut open = enabled && (was_open || trigger) && !rows.is_empty();
    if enabled && !was_open && trigger && rows.len() == 1 {
        return Some(rows[0].0);
    }
    let mut selected = None;
    if open {
        let mut cancel = false;
        if was_open {
            ctx.input(|input| {
                for event in &input.events {
                    if let Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } = event
                    {
                        if *modifiers == Modifiers::NONE {
                            cancel |= *key == Key::Escape;
                            if selected.is_none() {
                                selected = KEYS
                                    .iter()
                                    .position(|k| k == key)
                                    .and_then(|index| rows.get(index))
                                    .map(|row| row.0);
                            }
                        }
                    }
                }
            });
        }
        if !cancel && selected.is_none() {
            let response = egui::Modal::new(id()).show(ctx, |ui| {
                ui.heading("Effect UI");
                ui.label("a–z: show / hide and close   Esc: cancel");
                egui::ScrollArea::vertical()
                    .max_height((ctx.screen_rect().height() - 160.0).max(60.0))
                    .show(ui, |ui| {
                        for (index, (instance, label, visible)) in rows.iter().enumerate() {
                            let letter =
                                (index < KEYS.len()).then(|| char::from(b'a' + index as u8));
                            let key = letter.map_or(String::new(), |key| format!("{key}: "));
                            let action = if *visible { "Hide UI" } else { "Show UI" };
                            if ui.button(format!("{key}{label} — {action}")).clicked() {
                                selected = Some(*instance);
                            }
                        }
                    });
            });
            cancel |= response.should_close();
        }
        if cancel {
            selected = None;
        }
        open = !cancel && selected.is_none();
    }
    if was_open || open {
        // Even a closing key belongs to the picker, never to transport or SMF exit.
        ctx.input_mut(|input| {
            input.events.clear();
            input.pointer = Default::default();
        });
    }
    ctx.data_mut(|d| d.insert_temp(id(), open));
    selected
}

#[cfg(test)]
mod tests;
