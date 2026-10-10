//! Single transient editor for MML and chord notation.
use crate::{timed_sequence::Phrase, App, SequencePattern};
use eframe::egui;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct MmlInput {
    pub open: bool,
    pub confirmed: String,
    pub buffer: String,
    pub error: Option<String>,
    pub phrase: Option<Arc<Phrase>>,
    focus: bool,
    cursor_preview: crate::mml_preview::CursorPreview,
    sequence_before_open: Option<SequencePattern>,
}

impl MmlInput {
    pub fn restore_phrase(&mut self, rate: u32, selected: SequencePattern) -> Result<(), String> {
        self.phrase = if self.confirmed.trim().is_empty() {
            if selected == SequencePattern::Custom {
                return Err("Custom requires saved MML".into());
            }
            None
        } else {
            Some(Arc::new(Phrase::parse(&self.confirmed, rate)?))
        };
        Ok(())
    }
    pub fn open(&mut self) {
        self.cursor_preview = Default::default();
        self.buffer.clone_from(&self.confirmed);
        self.error = None;
        self.open = true;
        self.focus = true;
    }
    pub fn cancel(&mut self) {
        self.open = false;
        self.error = None;
        self.sequence_before_open = None;
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
    pub(crate) fn open_mml_editor(&mut self) {
        if self.mml_input.open {
            return;
        }
        self.mml_input.sequence_before_open =
            (self.sequence_pattern != SequencePattern::Off).then_some(self.sequence_pattern);
        self.stop_sequence_for_mml();
        self.mml_input.open();
    }

    fn cancel_mml(&mut self) {
        self.stop_mml_preview();
        if let Some(pattern) = self.mml_input.sequence_before_open {
            self.selected_sequence = pattern;
            self.sequence_pattern = pattern;
            for instance in &self.instances {
                if let Some(voice) = &instance.voice {
                    voice.set_sequence(pattern);
                }
            }
        }
        self.mml_input.cancel();
        self.save_session();
    }

    fn stop_sequence_for_mml(&mut self) {
        if self.sequence_pattern != SequencePattern::Off {
            self.selected_sequence = self.sequence_pattern;
            self.sequence_pattern = SequencePattern::Off;
            for instance in &self.instances {
                if let Some(voice) = &instance.voice {
                    voice.set_sequence(SequencePattern::Off);
                }
            }
            self.save_session();
        }
    }

    pub(crate) fn mml_editor(&mut self, ctx: &egui::Context) {
        if !self.mml_input.open
            && !ctx.wants_keyboard_input()
            && !self.actions_busy()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::I))
        {
            self.open_mml_editor();
            ctx.input_mut(|i| {
                i.events
                    .retain(|e| !matches!(e, egui::Event::Text(t) if t == "i" || t == "I"))
            });
        }
        if !self.mml_input.open {
            return;
        }
        self.stop_sequence_for_mml();
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.cancel_mml();
            return;
        }
        let ready = !self.actions_busy();
        let rate = self.sample_rate();
        let mut confirm =
            ready && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let mut cancel = false;
        egui::Window::new("MML / Chord progression")
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label("Enter: confirm   Esc: cancel");
                let output = (egui::TextEdit::multiline(&mut self.mml_input.buffer)
                    .id_salt("mml_editor_text")
                    .desired_rows(5)
                    .desired_width(f32::INFINITY))
                .show(ui);
                let response = output.response;
                if ready {
                    let request = output.cursor_range.and_then(|range| {
                        self.mml_input.cursor_preview.refresh(
                            &self.mml_input.buffer,
                            range.primary.ccursor.index,
                            rate,
                            response.changed(),
                        )
                    });
                    if let Some(notes) = request {
                        for instance in &self.instances {
                            if let Some(voice) = &instance.voice {
                                voice.preview(notes);
                            }
                        }
                    }
                }
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
            self.cancel_mml();
        } else if confirm {
            self.confirm_mml();
        }
    }

    pub(crate) fn stop_mml_preview(&self) {
        for instance in &self.instances {
            if let Some(voice) = &instance.voice {
                voice.preview(None);
            }
        }
    }

    fn confirm_mml(&mut self) {
        self.stop_mml_preview();
        let Some(phrase) = self.mml_input.convert(self.sample_rate()) else {
            return;
        };
        self.pause_audio();
        let old_pattern = self.sequence_pattern;
        let old_selected = self.selected_sequence;
        let old_phrase = self.mml_input.phrase.replace(phrase);
        self.selected_sequence = SequencePattern::Custom;
        if self.mml_input.sequence_before_open.is_some() {
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
        self.save_session();
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
