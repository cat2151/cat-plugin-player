//! Cached row names and visible-row-only painting; native identity is never shortened.
use super::*;
use crate::patch_browser_catalog::Snapshot;
use crate::patch_catalog::Candidate;
use row_colors::RowTint;
use std::collections::{HashMap, HashSet};

impl Browser {
    pub(crate) fn refresh_row_labels(&mut self) {
        self.widest_label = None;
        let Some(snapshot) = &self.snapshot else {
            self.row_labels.clear();
            self.row_tints.clear();
            return;
        };
        let labels = prefixed_labels(&snapshot.candidates, &self.results);
        // Only the role's ALL spans several presets; a chosen preset is one group by definition.
        let presets = (self.preset == 0).then(|| snapshot.presets[self.role].as_slice());
        let mixed = mixed_plugins(&snapshot.candidates, &self.results);
        self.row_tints = row_colors::tints(&snapshot.candidates, &self.results, presets, mixed)
            .into_iter()
            .zip(&labels)
            .map(|(tint, &(prefix_len, _))| RowTint { prefix_len, ..tint })
            .collect();
        self.row_labels = labels.into_iter().map(|(_, label)| label).collect();
    }
    pub(super) fn rows(&mut self, ui: &mut egui::Ui) {
        if self.snapshot.is_none() {
            return;
        }
        ui.label(format!("Patches ({} results)", self.results.len()));
        if self.results.is_empty() {
            ui.label("No results. Edit search or choose another Role / Preset.");
        }
        let row_height = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
        // Reserve scrollbar space so a column never disappears when scrolling starts.
        let gap = ui.spacing().item_spacing.x;
        let width = (ui.available_width() - ui.spacing().scroll.bar_width - gap).max(1.0);
        let cell = if self.fit_columns {
            self.widest_label(ui) + 2.0 * LABEL_INSET
        } else {
            240.0
        };
        let columns = ((width + gap) / (cell + gap)).floor().max(1.0) as usize;
        let resized = self.list_columns != columns;
        self.list_columns = columns;
        let cell_width = (width - gap * (columns - 1) as f32) / columns as f32;
        let total_rows = self.results.len().div_ceil(columns);
        let mut select = None;
        let mut random = false;
        let mut hover = None;
        let height = ui.available_height().max(0.0);
        let stride = row_height + ui.spacing().item_spacing.y;
        let content_height = (total_rows as f32 * stride - ui.spacing().item_spacing.y).max(0.0);
        let max_offset = (content_height - height).max(0.0);
        let id = ui.make_persistent_id(egui::Id::new("patch_rows"));
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
        // egui clamps after painting. Clamp here too so even the first frame is valid.
        let scroll = egui::ScrollArea::vertical()
            .id_salt("patch_rows")
            .max_height(height)
            .vertical_scroll_offset(offset.clamp(0.0, max_offset))
            .auto_shrink([false, false]);
        let output = scroll.show_rows(ui, row_height, total_rows, |ui, range| {
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("painted_patch_rows"),
                    range.start * columns..(range.end * columns).min(self.results.len()),
                )
            });
            for row in range {
                ui.horizontal(|ui| {
                    for cell in row * columns..((row + 1) * columns).min(self.results.len()) {
                        let index = self.results[cell];
                        let candidate = &self.snapshot.as_ref().unwrap().candidates[index];
                        let text = &self.row_labels[cell];
                        let response = cell_response(ui, index, cell_width, row_height);
                        let tint = self.row_tints[cell];
                        paint_cell(ui, &response, self.selected == Some(index), tint.background);
                        // Fixed-height virtual rows must not expand when a path wraps.
                        let painter = ui
                            .painter()
                            .with_clip_rect(response.rect.shrink2(egui::vec2(LABEL_INSET, 1.0)));
                        let dark = ui.visuals().dark_mode;
                        let plain = cell_text(dark);
                        let paint = |slot: Option<usize>| {
                            slot.map_or(plain, |slot| row_colors::color(slot, dark))
                        };
                        let name_color = if candidate.measurement.is_heavy_offline_load() {
                            PINK
                        } else {
                            tint.name.map_or(plain, |slot| {
                                plain.lerp_to_gamma(row_colors::color(slot, dark), NAME_TINT)
                            })
                        };
                        let font = egui::TextStyle::Body.resolve(ui.style());
                        let mut job = egui::text::LayoutJob::default();
                        let (prefix, name) = text.split_at(tint.prefix_len);
                        for (part, color) in [(prefix, paint(tint.plugin)), (name, name_color)] {
                            job.append(part, 0.0, egui::TextFormat::simple(font.clone(), color));
                        }
                        let galley = ui.fonts(|fonts| fonts.layout_job(job));
                        let top = response.rect.center().y - galley.size().y / 2.0;
                        painter.galley(
                            egui::pos2(response.rect.left() + LABEL_INSET, top),
                            galley,
                            plain,
                        );
                        if response.hovered() {
                            hover = Some(hover_tooltip(&response, index, self.hover_since, || {
                                details(self.snapshot.as_ref().unwrap(), index)
                            }));
                        }
                        if response.clicked() {
                            select = Some(index);
                        }
                        random |= response.secondary_clicked();
                    }
                });
            }
        });
        #[cfg(test)]
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::new("patch_scroll"),
                (output.inner_rect, output.state.offset.y),
            );
        });
        #[cfg(not(test))]
        let _ = output;
        if let Some(index) = select {
            self.select(index);
            // Pointer selection already targets a visible cell, including a partial edge row.
            self.scroll_to_selected = false;
        }
        self.hover_since = hover;
        if random {
            self.select_random();
        }
    }

    /// Width of the longest row label, measured once per result list.
    fn widest_label(&mut self, ui: &egui::Ui) -> f32 {
        if let Some(width) = self.widest_label {
            return width;
        }
        let font = egui::TextStyle::Body.resolve(ui.style());
        let width = ui.fonts(|fonts| {
            self.row_labels
                .iter()
                .map(|label| label.chars().map(|c| fonts.glyph_width(&font, c)).sum())
                .fold(0.0, f32::max)
        });
        self.widest_label = Some(width);
        width
    }
}

/// Horizontal space between a cell edge and its label.
const LABEL_INSET: f32 = 6.0;

/// A plain click target: unlike a button it shows no press, so a right click looks inert
/// until it acts, and the identity keeps its tooltip across scrolling.
fn cell_response(ui: &mut egui::Ui, index: usize, width: f32, height: f32) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    ui.interact(
        rect,
        ui.id().with(("patch_cell", index)),
        egui::Sense::click(),
    )
}

/// Shows the cell's tooltip once the pointer has rested on it for the tooltip delay,
/// returning when that rest began. egui's own rule waits for pointer movement after a click,
/// which it cannot see at the browser's low frame rate while the main window is hidden.
fn hover_tooltip(
    response: &egui::Response,
    index: usize,
    previous: Option<(usize, f64)>,
    text: impl FnOnce() -> String,
) -> (usize, f64) {
    let now = response.ctx.input(|input| input.time);
    let pressed = response.clicked() || response.secondary_clicked();
    let since = previous
        .filter(|&(at, _)| at == index && !pressed)
        .map_or(now, |(_, since)| since);
    if now - since >= f64::from(response.ctx.style().interaction.tooltip_delay) {
        response.show_tooltip_text(text());
    }
    (index, since)
}

/// Hovering never changes a cell; only the selected (playing) cell is lit, with a slow
/// pulse, so the pointer is never mistaken for the chosen patch.
fn paint_cell(ui: &egui::Ui, response: &egui::Response, selected: bool, group: Option<usize>) {
    let visuals = ui.visuals();
    let radius = visuals.widgets.inactive.corner_radius;
    if !selected {
        ui.painter()
            .rect_filled(response.rect, radius, cell_fill(visuals.dark_mode, group));
        return;
    }
    let time = ui.input(|input| input.time);
    let pulse = (0.5 + 0.5 * (time * std::f64::consts::TAU / PULSE_SECONDS).sin()) as f32;
    let glow = visuals.selection.stroke.color;
    let fill = visuals
        .selection
        .bg_fill
        .lerp_to_gamma(glow, 0.1 + 0.25 * pulse);
    ui.painter().rect_filled(response.rect, radius, fill);
    ui.painter().rect_stroke(
        response.rect,
        radius,
        egui::Stroke::new(2.0_f32, glow.gamma_multiply(0.4 + 0.6 * pulse)),
        egui::StrokeKind::Outside,
    );
}

/// Far from the text colors, faintly tinted by the row's preset so presets in ALL separate.
fn cell_fill(dark: bool, group: Option<usize>) -> Color32 {
    let base = if dark {
        Color32::from_gray(32)
    } else {
        Color32::from_gray(244)
    };
    group.map_or(base, |slot| {
        base.lerp_to_gamma(row_colors::color(slot, dark), GROUP_TINT)
    })
}

/// Text kept far from the cell fill in both themes.
fn cell_text(dark: bool) -> Color32 {
    if dark {
        Color32::from_gray(215)
    } else {
        Color32::from_gray(25)
    }
}

/// How much of the parent folder's color tints a patch name.
const NAME_TINT: f32 = 0.45;

/// How much of the preset color shows through an unselected cell.
const GROUP_TINT: f32 = 0.12;

/// One breath of the selected cell's glow.
const PULSE_SECONDS: f64 = 1.2;

fn suffix(path: &str, depth: usize) -> String {
    let parts: Vec<_> = path.split(['/', '\\']).collect();
    parts[parts.len().saturating_sub(depth)..].join("/")
}

#[cfg(test)]
fn labels(candidates: &[Candidate], results: &[usize]) -> Vec<String> {
    prefixed_labels(candidates, results)
        .into_iter()
        .map(|(_, label)| label)
        .collect()
}

/// Each label with the byte length of its plugin prefix (0 when plugins are not shown).
fn prefixed_labels(candidates: &[Candidate], results: &[usize]) -> Vec<(usize, String)> {
    let mixed = mixed_plugins(candidates, results);
    let mut groups: HashMap<_, Vec<usize>> = HashMap::new();
    for (row, &index) in results.iter().enumerate() {
        let candidate = &candidates[index];
        groups
            .entry((
                &candidate.format,
                &candidate.plugin_id,
                suffix(&candidate.display, 1),
            ))
            .or_default()
            .push(row);
    }
    let mut names: Vec<_> = results
        .iter()
        .map(|&index| suffix(&candidates[index].display, 1))
        .collect();
    for group in groups.values().filter(|group| group.len() > 1) {
        let max_depth = group
            .iter()
            .map(|&row| candidates[results[row]].display.split(['/', '\\']).count())
            .max()
            .unwrap();
        let mut unresolved = group.clone();
        for depth in 2..=max_depth {
            let mut counts = HashMap::new();
            for &row in group {
                *counts
                    .entry(suffix(&candidates[results[row]].display, depth))
                    .or_insert(0) += 1;
            }
            unresolved.retain(|&row| {
                names[row] = suffix(&candidates[results[row]].display, depth);
                counts[&names[row]] > 1
            });
            if unresolved.is_empty() {
                break;
            }
        }
        for row in unresolved {
            names[row] = format!(
                "{} [{}]",
                candidates[results[row]].display,
                candidates[results[row]].path.display()
            );
        }
    }
    if !mixed {
        return names.into_iter().map(|name| (0, name)).collect();
    }
    let mut plugins_by_name: HashMap<_, HashSet<_>> = HashMap::new();
    for &index in results {
        let candidate = &candidates[index];
        plugins_by_name
            .entry(&candidate.plugin_name)
            .or_default()
            .insert((&candidate.format, &candidate.plugin_id));
    }
    names
        .into_iter()
        .zip(results)
        .map(|(name, &index)| {
            let candidate = &candidates[index];
            let same_name_other_id = plugins_by_name[&candidate.plugin_name].len() > 1;
            let prefix = if same_name_other_id {
                format!(
                    "{} ({}/{}) — ",
                    candidate.plugin_name, candidate.format, candidate.plugin_id
                )
            } else {
                format!("{} — ", candidate.plugin_name)
            };
            (prefix.len(), prefix + &name)
        })
        .collect()
}

fn mixed_plugins(candidates: &[Candidate], results: &[usize]) -> bool {
    results.first().is_some_and(|&first| {
        results.iter().any(|&index| {
            candidates[index].format != candidates[first].format
                || candidates[index].plugin_id != candidates[first].plugin_id
        })
    })
}

fn details(snapshot: &Snapshot, index: usize) -> String {
    let candidate = &snapshot.candidates[index];
    let mut text = format!(
        "{} — {}\n{}",
        candidate.plugin_name,
        candidate.display,
        candidate.path.display()
    );
    if let Some(category) = candidate
        .entry
        .selector_category()
        .filter(|category| !category.trim().is_empty())
    {
        text.push_str(&format!("\nCategory: {category}"));
    }
    if candidate.entry.merged_count() > 1 {
        text.push_str(&format!("\n{} merged", candidate.entry.merged_count()));
    }
    for (role, presets) in snapshot.memberships(index) {
        text.push_str(&format!("\nRole: {role}"));
        if !presets.is_empty() {
            text.push_str(&format!("\nPreset: {}", presets.join(", ")));
        }
    }
    text
}

#[cfg(test)]
mod tests;
