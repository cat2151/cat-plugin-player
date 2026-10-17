//! Virtual multi-column effect rows: plugin and kind tints, taken plugins dimmed.
use super::*;
use crate::{effect_catalog::Candidate, patch_browser_ui::row_colors};

/// Horizontal space between a cell edge and its label.
const LABEL_INSET: f32 = 6.0;
const PLAYING: &str = "▶ ";

impl EffectBrowser {
    pub(super) fn rows(&mut self, ui: &mut egui::Ui, view: &View) {
        ui.label(format!("Effects ({} results)", self.results.len()));
        if self.results.is_empty() {
            ui.label("No results. Edit search or choose another Category / Kind.");
        }
        let row_height = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
        let rows: Vec<_> = self.results.iter().map(|&i| &self.candidates[i]).collect();
        let tints = tints(&rows);
        let mixed = tints.iter().any(|(plugin, _)| plugin.is_some());
        let cell = if self.fit_columns {
            widest(ui, &rows, mixed) + 2.0 * LABEL_INSET
        } else {
            240.0
        };
        // Reserve scrollbar space so a column never disappears when scrolling starts.
        let gap = ui.spacing().item_spacing.x;
        let width = (ui.available_width() - ui.spacing().scroll.bar_width - gap).max(1.0);
        let columns = ((width + gap) / (cell + gap)).floor().max(1.0) as usize;
        let resized = self.list_columns != columns;
        self.list_columns = columns;
        let cell_width = (width - gap * (columns - 1) as f32) / columns as f32;
        let total_rows = self.results.len().div_ceil(columns);
        let height = ui.available_height().max(0.0);
        let stride = row_height + ui.spacing().item_spacing.y;
        let content_height = (total_rows as f32 * stride - ui.spacing().item_spacing.y).max(0.0);
        let max_offset = (content_height - height).max(0.0);
        let id = ui.make_persistent_id(egui::Id::new("effect_rows"));
        let mut offset = egui::scroll_area::State::load(ui.ctx(), id)
            .map_or(0.0, |state| state.offset.y)
            .clamp(0.0, max_offset);
        if self.scroll_to_selected || resized {
            if let Some(cell) = self
                .selected
                .and_then(|index| self.results.iter().position(|&item| item == index))
            {
                let top = (cell / columns) as f32 * stride;
                let bottom = top + row_height;
                // Reveal only the hidden portion; never align an already visible row to the top.
                if top < offset {
                    offset = top;
                } else if bottom > offset + height {
                    offset = bottom - height;
                }
            }
            self.scroll_to_selected = false;
        }
        let applied = self.applied_in_target();
        let mut select = None;
        egui::ScrollArea::vertical()
            .id_salt("effect_rows")
            .max_height(height)
            .vertical_scroll_offset(offset.clamp(0.0, max_offset))
            .auto_shrink([false, false])
            .show_rows(ui, row_height, total_rows, |ui, range| {
                for row in range {
                    ui.horizontal(|ui| {
                        for cell in row * columns..((row + 1) * columns).min(rows.len()) {
                            let index = self.results[cell];
                            let candidate = rows[cell];
                            let response = ui
                                .push_id(index, |ui| {
                                    ui.add_sized(
                                        [cell_width, row_height],
                                        egui::Button::new("")
                                            .selected(self.selected == Some(index)),
                                    )
                                })
                                .inner;
                            let taken = view.taken.contains(&candidate.plugin_id);
                            let playing = applied.is_some_and(|applied| {
                                crate::effect_browser::same(applied, candidate)
                            });
                            paint_row(ui, &response, candidate, tints[cell], mixed, taken, playing);
                            let mut hover = details(candidate);
                            if taken {
                                hover.push_str(&format!(
                                    "\n{} is already connected in another slot",
                                    candidate.plugin_name
                                ));
                            }
                            if response.on_hover_text(hover).clicked() {
                                select = Some(index);
                            }
                        }
                    });
                }
            });
        if let Some(index) = select {
            self.select(index);
            // Pointer selection already targets a visible cell.
            self.scroll_to_selected = false;
        }
    }
}

/// Palette slots per row: plugin when several plugins are listed, kind when kinds differ.
fn tints(rows: &[&Candidate]) -> Vec<(Option<usize>, Option<usize>)> {
    let plugins = row_colors::slots(rows.iter().map(|c| Some(&c.plugin_id)).collect());
    let kinds = row_colors::slots(rows.iter().map(|c| Some(&c.kind)).collect());
    (0..rows.len())
        .map(|at| {
            (
                plugins.as_ref().and_then(|slots| slots[at]),
                kinds.as_ref().and_then(|slots| slots[at]),
            )
        })
        .collect()
}

fn paint_row(
    ui: &egui::Ui,
    response: &egui::Response,
    candidate: &Candidate,
    (plugin, kind): (Option<usize>, Option<usize>),
    mixed: bool,
    taken: bool,
    playing: bool,
) {
    // Fixed-height virtual rows must not expand when a name wraps.
    let inner = response.rect.shrink2(egui::vec2(LABEL_INSET, 1.0));
    let painter = ui.painter().with_clip_rect(inner);
    let dark = ui.visuals().dark_mode;
    let plain = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let paint = |slot: Option<usize>| {
        if taken {
            weak
        } else {
            slot.map_or(plain, |slot| row_colors::color(slot, dark))
        }
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let marker = if playing { PLAYING } else { "" };
    let (prefix, name) = ui.fonts(|fonts| {
        let char_width = |c| fonts.glyph_width(&font, c);
        let room = inner.width() - marker.chars().map(char_width).sum::<f32>();
        let plugin = mixed.then_some(candidate.plugin_name.as_str());
        label::fit(plugin, &candidate.name, room, char_width)
    });
    let mut job = egui::text::LayoutJob::default();
    for (text, color) in [
        (marker, paint(None)),
        (prefix.as_str(), paint(plugin)),
        (name.as_str(), paint(kind)),
    ] {
        job.append(text, 0.0, egui::TextFormat::simple(font.clone(), color));
    }
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    let top = response.rect.center().y - galley.size().y / 2.0;
    painter.galley(egui::pos2(inner.left(), top), galley, plain);
}

/// Width of the longest whole label in `rows`, the playing marker included.
fn widest(ui: &egui::Ui, rows: &[&Candidate], mixed: bool) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    ui.fonts(|fonts| {
        let width = |text: &str| {
            text.chars()
                .map(|c| fonts.glyph_width(&font, c))
                .sum::<f32>()
        };
        let plugin = |candidate: &Candidate| {
            if mixed {
                width(&candidate.plugin_name) + width(label::PLUGIN_SEPARATOR)
            } else {
                0.0
            }
        };
        width(PLAYING)
            + rows
                .iter()
                .map(|candidate| plugin(candidate) + width(&candidate.name))
                .fold(0.0, f32::max)
    })
}

fn details(candidate: &Candidate) -> String {
    format!(
        "{}\n{}\nCategory: {} / Kind: {}",
        candidate.preset.display,
        candidate.preset.path.display(),
        candidate.category,
        candidate.kind
    )
}
