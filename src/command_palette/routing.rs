//! Candidate ownership and deferred navigation through the existing browser lifecycle.
use super::{catalog, history, viewport_id, Command, State};
use crate::App;
use eframe::egui;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Screen {
    #[default]
    Main,
    Patch,
    Effect,
    Drum,
}

/// Captures the object a command would act on, independently of window visibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Target {
    drum: Option<(
        Option<crate::status::PatchIdentity>,
        Option<u8>,
        crate::drum_browser::Focus,
    )>,
    instrument: Option<i32>,
    effects: Vec<i32>,
    selection: Option<std::path::PathBuf>,
    patch: Option<(u64, Option<(u64, u64)>)>,
    effect: Option<(Option<i32>, Option<usize>, u64)>,
}

impl Target {
    fn of(app: &App, screen: Screen) -> Self {
        Self {
            drum: (screen == Screen::Drum).then(|| {
                (
                    app.drum_browser.saved.selected.clone(),
                    app.drum_browser
                        .selected()
                        .and_then(|c| app.drum_browser.saved.note(c)),
                    app.drum_browser.saved.focus,
                )
            }),
            instrument: app.instrument_id(),
            effects: app.effect_ids(),
            selection: app.selection.as_ref().map(|s| s.path.clone()),
            patch: (screen == Screen::Patch).then(|| {
                (
                    app.patch_browser.generation,
                    app.patch_browser
                        .requests
                        .selected
                        .as_ref()
                        .map(|t| (t.serial, t.generation)),
                )
            }),
            effect: (screen == Screen::Effect).then_some((
                app.effect_browser.target,
                app.effect_browser.selected,
                app.effect_catalog.generation,
            )),
        }
    }
}

impl Screen {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::Patch => "Patch browser",
            Self::Effect => "Effect browser",
            Self::Drum => "Drum kits",
        }
    }
    fn viewport(self) -> egui::ViewportId {
        match self {
            Self::Main => egui::ViewportId::ROOT,
            Self::Patch => crate::patch_browser_ui::browser_viewport(),
            Self::Effect => crate::effect_browser_ui::viewport(),
            Self::Drum => crate::drum_browser_ui::viewport(),
        }
    }
}

pub(crate) fn allowed(app: &App, screen: Screen) -> bool {
    if app.drum_browser.editing.is_some()
        || app.mml_input.open
        || app.favorites.rename.is_some()
        || app.confirm_rescan
        || app.first_instrument.dialog_open()
        || app.patch_browser.main_random_overlay
        || app.patch_browser.solo_draft.is_some()
        || app.patch_browser.condition_menu
        || app.patch_browser.requests.confirmation.is_some()
        || app
            .patch_browser
            .requests
            .preparing
            .as_ref()
            .or(app.patch_browser.requests.applying.as_ref())
            .is_some_and(|t| t.candidate.measurement.is_heavy_offline_load())
    {
        return false;
    }
    match screen {
        Screen::Main => app.resident.is_none(),
        Screen::Patch => !app.selecting_master(),
        Screen::Effect => true,
        Screen::Drum => app.can_open_drums(),
    }
}

fn definitions(app: &App, screen: Screen) -> Vec<Command> {
    let mut result = match screen {
        Screen::Main => catalog::main_commands(app),
        Screen::Patch => catalog::patch_commands(&app.patch_browser),
        Screen::Effect => catalog::effect_commands(&app.effect_browser),
        Screen::Drum => super::drums::commands(app),
    };
    for c in &mut result {
        c.screen = screen;
        c.target = Some(Target::of(app, screen));
        if c.id == "add-favorite" {
            c.enabled &= match screen {
                Screen::Effect => app
                    .effect_browser
                    .target
                    .is_some_and(|id| app.effect_ids().contains(&id)),
                _ => app.instrument_id().is_some(),
            };
        }
    }
    result
}

pub(crate) fn commands(app: &App, current: Screen) -> Vec<Command> {
    if !allowed(app, current) {
        return vec![];
    }
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for screen in [
        current,
        Screen::Main,
        Screen::Patch,
        Screen::Effect,
        Screen::Drum,
    ] {
        // Resident windows edit an editor-owned target; another browser needs a new IPC Open.
        if !allowed(app, screen) || (app.resident.is_some() && screen != current) {
            continue;
        }
        for c in definitions(app, screen) {
            if !c.enabled || (screen != current && c.id.starts_with("close-")) {
                continue;
            }
            // Instrument and effect favorites are different operations; other shared commands
            // (help, column width, random patch) use the current screen's definition first.
            let key = if c.id == "add-favorite" && screen == Screen::Effect {
                "add-effect-favorite"
            } else {
                c.id
            };
            if seen.insert(key) {
                result.push(c);
            }
        }
    }
    result
}

fn help_open(ctx: &egui::Context, screen: Screen) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new("help_overlay").with(screen.viewport())))
        .unwrap_or(false)
}

fn navigate(app: &mut App, ctx: &egui::Context, screen: Screen) {
    if let Some(selection) = &mut app.selection {
        selection.browser_only = match screen {
            Screen::Main | Screen::Drum => None,
            Screen::Patch => Some(crate::cli::Browser::Patch),
            Screen::Effect => Some(crate::cli::Browser::Effect),
        };
        selection.main_visible = screen == Screen::Main;
    }
    if screen != Screen::Drum && app.drum_browser.open {
        app.close_drum_browser();
    }
    if screen != Screen::Patch && app.patch_browser.open {
        app.patch_browser.close();
    }
    if screen != Screen::Effect && app.effect_browser.open {
        app.effect_browser.close();
    }
    match screen {
        Screen::Drum if !app.drum_browser.open => app.open_drum_browser(),
        Screen::Patch if !app.patch_browser.open => app.open_patch_browser(),
        Screen::Effect if !app.effect_browser.open => app.open_effect_browser(),
        _ => {}
    }
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("palette-destination-focus"), screen));
    ctx.send_viewport_cmd_to(screen.viewport(), egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd_to(screen.viewport(), egui::ViewportCommand::Focus);
}

pub(crate) fn take_focus(ctx: &egui::Context, screen: Screen) -> bool {
    let id = egui::Id::new("palette-destination-focus");
    if ctx.data(|d| d.get_temp::<Screen>(id)) != Some(screen) {
        return false;
    }
    ctx.data_mut(|d| d.remove::<Screen>(id));
    true
}

/// Root update only, before any viewport consumes keys. A child selection is handled
/// on the next root update, so closing it cannot trigger the selection-exit callback.
pub(crate) fn prepare(app: &mut App, ctx: &egui::Context) {
    if let Some(owner) = super::owner(ctx) {
        let closed = (owner == Screen::Patch.viewport() && !app.patch_browser.open)
            || (owner == Screen::Effect.viewport() && !app.effect_browser.open)
            || (owner == Screen::Drum.viewport() && !app.drum_browser.open);
        if closed {
            ctx.data_mut(|d| {
                d.remove::<egui::ViewportId>(egui::Id::new("command-palette-owner"));
                d.remove::<State>(viewport_id(owner));
            });
        }
    }
    history::configure(ctx, app.config_path.as_ref().ok().map(|p| p.as_path()));
    if let Some(error) = history::take_error(ctx) {
        app.config_error = Some(error);
    }
    for source in [Screen::Main, Screen::Patch, Screen::Effect, Screen::Drum] {
        let source_id = viewport_id(source.viewport());
        let mut state = ctx
            .data(|d| d.get_temp::<State>(source_id))
            .unwrap_or_default();
        let Some(label) = state.pending.take() else {
            continue;
        };
        let destination = state.pending_screen;
        ctx.data_mut(|d| d.insert_temp(source_id, state.clone()));
        let source_open = match source {
            Screen::Main => true,
            Screen::Patch => app.patch_browser.open,
            Screen::Effect => app.effect_browser.open,
            Screen::Drum => app.drum_browser.open,
        };
        if !source_open
            || !allowed(app, source)
            || help_open(ctx, source)
            || help_open(ctx, destination)
        {
            continue;
        }
        let valid = commands(app, source).iter().any(|c| {
            c.label == label && c.screen == destination && c.target == state.pending_target
        });
        if !valid {
            state.open = true;
            state.query = state.pending_query.clone();
            state.selected = 0;
            ctx.data_mut(|d| d.insert_temp(source_id, state));
            super::claim_owner(ctx, source.viewport());
            continue;
        }
        if destination != source || (destination == Screen::Main && app.browser_open()) {
            navigate(app, ctx, destination);
        }
        let target_id = viewport_id(destination.viewport());
        let mut target = ctx
            .data(|d| d.get_temp::<State>(target_id))
            .unwrap_or_default();
        target.pending = Some(label);
        target.pending_screen = destination;
        target.pending_query = state.pending_query;
        target.pending_target = state.pending_target;
        ctx.data_mut(|d| d.insert_temp(target_id, target));
        ctx.request_repaint();
        break;
    }
}

#[cfg(test)]
mod tests;
