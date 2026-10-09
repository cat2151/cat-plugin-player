//! Single transient editor for MML and chord notation.
use crate::{timed_sequence::Phrase, App, SequencePattern};
use eframe::egui;
use std::sync::Arc;

#[derive(Default)]
pub struct MmlInput {
    pub open: bool,
    pub confirmed: String,
    pub buffer: String,
    pub error: Option<String>,
    pub phrase: Option<Arc<Phrase>>,
    focus: bool,
}

impl MmlInput {
    pub fn open(&mut self) {
        self.buffer.clone_from(&self.confirmed);
        self.error = None;
        self.open = true;
        self.focus = true;
    }
    pub fn cancel(&mut self) {
        self.open = false;
        self.error = None;
    }
    pub fn convert(&mut self, rate: u32) -> Option<Arc<Phrase>> {
        match Phrase::parse(&self.buffer, rate) {
            Ok(phrase) => {
                self.error = None;
                Some(Arc::new(phrase))
            }
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }
}

impl App {
    pub(crate) fn mml_editor(&mut self, ctx: &egui::Context) {
        if !self.mml_input.open
            && !ctx.wants_keyboard_input()
            && !self.actions_busy()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::I))
        {
            self.mml_input.open();
            ctx.input_mut(|i| {
                i.events
                    .retain(|e| !matches!(e, egui::Event::Text(t) if t == "i" || t == "I"))
            });
        }
        if !self.mml_input.open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.mml_input.cancel();
            return;
        }
        let ready = !self.actions_busy();
        let mut confirm =
            ready && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let mut cancel = false;
        egui::Window::new("MML / Chord progression")
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label("Enter: confirm   Esc: cancel");
                let response = ui.add(
                    egui::TextEdit::multiline(&mut self.mml_input.buffer)
                        .id_salt("mml_editor_text")
                        .desired_rows(5)
                        .desired_width(f32::INFINITY),
                );
                if self.mml_input.focus {
                    response.request_focus();
                    self.mml_input.focus = false;
                }
                if let Some(error) = &self.mml_input.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.horizontal(|ui| {
                    confirm |= ui
                        .add_enabled(ready, egui::Button::new("Confirm"))
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if cancel {
            self.mml_input.cancel();
        } else if confirm {
            self.confirm_mml();
        }
    }

    fn confirm_mml(&mut self) {
        let Some(phrase) = self.mml_input.convert(self.sample_rate()) else {
            return;
        };
        self.pause_audio();
        let old_pattern = self.sequence_pattern;
        let old_selected = self.selected_sequence;
        let old_phrase = self.mml_input.phrase.replace(phrase);
        self.selected_sequence = SequencePattern::Custom;
        if old_pattern != SequencePattern::Off {
            self.sequence_pattern = SequencePattern::Custom;
        }
        if let Err(error) = self.resume_audio() {
            self.mml_input.phrase = old_phrase;
            self.sequence_pattern = old_pattern;
            self.selected_sequence = old_selected;
            let restored = self.resume_audio();
            self.mml_input.error = Some(format!(
                "{error}{}",
                restored
                    .err()
                    .map_or(String::new(), |e| format!("; rollback audio: {e}"))
            ));
            return;
        }
        self.mml_input.confirmed.clone_from(&self.mml_input.buffer);
        self.mml_input.cancel();
        self.status = "MML / chord phrase ready (Play / Stop)".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_and_failed_conversion_preserve_the_confirmed_phrase() {
        let original = Arc::new(Phrase::parse("ceg", 48000).unwrap());
        let mut input = MmlInput {
            confirmed: "ceg".into(),
            phrase: Some(original.clone()),
            ..Default::default()
        };
        input.open();
        input.buffer = "C F G".into();
        input.cancel();
        assert!(Arc::ptr_eq(input.phrase.as_ref().unwrap(), &original));
        input.open();
        assert_eq!(input.buffer, "ceg");
        input.buffer = "   ".into();
        assert!(input.convert(48000).is_none());
        assert!(input.error.is_some());
        assert_eq!(input.confirmed, "ceg");
        assert!(Arc::ptr_eq(input.phrase.as_ref().unwrap(), &original));
    }
}
