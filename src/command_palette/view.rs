//! Layout and selection following cat-smf-editor; row drawing lives in row.rs.
use super::{filter, id, row, Command, State};
use eframe::egui;

pub(super) fn show<'a>(
    ctx: &egui::Context,
    commands: &'a [Command],
    filtered: &mut Vec<&'a Command>,
    state: &mut State,
    mut reveal: bool,
) {
    crate::japanese_font::add(ctx);
    let screen = ctx.screen_rect();
    let style = ctx.style();
    let margins = egui::Frame::popup(&style).total_margin().sum().x;
    if state.screen_rect != Some(screen) {
        state.geometry = None;
        state.screen_rect = Some(screen);
    }
    // Measure all candidates, so filtering never moves or narrows the query.
    let (width, left) = *state.geometry.get_or_insert_with(|| {
        let text_width = commands
            .iter()
            .filter(|c| c.enabled)
            .map(|c| row::width(ctx, c))
            .fold(0.0, f32::max);
        let width = (text_width
            + 2.0 * style.spacing.button_padding.x
            + style.spacing.scroll.allocated_width())
        .ceil()
        .min((screen.width() - 40.0 - margins).max(1.0));
        (width, screen.center().x - (width + margins) / 2.0)
    });
    let area = egui::Modal::default_area(id(ctx)).anchor(
        egui::Align2::LEFT_TOP,
        egui::vec2(left - screen.left(), 8.0),
    );
    egui::Modal::new(id(ctx)).area(area).show(ctx, |ui| {
        ui.set_width(width);
        // Area remembers the previous small result set. Reset its upper bound
        // every frame so clearing a filter can grow back to the full list.
        ui.set_max_height((screen.bottom() - ui.min_rect().top() - 24.0).max(1.0));
        let query_id = id(ctx).with("query");
        ctx.memory_mut(|m| m.request_focus(query_id));
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .id(query_id)
                .desired_width(f32::INFINITY),
        );
        response.request_focus();
        if response.changed() {
            state.selected = 0;
            reveal = true;
            state.search.update(&state.query);
            *filtered = filter(commands, state);
        }
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let position = if filtered.is_empty() {
                    0
                } else {
                    state.selected + 1
                };
                ui.label(
                    egui::RichText::new(format!("{position} / {}", filtered.len()))
                        .small()
                        .weak(),
                );
            });
        });
        egui::ScrollArea::vertical()
            .id_salt("palette-results")
            .max_height((screen.bottom() - ui.cursor().top() - 24.0).max(1.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (index, command) in filtered.iter().enumerate() {
                    let row = row::show(ui, command, index == state.selected);
                    if index == state.selected && reveal {
                        row.scroll_to_me(Some(egui::Align::Center));
                    }
                    if row.clicked() {
                        state.selected = index;
                    }
                }
            });
    });
}
