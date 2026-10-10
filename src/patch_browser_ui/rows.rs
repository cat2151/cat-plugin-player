//! Cached row names and visible-row-only painting; native identity is never shortened.
use super::*;
use crate::random_patch_catalog::Candidate;
use row_colors::RowTint;
use std::collections::{HashMap, HashSet};

impl Browser {
    pub(crate) fn refresh_row_labels(&mut self) {
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
        let columns = ((width + gap) / (240.0 + gap)).floor().max(1.0) as usize;
        let resized = self.list_columns != columns;
        self.list_columns = columns;
        let cell_width = (width - gap * (columns - 1) as f32) / columns as f32;
        let total_rows = self.results.len().div_ceil(columns);
        let mut select = None;
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
                        let response = ui
                            .push_id(index, |ui| {
                                ui.add_sized(
                                    [cell_width, row_height],
                                    egui::Button::new("").selected(self.selected == Some(index)),
                                )
                            })
                            .inner;
                        // Fixed-height virtual rows must not expand when a path wraps.
                        let painter = ui
                            .painter()
                            .with_clip_rect(response.rect.shrink2(egui::vec2(6.0, 1.0)));
                        let tint = self.row_tints[cell];
                        let dark = ui.visuals().dark_mode;
                        let plain = ui.visuals().text_color();
                        let paint = |slot: Option<usize>| {
                            slot.map_or(plain, |slot| row_colors::color(slot, dark))
                        };
                        let name_color = if candidate.measurement.is_heavy_offline_load() {
                            PINK
                        } else {
                            paint(tint.name)
                        };
                        let font = egui::TextStyle::Body.resolve(ui.style());
                        let mut job = egui::text::LayoutJob::default();
                        let (prefix, name) = text.split_at(tint.prefix_len);
                        for (part, color) in [(prefix, paint(tint.plugin)), (name, name_color)] {
                            job.append(part, 0.0, egui::TextFormat::simple(font.clone(), color));
                        }
                        let galley = ui.fonts(|fonts| fonts.layout_job(job));
                        let top = response.rect.center().y - galley.size().y / 2.0;
                        painter.galley(egui::pos2(response.rect.left() + 6.0, top), galley, plain);
                        let response = response.on_hover_text(details(candidate));
                        if response.clicked() {
                            select = Some(index);
                        }
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
    }
}

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

fn details(candidate: &Candidate) -> String {
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
    text
}

#[cfg(test)]
mod tests;
