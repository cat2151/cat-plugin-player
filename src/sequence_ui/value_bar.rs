//! The value bar beside each sequence control; CC1 and macro bars also take drags.
use crate::SequenceModulation;
use eframe::egui;

/// What one frame of dragging a value bar changed.
pub(super) struct Drag {
    value: Option<u8>,
    held: bool,
    released: bool,
}

impl Drag {
    /// A dragged value becomes a fixed choice, which stops a sweep.
    pub(super) fn apply(
        self,
        choice: &mut Option<SequenceModulation>,
        held: &mut bool,
        released: &mut bool,
    ) {
        if let Some(value) = self.value {
            *choice = Some(SequenceModulation::Fixed(value));
        }
        *held |= self.held;
        *released |= self.released;
    }
}

/// Horizontal drag moves the value from where the drag started; the full
/// width covers 0-127.
pub(super) fn drag_bar(ui: &mut egui::Ui, value: u8) -> Drag {
    let response = value_bar(ui, value, egui::Sense::drag())
        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
        .on_hover_text("Drag horizontally to send a fixed value");
    let id = response.id;
    if response.drag_started() {
        ui.data_mut(|data| data.insert_temp(id, value));
    }
    let start = ui.data(|data| data.get_temp::<u8>(id));
    // A click alone keeps "-" and sweep.
    let moved = ui.input(|input| {
        let pointer = &input.pointer;
        pointer.is_decidedly_dragging().then_some(())?;
        Some(pointer.interact_pos()?.x - pointer.press_origin()?.x)
    });
    let value = match (response.dragged(), start, moved) {
        (true, Some(start), Some(moved)) => {
            let delta = moved / response.rect.width() * 127.0;
            Some((f32::from(start) + delta).round().clamp(0.0, 127.0) as u8)
        }
        _ => None,
    };
    Drag {
        value,
        held: response.dragged(),
        released: response.drag_stopped(),
    }
}

pub(super) fn value_bar(ui: &mut egui::Ui, value: u8, sense: egui::Sense) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(120.0, 18.0), sense);
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
    response.on_hover_text(format!("Current MIDI value: {value} / 127"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, event: egui::Event, value: u8) -> Drag {
        let input = egui::RawInput {
            events: vec![event],
            ..Default::default()
        };
        let mut drag = None;
        let _ = ctx.run(input, |ctx| {
            egui::Area::new(egui::Id::new("bar"))
                .fixed_pos(egui::Pos2::ZERO)
                .show(ctx, |ui| drag = Some(drag_bar(ui, value)));
        });
        drag.unwrap()
    }

    fn button(x: f32, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: egui::pos2(x, 9.0),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn horizontal_drag_moves_the_value_from_its_start_and_a_click_keeps_it() {
        let ctx = egui::Context::default();
        let at = |x| egui::Event::PointerMoved(egui::pos2(x, 9.0));
        frame(&ctx, at(10.0), 20);
        frame(&ctx, button(10.0, true), 20);
        let clicked = frame(&ctx, button(10.0, false), 20);
        assert_eq!(clicked.value, None);

        frame(&ctx, button(10.0, true), 20);
        // 30 of 120 points: a quarter of 127.
        let dragged = frame(&ctx, at(40.0), 20);
        assert_eq!((dragged.value, dragged.held), (Some(52), true));
        let left = frame(&ctx, at(-200.0), 20);
        assert_eq!(left.value, Some(0));
        let right = frame(&ctx, at(400.0), 20);
        assert_eq!(right.value, Some(127));
        let released = frame(&ctx, button(400.0, false), 20);
        assert_eq!((released.value, released.held), (None, false));
        assert!(released.released);
    }
}
