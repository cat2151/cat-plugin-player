//! Fixed column positions and separate metadata colors within each selectable row.
use super::*;
use egui::{Align2, Color32, Response, Sense};

pub(super) const GAP: f32 = 8.0;
const PLUGIN: Color32 = Color32::from_rgb(120, 195, 245);
const COUNT: Color32 = Color32::from_rgb(125, 210, 190);
const SIZE: Color32 = Color32::from_rgb(230, 195, 110);
const HEAVY: Color32 = Color32::from_rgb(255, 135, 190);

pub(super) fn kit_name(display: &str) -> &str {
    let name = display.rsplit(['/', '\\']).next().unwrap_or(display);
    if name.eq_ignore_ascii_case("main.sfz") {
        display
            .rsplit_once(['/', '\\'])
            .and_then(|(parent, _)| parent.rfind(['/', '\\']).map(|at| &display[at + 1..]))
            .unwrap_or(display)
    } else {
        name
    }
}

pub(super) fn kit_parent(candidate: &crate::patch_catalog::Candidate) -> String {
    candidate
        .display
        .rsplit_once(['/', '\\'])
        .map(|(parent, _)| parent.to_owned())
        .unwrap_or_else(|| {
            candidate
                .path
                .parent()
                .map_or_else(String::new, |parent| parent.display().to_string())
        })
}

pub(super) fn sample_size(candidate: &crate::patch_catalog::Candidate) -> String {
    candidate
        .measurement
        .sfz_sample_bytes
        .map_or("unknown".into(), |bytes| {
            if bytes >= 1_000_000_000 {
                format!("{:.1} GB", bytes as f64 / 1e9)
            } else {
                format!("{:.1} MB", bytes as f64 / 1e6)
            }
        })
}

fn row(
    ui: &mut egui::Ui,
    widths: &[f32],
    cells: &[(&str, Color32)],
    selected: bool,
    header: bool,
    focused: bool,
) -> Response {
    let height = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(
            widths.iter().sum::<f32>() + GAP * (widths.len() - 1) as f32,
            height,
        ),
        if header {
            Sense::hover()
        } else {
            Sense::click()
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            cells
                .iter()
                .map(|(text, _)| *text)
                .collect::<Vec<_>>()
                .join(" · "),
        )
    });
    if selected && focused {
        super::focus::cursor(ui, rect);
    } else if selected || response.hovered() && !header {
        let fill = if selected {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().widgets.hovered.weak_bg_fill
        };
        ui.painter().rect_filled(rect, 2.0, fill);
    }
    let mut x = rect.min.x + 3.0;
    for (&width, &(text, color)) in widths.iter().zip(cells) {
        let cell =
            egui::Rect::from_min_size(egui::pos2(x, rect.min.y), egui::vec2(width - 4.0, height));
        ui.painter()
            .with_clip_rect(cell.intersect(ui.clip_rect()))
            .text(
                egui::pos2(x, rect.center().y),
                Align2::LEFT_CENTER,
                text,
                egui::TextStyle::Body.resolve(ui.style()),
                color,
            );
        x += width + GAP;
    }
    response
}

pub(super) fn note_widths(ui: &egui::Ui, candidate: &crate::patch_catalog::Candidate) -> [f32; 5] {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mut name = 50.0_f32;
    let mut one_shot = 68.0_f32;
    for note in notes(candidate).unwrap_or_default() {
        let measure = |text: &str| {
            ui.painter()
                .layout_no_wrap(text.into(), font.clone(), ui.visuals().text_color())
                .size()
                .x
                + 8.0
        };
        name = name.max(measure(crate::drum_browser::note_name(candidate, note)));
        one_shot = one_shot.max(measure(crate::drum_browser::one_shot(candidate, note)));
    }
    [14.0, 30.0, 40.0, name, one_shot]
}

impl App {
    pub(super) fn drum_kit_rows(&mut self, ui: &mut egui::Ui) {
        let mut clicked = None;
        let widths = self.drum_kit_widths(ui);
        egui::ScrollArea::both()
            .id_salt("drum_kits")
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                let weak = ui.visuals().weak_text_color();
                row(
                    ui,
                    &widths,
                    &[
                        ("Plugin", weak),
                        ("Kit / SFZ", weak),
                        ("Path", weak),
                        ("Notes", weak),
                        ("Samples", weak),
                        ("Status", weak),
                    ],
                    false,
                    true,
                    false,
                );
                for &index in &self.drum_browser.results {
                    let candidate = &self.drum_browser.kits[index];
                    let installed = self.plugins.iter().any(|p| {
                        p.id == candidate.plugin_id
                            && p.format == candidate.format
                            && p.kind == crate::plugin_list::PluginKind::Instrument
                    });
                    let requests = &self.patch_browser.requests;
                    let key = identity(candidate);
                    let loading = requests
                        .preparing
                        .as_ref()
                        .or(requests.applying.as_ref())
                        .is_some_and(|t| same(&identity(&t.candidate), &key));
                    let applied = requests
                        .applied
                        .as_ref()
                        .is_some_and(|c| same(&identity(c), &key));
                    let heavy = candidate.measurement.is_heavy_offline_load();
                    let (state, state_color) = if !installed {
                        ("unavailable", HEAVY)
                    } else if loading {
                        ("loading...", SIZE)
                    } else if applied {
                        ("loaded", Color32::LIGHT_GREEN)
                    } else if heavy {
                        ("Space: load", HEAVY)
                    } else {
                        ("ready", weak)
                    };
                    let count = notes(candidate).map_or("?".into(), |n| n.len().to_string());
                    let size = sample_size(candidate);
                    let selected = self.drum_browser.selected == Some(index);
                    let parent = kit_parent(candidate);
                    let response = row(
                        ui,
                        &widths,
                        &[
                            (&candidate.plugin_name, PLUGIN),
                            (kit_name(&candidate.display), ui.visuals().text_color()),
                            (&parent, weak),
                            (&count, COUNT),
                            (&size, if heavy { HEAVY } else { SIZE }),
                            (state, state_color),
                        ],
                        selected,
                        false,
                        self.drum_browser.saved.focus == Focus::Kits,
                    );
                    if selected && self.drum_browser.scroll {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if response.clicked() {
                        clicked = Some(index);
                    }
                }
            });
        if let Some(index) = clicked {
            self.drum_browser.saved.focus = Focus::Kits;
            self.select_drum_kit(index);
            self.save_drum_cursor();
        }
    }

    pub(super) fn drum_note_rows(&mut self, ui: &mut egui::Ui) {
        let Some(candidate) = self.drum_browser.selected().cloned() else {
            ui.label("No kit selected");
            return;
        };
        let Some(available) = notes(&candidate) else {
            ui.label("Note assignments unknown");
            return;
        };
        if available.is_empty() {
            ui.label("No assigned notes");
            return;
        }
        let selected = self.drum_browser.saved.note(&candidate);
        let widths = note_widths(ui, &candidate);
        let mut clicked = None;
        egui::ScrollArea::both()
            .id_salt("drum_notes")
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                let weak = ui.visuals().weak_text_color();
                row(
                    ui,
                    &widths,
                    &[
                        ("", weak),
                        ("No.", weak),
                        ("Pitch", weak),
                        ("Name", weak),
                        ("One-shot", weak),
                    ],
                    false,
                    true,
                    false,
                );
                for note in available {
                    let marker = if self.current_drum_note() == Some(note) {
                        "▶"
                    } else {
                        ""
                    };
                    let number = note.to_string();
                    let pitch = super::feedback::pitch(note);
                    let name = crate::drum_browser::note_name(&candidate, note);
                    let shot = crate::drum_browser::one_shot(&candidate, note);
                    let response = row(
                        ui,
                        &widths,
                        &[
                            (marker, Color32::LIGHT_GREEN),
                            (&number, COUNT),
                            (&pitch, PLUGIN),
                            (name, ui.visuals().text_color()),
                            (shot, SIZE),
                        ],
                        selected == Some(note),
                        false,
                        self.drum_browser.saved.focus == Focus::Notes,
                    );
                    if selected == Some(note) && self.drum_browser.scroll {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if response.clicked() {
                        clicked = Some(note);
                    }
                }
            });
        if let Some(note) = clicked {
            self.drum_browser.saved.focus = Focus::Notes;
            self.drum_browser.saved.set_note(&candidate, note);
            self.request_drum_audition(Audition::Note(note), false);
            self.save_drum_cursor();
        }
    }
}
