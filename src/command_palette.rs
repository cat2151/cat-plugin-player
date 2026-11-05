//! Per-viewport command search. Background processing continues with inert UI input.
//! Commands replay the existing shortcut path on the next frame, after revalidation.
use eframe::egui::{self, Event, Key, Modifiers};

mod catalog;
mod drums;
mod history;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod interaction_tests;
mod routing;
mod row;
pub(crate) use routing::{allowed, commands, prepare, take_focus, Screen};
pub(crate) use row::shortcut_hint;
#[cfg(test)]
mod layout_tests;
mod migemo;
mod search;
#[cfg(test)]
mod search_tests;
mod view;
#[cfg(test)]
pub(crate) use catalog::{effect_commands, main_commands, patch_commands};
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub(crate) struct Command {
    id: &'static str,
    screen: Screen,
    target: Option<routing::Target>,
    label: &'static str,
    japanese: &'static str,
    words: &'static str,
    shortcut: &'static str,
    keys: Vec<Key>,
    enabled: bool,
}

impl Command {
    fn new(
        id: &'static str,
        label: &'static str,
        japanese: &'static str,
        words: &'static str,
        shortcut: &'static str,
        keys: &[Key],
        enabled: bool,
    ) -> Self {
        Self {
            id,
            screen: Screen::Main,
            target: None,
            label,
            japanese,
            words,
            shortcut,
            keys: keys.to_vec(),
            enabled,
        }
    }

    fn label(&self) -> String {
        format!(
            "{}: {} / {}",
            self.screen.label(),
            self.label,
            self.japanese
        )
    }

    fn key_label(&self) -> String {
        self.shortcut.into()
    }

    fn identity(&self) -> String {
        format!("{:?}/{}", self.screen, self.id)
    }

    fn matches(&self, query: &search::SearchQuery) -> bool {
        let text = format!("{} {} {}", self.label(), self.words, self.shortcut);
        query.matches(&text)
    }
}

#[derive(Clone, Default)]
struct State {
    open: bool,
    query: String,
    search: search::SearchQuery,
    selected: usize,
    focus: Option<egui::Id>,
    pending: Option<&'static str>,
    pending_screen: Screen,
    pending_query: String,
    pending_target: Option<routing::Target>,
    inventory: Vec<(String, Option<routing::Target>)>,
    dispatched: Option<(String, String, Vec<Key>)>,
    history: history::History,
    geometry: Option<(f32, f32)>,
    screen_rect: Option<egui::Rect>,
    composing: bool,
    handled: Option<u64>,
}

pub(crate) struct Suspended {
    input: egui::InputState,
    commands: Vec<Command>,
}

fn viewport_id(viewport: egui::ViewportId) -> egui::Id {
    egui::Id::new("command-palette").with(viewport)
}

fn id(ctx: &egui::Context) -> egui::Id {
    viewport_id(ctx.viewport_id())
}

fn state(ctx: &egui::Context) -> State {
    let id = id(ctx);
    ctx.data(|d| d.get_temp(id)).unwrap_or_default()
}

fn save(ctx: &egui::Context, state: State) {
    let id = id(ctx);
    ctx.data_mut(|d| d.insert_temp(id, state));
}

fn frame_token(ctx: &egui::Context) -> u64 {
    ctx.cumulative_pass_nr()
        .saturating_sub(ctx.output(|o| o.num_completed_passes as u64))
}

// Match cat-smf-editor's list navigation without taking Home/End away from editing.
fn movement(key: Key, modifiers: Modifiers, selected: usize, count: usize) -> Option<usize> {
    let last = count.saturating_sub(1);
    if modifiers.ctrl && !modifiers.shift && !modifiers.alt && !modifiers.mac_cmd {
        return match key {
            Key::Home => Some(0),
            Key::End => Some(last),
            _ => None,
        };
    }
    if !modifiers.is_none() {
        return None;
    }
    match key {
        Key::ArrowDown => Some(selected.saturating_add(1).min(last)),
        Key::ArrowUp => Some(selected.saturating_sub(1)),
        Key::PageDown => Some(selected.saturating_add(10).min(last)),
        Key::PageUp => Some(selected.saturating_sub(10)),
        _ => None,
    }
}

fn clear(ctx: &egui::Context, pointer: bool) {
    ctx.input_mut(|i| {
        i.events.clear();
        i.keys_down.clear();
        i.modifiers = Modifiers::NONE;
        i.raw.events.clear();
        i.raw.dropped_files.clear();
        i.raw_scroll_delta = egui::Vec2::ZERO;
        i.smooth_scroll_delta = egui::Vec2::ZERO;
        if pointer {
            i.pointer = Default::default();
        }
    });
}

/// Before any help, exit, or shortcut handlers in the owning viewport.
pub(crate) fn begin(
    ctx: &egui::Context,
    commands: Vec<Command>,
    allowed: bool,
) -> Option<Suspended> {
    if owner(ctx).is_some_and(|owner| owner != ctx.viewport_id()) {
        clear(ctx, true);
        return None;
    }
    let allowed = allowed && !ctx.input(|i| i.viewport().close_requested());
    let mut state = state(ctx);
    let token = frame_token(ctx);
    let fresh = state.handled != Some(token);
    if !fresh {
        clear(ctx, true);
    }
    if let Some(label) = fresh.then(|| state.pending.take()).flatten() {
        state.handled = Some(token);
        // UI state can change while a worker completes: never dispatch a stale action.
        if allowed {
            if let Some(command) = commands.iter().find(|c| {
                c.label == label
                    && c.screen == state.pending_screen
                    && c.enabled
                    && c.target == state.pending_target
            }) {
                if let Some(focus) = ctx.memory(|m| m.focused()) {
                    ctx.memory_mut(|m| m.surrender_focus(focus));
                }
                state.dispatched = Some((
                    command.identity(),
                    state.pending_query.clone(),
                    command.keys.clone(),
                ));
                clear(ctx, true);
                ctx.input_mut(|i| {
                    i.events = command
                        .keys
                        .iter()
                        .map(|&key| Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Modifiers::NONE,
                        })
                        .collect();
                });
            } else {
                state.open = true;
                state.selected = 0;
                state.query = state.pending_query.clone();
                state.history = history::load(ctx);
            }
        }
    }
    let toggle = ctx.input(|i| {
        i.events.iter().any(|e| {
            matches!(e,
        Event::Key { key: Key::P, pressed: true, repeat: false, modifiers, .. }
        if modifiers.ctrl && modifiers.shift && !modifiers.alt && !modifiers.mac_cmd)
        })
    });
    if !allowed {
        if state.open {
            clear(ctx, true);
        }
        state = State::default();
        release_owner(ctx);
    } else if !state.open && toggle {
        state = State {
            open: true,
            focus: ctx.memory(|m| m.focused()),
            history: history::load(ctx),
            ..Default::default()
        };
        ctx.memory_mut(|m| m.close_popup());
        clear(ctx, true);
    }
    let suspended = state.open.then(|| {
        claim_owner(ctx, ctx.viewport_id());
        let input = ctx.input(Clone::clone);
        clear(ctx, true);
        Suspended { input, commands }
    });
    save(ctx, state);
    suspended
}

/// After painting background widgets, restore the real input only for the palette.
pub(crate) fn end(ctx: &egui::Context, suspended: Option<Suspended>) {
    let mut current = state(ctx);
    if let Some((identity, query, keys)) = current.dispatched.take() {
        let unhandled = ctx.input(|i| {
            i.events.iter().any(|e| {
                matches!(e,
            Event::Key { key, pressed: true, .. } if keys.contains(key))
            })
        });
        if !unhandled {
            current.history = history::load(ctx);
            current.history.record(&query, &identity);
            history::save(ctx, &current.history);
        } else {
            current.open = true;
            current.selected = 0;
            claim_owner(ctx, ctx.viewport_id());
            clear(ctx, false);
            ctx.request_repaint();
        }
        save(ctx, current);
    }
    let Some(Suspended { input, commands }) = suspended else {
        return;
    };
    ctx.input_mut(|i| *i = input);
    let mut state = state(ctx);
    state.handled = Some(frame_token(ctx));
    let events = ctx.input(|i| i.events.clone());
    let ime = state.composing || events.iter().any(|e| matches!(e, Event::Ime(_)));
    for event in &events {
        match event {
            Event::Ime(egui::ImeEvent::Preedit(text)) => state.composing = !text.is_empty(),
            Event::Ime(egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled) => {
                state.composing = false
            }
            _ => {}
        }
    }
    state.search.update(&state.query);
    let inventory: Vec<_> = commands
        .iter()
        .filter(|c| c.enabled)
        .map(|c| (c.identity(), c.target.clone()))
        .collect();
    let invalid = state.inventory != inventory;
    if invalid {
        state.selected = 0;
        state.inventory = inventory;
    }
    let mut filtered = filter(&commands, &state);
    state.selected = state.selected.min(filtered.len().saturating_sub(1));
    let mut execute = false;
    let mut cancel = false;
    let search_input = events
        .iter()
        .any(|e| matches!(e, Event::Text(_) | Event::Paste(_) | Event::Ime(_)));
    let mut reveal = false;
    if !ime {
        ctx.input_mut(|i| {
            i.events.retain(|event| {
                let Event::Key {
                    key,
                    pressed,
                    repeat,
                    modifiers,
                    ..
                } = event
                else {
                    return true;
                };
                if let Some(selected) = movement(*key, *modifiers, state.selected, filtered.len()) {
                    if *pressed {
                        state.selected = selected;
                        reveal = true;
                    }
                    return false;
                }
                if !modifiers.is_none() {
                    return true;
                }
                if *pressed {
                    match key {
                        Key::Enter if !repeat && !search_input && !invalid => execute = true,
                        Key::Escape if !repeat => cancel = true,
                        _ => {}
                    }
                }
                !matches!(key, Key::Enter | Key::Escape)
            });
        });
    }
    let chosen = if execute {
        filtered.get(state.selected).filter(|c| c.enabled).copied()
    } else {
        None
    };
    if !cancel && chosen.is_none() {
        view::show(ctx, &commands, &mut filtered, &mut state, reveal);
    }
    if cancel || chosen.is_some() {
        state.open = false;
        release_owner(ctx);
        state.pending = chosen.map(|c| c.label);
        if let Some(command) = chosen {
            state.pending_screen = command.screen;
            state.pending_query = state.query.clone();
            state.pending_target = command.target.clone();
        }
        if let Some(focus) = ctx.memory(|m| m.focused()) {
            ctx.memory_mut(|m| m.surrender_focus(focus));
        }
        if cancel {
            if let Some(focus) = state.focus {
                ctx.memory_mut(|m| m.request_focus(focus));
            }
        }
        ctx.request_repaint();
    }
    clear(ctx, false);
    save(ctx, state);
}

fn owner(ctx: &egui::Context) -> Option<egui::ViewportId> {
    ctx.data(|d| d.get_temp(egui::Id::new("command-palette-owner")))
}

fn claim_owner(ctx: &egui::Context, viewport: egui::ViewportId) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("command-palette-owner"), viewport));
}

fn release_owner(ctx: &egui::Context) {
    if owner(ctx) == Some(ctx.viewport_id()) {
        ctx.data_mut(|d| d.remove::<egui::ViewportId>(egui::Id::new("command-palette-owner")));
    }
}

fn filter<'a>(commands: &'a [Command], state: &State) -> Vec<&'a Command> {
    let mut result: Vec<_> = commands
        .iter()
        .filter(|c| c.enabled && c.matches(&state.search))
        .collect();
    result.sort_by_key(|c| state.history.rank(&state.query, &c.identity()));
    result
}
