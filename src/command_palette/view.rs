//! Screen-bounded layout following cat-smf-editor's palette growth policy.
use super::{id, Command, State};
use eframe::egui;

fn label(command: &Command) -> String {
    format!("{} / {}", command.label, command.japanese)
}

fn command_row(ui: &mut egui::Ui, command: &Command, selected: bool) -> egui::Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let shortcut = ui.painter().layout_no_wrap(
        command.shortcut.into(),
        font,
        egui::Color32::from_rgb(224, 228, 235),
    );
    let badge_size = shortcut.size() + egui::vec2(12.0, 4.0);
    let padding = ui.spacing().button_padding;
    let row = ui.add_enabled(
        command.enabled,
        egui::Button::new("")
            .min_size(egui::vec2(
                ui.available_width(),
                badge_size.y + 2.0 * padding.y,
            ))
            .selected(selected),
    );
    row.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, command.enabled, label(command))
    });
    let badge = egui::Rect::from_min_size(
        egui::pos2(
            row.rect.right() - padding.x - badge_size.x,
            row.rect.center().y - badge_size.y / 2.0,
        ),
        badge_size,
    );
    let text = egui::WidgetText::from(label(command)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        (badge.left() - row.rect.left() - padding.x - ui.spacing().item_spacing.x).max(0.0),
        egui::TextStyle::Button,
    );
    let text_color = if selected && command.enabled {
        ui.visuals().selection.stroke.color
    } else {
        ui.style().interact(&row).text_color()
    };
    ui.painter().galley(
        egui::pos2(
            row.rect.left() + padding.x,
            row.rect.center().y - text.size().y / 2.0,
        ),
        text,
        text_color,
    );
    ui.painter().rect(
        badge,
        4,
        egui::Color32::from_rgb(20, 40, 70),
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(80, 109, 146)),
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        badge.min + egui::vec2(6.0, 2.0),
        shortcut,
        egui::Color32::WHITE,
    );
    row
}

pub(super) fn show<'a>(
    ctx: &egui::Context,
    commands: &'a [Command],
    filtered: &mut Vec<&'a Command>,
    state: &mut State,
    mut reveal: bool,
) -> Option<&'static str> {
    crate::japanese_font::add(ctx);
    let screen = ctx.screen_rect();
    let style = ctx.style();
    let margins = egui::Frame::popup(&style).total_margin().sum().x;
    // Measure all candidates, so filtering never moves or narrows the query.
    let text_width = ctx.fonts(|fonts| {
        commands
            .iter()
            .map(|command| {
                fonts
                    .layout_no_wrap(
                        format!("{}    {}", label(command), command.shortcut),
                        egui::TextStyle::Button.resolve(&style),
                        egui::Color32::WHITE,
                    )
                    .size()
                    .x
            })
            .fold(0.0, f32::max)
    });
    let width = (text_width
        + 12.0 // Horizontal padding inside the key binding badge.
        + 2.0 * style.spacing.button_padding.x
        + style.spacing.scroll.allocated_width())
    .ceil()
    .max(350.0)
    .min((screen.width() - 32.0 - margins).max(1.0));
    let left = screen.center().x - (width + margins) / 2.0;
    let area = egui::Modal::default_area(id(ctx)).anchor(
        egui::Align2::LEFT_TOP,
        egui::vec2(left - screen.left(), 8.0),
    );
    let mut chosen = None;
    egui::Modal::new(id(ctx)).area(area).show(ctx, |ui| {
        ui.set_width(width);
        // Area remembers the previous small result set. Reset its upper bound
        // every frame so clearing a filter can grow back to the full list.
        ui.set_max_height((screen.bottom() - ui.min_rect().top() - 16.0).max(1.0));
        ui.heading("Command palette / コマンドパレット");
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .id(id(ctx).with("query"))
                .hint_text("Search commands / コマンドを検索")
                .desired_width(f32::INFINITY),
        );
        response.request_focus();
        if response.changed() {
            state.selected = 0;
            reveal = true;
            state.search.update(&state.query);
            *filtered = commands
                .iter()
                .filter(|c| c.matches(&state.search))
                .collect();
        }
        ui.weak("↑ ↓ / PgUp PgDn: 選択 · Ctrl+Home/End: 先頭/末尾");
        ui.weak("Enter / click: 実行 · Esc: 閉じる");
        ui.label(format!(
            "{} / {} commands / コマンド",
            if filtered.is_empty() {
                0
            } else {
                state.selected + 1
            },
            filtered.len()
        ));
        egui::ScrollArea::vertical()
            .id_salt("palette-results")
            .max_height((screen.bottom() - ui.cursor().top() - 16.0).max(1.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (index, command) in filtered.iter().enumerate() {
                    let row = command_row(ui, command, index == state.selected)
                        .on_hover_text(label(command))
                        .on_disabled_hover_text(
                            "Unavailable in the current state / 現在は実行できません",
                        );
                    if index == state.selected && reveal {
                        row.scroll_to_me(Some(egui::Align::Center));
                    }
                    if row.clicked() {
                        chosen = Some(command.label);
                    }
                }
                if filtered.is_empty() {
                    ui.label("No matching commands / 該当するコマンドはありません");
                }
            });
    });
    chosen
}
