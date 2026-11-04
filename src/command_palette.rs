//! Per-viewport command search. Background processing continues with inert UI input.
//! Commands replay the existing shortcut path on the next frame, after revalidation.
use eframe::egui::{self, Event, Key, Modifiers};

mod catalog;
#[cfg(test)]
mod layout_tests;
mod migemo;
mod search;
#[cfg(test)]
mod search_tests;
mod view;
pub(crate) use catalog::{effect_commands, main_commands, patch_commands};
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub(crate) struct Command {
    label: &'static str,
    japanese: &'static str,
    words: &'static str,
    shortcut: &'static str,
    keys: Vec<Key>,
    enabled: bool,
}

impl Command {
    fn new(
        label: &'static str,
        japanese: &'static str,
        words: &'static str,
        shortcut: &'static str,
        keys: &[Key],
        enabled: bool,
    ) -> Self {
        Self {
            label,
            japanese,
            words,
            shortcut,
            keys: keys.to_vec(),
            enabled,
        }
    }

    fn matches(&self, query: &search::SearchQuery) -> bool {
        let text = format!(
            "{} {} {} {}",
            self.label, self.japanese, self.words, self.shortcut
        );
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
    composing: bool,
    handled: Option<u64>,
}

pub(crate) struct Suspended {
    input: egui::InputState,
    commands: Vec<Command>,
}

fn id(ctx: &egui::Context) -> egui::Id {
    egui::Id::new("command-palette").with(ctx.viewport_id())
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
            if let Some(command) = commands.iter().find(|c| c.label == label && c.enabled) {
                if let Some(focus) = ctx.memory(|m| m.focused()) {
                    ctx.memory_mut(|m| m.surrender_focus(focus));
                }
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
    } else if !state.open && toggle {
        state = State {
            open: true,
            focus: ctx.memory(|m| m.focused()),
            ..Default::default()
        };
        ctx.memory_mut(|m| m.close_popup());
        clear(ctx, true);
    }
    let suspended = state.open.then(|| {
        let input = ctx.input(Clone::clone);
        clear(ctx, true);
        Suspended { input, commands }
    });
    save(ctx, state);
    suspended
}

/// After painting background widgets, restore the real input only for the palette.
pub(crate) fn end(ctx: &egui::Context, suspended: Option<Suspended>) {
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
    let mut filtered: Vec<_> = commands
        .iter()
        .filter(|c| c.matches(&state.search))
        .collect();
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
                        Key::Enter if !repeat && !search_input => execute = true,
                        Key::Escape if !repeat => cancel = true,
                        _ => {}
                    }
                }
                !matches!(key, Key::Enter | Key::Escape)
            });
        });
    }
    let mut chosen = if execute {
        filtered
            .get(state.selected)
            .filter(|c| c.enabled)
            .map(|c| c.label)
    } else {
        None
    };
    if !cancel && chosen.is_none() {
        chosen = view::show(ctx, &commands, &mut filtered, &mut state, reveal);
    }
    if cancel || chosen.is_some() {
        state.open = false;
        state.pending = chosen;
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
