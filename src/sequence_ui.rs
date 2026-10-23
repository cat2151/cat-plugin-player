//! Sequence controls independent of their containing panel and placement.
use crate::{App, SequenceModulation, SequencePattern, SequenceVelocity};
use eframe::egui;
use value_bar::{drag_bar, value_bar};

mod value_bar;

/// The dropdowns' longest label is "MML / Chord"; the arrows take the saved width.
const CHOICE_WIDTH: f32 = 110.0;

/// Wide arrows beside a narrow dropdown are easy to hit.
fn arrow(ui: &mut egui::Ui, forward: bool, hover: &str) -> bool {
    let text = if forward { ">" } else { "<" };
    ui.add(egui::Button::new(text).min_size(egui::vec2(44.0, 0.0)))
        .on_hover_text(hover)
        .clicked()
}

/// Selecting a type preserves transport state; Play/Stop changes it explicitly.
fn selected_state(current: SequencePattern, selected: SequencePattern) -> SequencePattern {
    if current == SequencePattern::Off {
        SequencePattern::Off
    } else {
        selected
    }
}

// Consume repeats too, so a focused button cannot also react to Space.
fn take_transport_shortcut(ctx: &egui::Context, enabled: bool) -> bool {
    let editing_text = ctx
        .memory(|memory| memory.focused())
        .is_some_and(|id| egui::text_edit::TextEditState::load(ctx, id).is_some());
    if !enabled || editing_text {
        return false;
    }
    ctx.input_mut(|input| {
        let mut toggle = false;
        input.events.retain(|event| {
            if let egui::Event::Key {
                key: egui::Key::Space,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = event
            {
                if modifiers.is_none() {
                    toggle |= !repeat;
                    return false;
                }
            }
            true
        });
        toggle
    })
}

impl App {
    pub(crate) fn sequence_controls(&mut self, ui: &mut egui::Ui) {
        if let Some(selection) = &self.selection {
            let seconds = selection
                .position
                .load(std::sync::atomic::Ordering::Acquire) as f64
                / f64::from(self.sample_rate());
            ui.label(format!(
                "SMF loop · {seconds:.2}s · Esc / q / close: save and return"
            ));
            return;
        }
        let current = self.sequence_pattern;
        let original = current.selection(self.selected_sequence);
        let mut selected = original;
        let mut next = current;
        let original_velocity = self.sequence_velocity;
        let mut velocity = original_velocity;
        let original_modulation = self.sequence_modulation;
        let mut modulation = original_modulation;
        let original_macros = self.macro_lane.choices;
        let mut macro_choices = original_macros;
        // Saving and rereading the state wait for the drag to end.
        let mut dragging = false;
        let mut dropped = false;
        let ready = !self.actions_busy() && !self.mml_input.open && !self.browser_open();
        let mut open_mml = false;
        let has_audio = self.instances.iter().any(|i| i.voice.is_some());
        let shortcut =
            take_transport_shortcut(ui.ctx(), ready && has_audio && !self.mml_input.open);
        let [next_sequence, next_velocity, next_modulation] =
            [egui::Key::S, egui::Key::V, egui::Key::W]
                .map(|key| ready && self.main_shortcut(ui.ctx(), key));

        let (velocity_value, modulation_value, macro_values) = self
            .instances
            .iter()
            .find_map(|i| i.voice.as_ref().map(|v| v.current_values()))
            .unwrap_or_default();

        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Sequence");
                if ui
                    .add_enabled(ready, egui::Button::new("Chord"))
                    .on_hover_text(
                        "Chord / MML input\nShortcut: i\nEnter chord notation and MML in the same field",
                    )
                    .clicked()
                {
                    open_mml = true;
                }
                ui.add_enabled_ui(ready, |ui| {
                    if arrow(ui, false, "Previous sequence") {
                        selected = selected.adjacent(false);
                    }
                    egui::ComboBox::from_id_salt("sequence_type")
                        .selected_text(selected.label())
                        .width(CHOICE_WIDTH)
                        .show_ui(ui, |ui| {
                            for &pattern in SequencePattern::TYPES {
                                ui.selectable_value(&mut selected, pattern, pattern.label());
                            }
                            if self.mml_input.phrase.is_some() {
                                ui.selectable_value(
                                    &mut selected,
                                    SequencePattern::Custom,
                                    SequencePattern::Custom.label(),
                                );
                            }
                        });
                    if arrow(ui, true, "Next sequence (S)") || next_sequence {
                        selected = selected.adjacent(true);
                    }
                });
                if selected != original {
                    next = selected_state(current, selected);
                }
                let playing = current != SequencePattern::Off;
                if ui
                    .add_enabled(
                        ready && has_audio,
                        egui::Button::new(if playing { "Stop" } else { "Play" }),
                    )
                    .on_hover_text(if playing {
                        "Stop sequence (Space)"
                    } else {
                        "Play selected sequence (Space)"
                    })
                    .on_disabled_hover_text(
                        "Load an instrument with audio first; wait for loading to finish",
                    )
                    .clicked()
                    || shortcut
                {
                    next = if playing {
                        SequencePattern::Off
                    } else {
                        selected
                    };
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Velocity");
                ui.add_enabled_ui(ready, |ui| {
                    if arrow(ui, false, "Previous velocity") {
                        velocity = velocity.adjacent(false);
                    }
                    egui::ComboBox::from_id_salt("sequence_velocity")
                        .selected_text(velocity.label())
                        .width(CHOICE_WIDTH)
                        .show_ui(ui, |ui| {
                            for &choice in SequenceVelocity::TYPES {
                                ui.selectable_value(&mut velocity, choice, choice.label());
                            }
                        })
                        .response
                        .on_hover_text("Ranges: one phrase up, the next phrase down; repeat");
                    if arrow(ui, true, "Next velocity (V)") || next_velocity {
                        velocity = velocity.adjacent(true);
                    }
                });
                value_bar(ui, velocity_value, egui::Sense::hover());
            });
            // "-" sends nothing: show the plugin's own value when its state reports it.
            let shown = |choice: Option<SequenceModulation>, own: Option<u8>, sent| {
                choice.map_or(own, |_| None).unwrap_or(sent)
            };
            let controls = self.macro_lane.controls.clone();
            if ready
                && controls.ccs.iter().any(Option::is_some)
                && self.main_shortcut(ui.ctx(), egui::Key::M)
            {
                SequenceModulation::toggle_sweep(&mut macro_choices, &controls.ccs);
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("CC1 modulation");
                ui.add_enabled_ui(ready, |ui| {
                    if next_modulation {
                        modulation = SequenceModulation::adjacent(modulation, true);
                    }
                    modulation_selector(
                        ui,
                        "sequence_modulation",
                        &mut modulation,
                        "-: not sent; the patch's modulation wheel stays
sweep: 0 -> 127 in 2 seconds, then 127 -> 0 in 2 seconds; repeat
w: next choice",
                    )
                });
                let value = shown(modulation, controls.wheel, modulation_value);
                let drag = ui.add_enabled_ui(ready, |ui| drag_bar(ui, value)).inner;
                drag.apply(&mut modulation, &mut dragging, &mut dropped);
            });
            for (index, cc) in controls.ccs.iter().enumerate() {
                let Some(cc) = cc else { continue };
                ui.horizontal_wrapped(|ui| {
                    let number = format!("CC{cc} {}", controls.labels[index]);
                    match &controls.names[index] {
                        Some(name) => ui.label(name).on_hover_text(number),
                        None => ui.label(number),
                    };
                    ui.add_enabled_ui(ready, |ui| {
                        modulation_selector(
                            ui,
                            ("sequence_macro_modulation", index),
                            &mut macro_choices[index],
                            "This macro of the patch, sent as MIDI CC
-: not sent; the patch's macro value stays
sweep: 0 -> 127 in 4 seconds, then 127 -> 0 in 4 seconds; repeat
m: every macro to sweep, or back to - once all sweep",
                        )
                    });
                    let own = controls.macros[index];
                    let value = shown(macro_choices[index], own, macro_values[index]);
                    let drag = ui.add_enabled_ui(ready, |ui| drag_bar(ui, value)).inner;
                    drag.apply(&mut macro_choices[index], &mut dragging, &mut dropped);
                });
            }
        });

        if !self.actions_busy()
            && (selected != original
                || next != current
                || velocity != original_velocity
                || modulation != original_modulation
                || dropped)
        {
            self.selected_sequence = selected;
            self.sequence_pattern = next;
            self.sequence_velocity = velocity;
            self.sequence_modulation = modulation;
            // A choice of "-" shows the value the plugin keeps now.
            self.macro_lane.reread |= modulation != original_modulation && modulation.is_none();
            for instance in &self.instances {
                if let Some(voice) = &instance.voice {
                    if next != current {
                        voice.set_sequence(next);
                    }
                    voice.set_velocity(velocity);
                    voice.set_modulation(modulation);
                }
            }
            if !dragging {
                self.save_session();
            }
        }
        if !self.actions_busy() && macro_choices != original_macros {
            self.macro_lane.choices = macro_choices;
            self.macro_lane.reread |= macro_choices
                .iter()
                .zip(original_macros)
                .any(|(now, was)| now.is_none() && was.is_some());
            self.apply_macro_lane();
            if !dragging {
                self.save_session();
            }
        }
        if open_mml {
            self.open_mml_editor();
        }
    }
}

/// A patch change sets fixed values back to "-" (ADR 0036).
fn modulation_selector(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    choice: &mut Option<SequenceModulation>,
    hover: &str,
) {
    if arrow(ui, false, "Previous modulation") {
        *choice = SequenceModulation::adjacent(*choice, false);
    }
    egui::ComboBox::from_id_salt(id)
        .selected_text(SequenceModulation::label(*choice))
        .width(CHOICE_WIDTH)
        .show_ui(ui, |ui| {
            for value in SequenceModulation::CHOICES {
                ui.selectable_value(choice, value, SequenceModulation::label(value));
            }
        })
        .response
        .on_hover_text(hover);
    if arrow(ui, true, "Next modulation") {
        *choice = SequenceModulation::adjacent(*choice, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space(repeat: bool, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers,
        }
    }

    #[test]
    fn space_shortcut_consumes_press_and_ignores_repeat_modifiers_and_disabled_transport() {
        let ctx = egui::Context::default();
        for (event, enabled, expected, consumed) in [
            (space(false, egui::Modifiers::NONE), true, true, true),
            (space(true, egui::Modifiers::NONE), true, false, true),
            (space(false, egui::Modifiers::CTRL), true, false, false),
            (space(false, egui::Modifiers::NONE), false, false, false),
        ] {
            let input = egui::RawInput {
                events: vec![event],
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                assert_eq!(take_transport_shortcut(ctx, enabled), expected);
                assert_eq!(ctx.input(|i| i.events.is_empty()), consumed);
                assert!(!take_transport_shortcut(ctx, enabled));
            });
        }
    }

    #[test]
    fn space_in_focused_text_edit_remains_available_for_typing() {
        let ctx = egui::Context::default();
        let mut text = String::new();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.text_edit_singleline(&mut text).request_focus();
            });
        });
        let input = egui::RawInput {
            events: vec![
                space(false, egui::Modifiers::NONE),
                egui::Event::Text(" ".into()),
            ],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            assert!(!take_transport_shortcut(ctx, true));
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.text_edit_singleline(&mut text);
            });
        });
        assert_eq!(text, " ");
    }

    #[test]
    fn selecting_while_stopped_does_not_start_playback() {
        assert_eq!(
            selected_state(SequencePattern::Off, SequencePattern::GuitarArpeggio),
            SequencePattern::Off
        );
        assert_eq!(
            selected_state(SequencePattern::Steps, SequencePattern::GuitarArpeggio),
            SequencePattern::GuitarArpeggio
        );
    }
}
