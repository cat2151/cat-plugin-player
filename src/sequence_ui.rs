//! Sequence controls independent of their containing panel and placement.
use crate::{App, SequenceModulation, SequencePattern, SequenceVelocity};
use eframe::egui;

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
        let current = self.sequence_pattern;
        let original = current.selection(self.selected_sequence);
        let mut selected = original;
        let mut next = current;
        let original_velocity = self.sequence_velocity;
        let mut velocity = original_velocity;
        let original_modulation = self.sequence_modulation;
        let mut modulation = original_modulation;
        let ready = !self.actions_busy();
        let has_audio = self.instances.iter().any(|i| i.voice.is_some());
        let shortcut =
            take_transport_shortcut(ui.ctx(), ready && has_audio && !self.mml_input.open);

        let (velocity_value, modulation_value) = self
            .instances
            .iter()
            .find_map(|i| i.voice.as_ref().map(|v| v.current_values()))
            .unwrap_or((0, 0));

        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Sequence");
                if ui
                    .add_enabled(ready, egui::Button::new("MML / Chord (i)"))
                    .clicked()
                {
                    self.mml_input.open();
                }
                ui.add_enabled_ui(ready, |ui| {
                    if ui.button("<").on_hover_text("Previous sequence").clicked() {
                        selected = selected.adjacent(false);
                    }
                    egui::ComboBox::from_id_salt("sequence_type")
                        .selected_text(selected.label())
                        .width(170.0)
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
                    if ui.button(">").on_hover_text("Next sequence").clicked() {
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
                    if ui.button("<").on_hover_text("Previous velocity").clicked() {
                        velocity = velocity.adjacent(false);
                    }
                    egui::ComboBox::from_id_salt("sequence_velocity")
                        .selected_text(velocity.label())
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for &choice in SequenceVelocity::TYPES {
                                ui.selectable_value(&mut velocity, choice, choice.label());
                            }
                        })
                        .response
                        .on_hover_text("Ranges: one phrase up, the next phrase down; repeat");
                    if ui.button(">").on_hover_text("Next velocity").clicked() {
                        velocity = velocity.adjacent(true);
                    }
                });
                value_bar(ui, velocity_value);
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("CC1 modulation");
                ui.add_enabled_ui(ready, |ui| {
                    if ui
                        .button("<")
                        .on_hover_text("Previous modulation")
                        .clicked()
                    {
                        modulation = modulation.adjacent(false);
                    }
                    egui::ComboBox::from_id_salt("sequence_modulation")
                        .selected_text(modulation.label())
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for &choice in SequenceModulation::TYPES {
                                ui.selectable_value(&mut modulation, choice, choice.label());
                            }
                        })
                        .response
                        .on_hover_text(
                            "sweep: 0 -> 127 in 2 seconds, then 127 -> 0 in 2 seconds; repeat",
                        );
                    if ui.button(">").on_hover_text("Next modulation").clicked() {
                        modulation = modulation.adjacent(true);
                    }
                });
                value_bar(ui, modulation_value);
            });
        });

        if !self.actions_busy()
            && (selected != original
                || next != current
                || velocity != original_velocity
                || modulation != original_modulation)
        {
            self.selected_sequence = selected;
            self.sequence_pattern = next;
            self.sequence_velocity = velocity;
            self.sequence_modulation = modulation;
            for instance in &self.instances {
                if let Some(voice) = &instance.voice {
                    if next != current {
                        voice.set_sequence(next);
                    }
                    voice.set_velocity(velocity);
                    voice.set_modulation(modulation);
                }
            }
            self.save_session();
        }
    }
}

fn value_bar(ui: &mut egui::Ui, value: u8) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(120.0, 18.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
    if value > 0 {
        let mut fill = rect;
        fill.max.x = fill.min.x + rect.width() * f32::from(value) / 127.0;
        ui.painter()
            .rect_filled(fill, 2.0, ui.visuals().selection.bg_fill);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        value.to_string(),
        egui::FontId::monospace(12.0),
        ui.visuals().text_color(),
    );
    response.on_hover_text(format!("Current MIDI value: {value} / 127"));
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
