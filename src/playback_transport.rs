//! One main-window Play/Stop action, shared by the button, Space and palette.
use crate::{App, SequencePattern};
use eframe::egui;

pub(crate) struct Transport {
    playing: bool,
    enabled: bool,
    shortcut: bool,
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

impl Transport {
    pub(crate) fn button(&self, ui: &mut egui::Ui) -> bool {
        ui.add_enabled(
            self.enabled,
            egui::Button::new(if self.playing { "Stop" } else { "Play" }),
        )
        .on_hover_text(if self.playing {
            "Stop (Space)"
        } else {
            "Play (Space)"
        })
        .on_disabled_hover_text("Load an instrument with audio first; wait for loading to finish")
        .clicked()
            || self.shortcut
    }
}

impl App {
    /// Palette availability after navigation; direct input also checks child windows.
    pub(crate) fn main_playback_available(&self) -> bool {
        self.selection.is_none()
            && self.instances.iter().any(|i| i.voice.is_some())
            && (!self.drum_kit_active()
                || self
                    .main_drum_candidate()
                    .is_some_and(|c| !crate::drum_main_preview::main_notes(c).is_empty()))
    }

    pub(crate) fn main_transport(&self, ctx: &egui::Context) -> Transport {
        let enabled = self.main_playback_available()
            && !self.actions_busy()
            && !self.mml_input.open
            && !self.browser_open();
        let playing = if self.drum_kit_active() {
            self.drum_browser.main_audition
                && self
                    .instances
                    .iter()
                    .filter_map(|i| i.voice.as_ref())
                    .any(|v| v.drum_active())
        } else {
            self.sequence_pattern != SequencePattern::Off
        };
        Transport {
            playing,
            enabled,
            shortcut: take_transport_shortcut(ctx, enabled),
        }
    }

    pub(crate) fn toggle_main_playback(&mut self) {
        if self.drum_kit_active() {
            let playing = self.drum_browser.main_audition
                && self
                    .instances
                    .iter()
                    .filter_map(|i| i.voice.as_ref())
                    .any(|v| v.drum_active());
            if playing {
                self.stop_drum_audio();
            } else {
                self.play_main_drum_preview();
            }
        } else {
            self.sequence_pattern = if self.sequence_pattern == SequencePattern::Off {
                self.selected_sequence
            } else {
                SequencePattern::Off
            };
            for voice in self.instances.iter().filter_map(|i| i.voice.as_ref()) {
                voice.set_sequence(self.sequence_pattern);
            }
            self.save_session();
        }
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
}
