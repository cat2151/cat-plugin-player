use super::{CursorPreview, Notes};
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Confirm,
    Cancel,
}

/// The caller owns playback, confirmation, and persistence. Keyboard events are
/// consumed before TextEdit, so Enter confirms without appending a newline.
pub fn editor(
    ctx: &egui::Context,
    buffer: &mut String,
    focus: &mut bool,
    preview: &mut CursorPreview,
    error: Option<&str>,
    rate: u32,
    ready: bool,
) -> (Action, Option<Option<Notes>>) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        return (Action::Cancel, None);
    }
    let mut action =
        if ready && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            Action::Confirm
        } else {
            Action::None
        };
    let mut request = None;
    let modal = egui::Modal::new(egui::Id::new("mml-editor")).show(ctx, |ui| {
        ui.set_width(520.0);
        ui.heading("MML / Chord progression");
        let output = egui::TextEdit::multiline(buffer)
            .id_salt("mml_editor_text")
            .font(egui::TextStyle::Monospace)
            .desired_rows(5)
            .desired_width(f32::INFINITY)
            .show(ui);
        if ready {
            request = output.cursor_range.and_then(|range| {
                preview.refresh(
                    buffer,
                    range.primary.ccursor.index,
                    rate,
                    output.response.changed(),
                )
            });
        }
        if *focus {
            output.response.request_focus();
            *focus = false;
        }
        if let Some(error) = error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(ready, egui::Button::new("Enter: Confirm"))
                .clicked()
            {
                action = Action::Confirm;
            }
            if ui.button("Esc: Cancel").clicked() {
                action = Action::Cancel;
            }
        });
    });
    if modal.should_close() {
        action = Action::Cancel;
    }
    (action, request)
}
