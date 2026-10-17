//! Access the other browser and optional full controls without showing the root on startup.
use crate::{cli::Browser, App};
use eframe::egui;

pub(crate) enum Action {
    Patch,
    Effect,
    Main,
}

pub(crate) fn toolbar(ctx: &egui::Context, enabled: bool, master: bool) -> Option<Action> {
    if !enabled {
        return None;
    }
    let mut action = None;
    egui::TopBottomPanel::top("selection_navigation").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if !master && ui.button("Patch browser").clicked() {
                action = Some(Action::Patch);
            }
            if ui.button("Effect browser").clicked() {
                action = Some(Action::Effect);
            }
            if ui.button("Favorites / routing").clicked() {
                action = Some(Action::Main);
            }
            ui.label(if master {
                "Master effects · close browser to save and return"
            } else {
                "Close browser to save and return"
            });
        });
    });
    action
}

impl App {
    pub(crate) fn selection_action(&mut self, action: Option<Action>, ctx: &egui::Context) {
        if self.selecting_master() && matches!(action, Some(Action::Patch)) {
            return;
        }
        let browser = match action {
            Some(Action::Patch) => {
                self.patch_browser.open = true;
                Some(Browser::Patch)
            }
            Some(Action::Effect) => {
                self.effect_browser.open = true;
                Some(Browser::Effect)
            }
            Some(Action::Main) => {
                if let Some(selection) = &mut self.selection {
                    selection.main_visible = true;
                }
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::Visible(true),
                );
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                None
            }
            None => None,
        };
        if let Some(browser) = browser {
            self.configure_browser_only(browser);
        }
    }
}
