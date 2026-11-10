//! A compact, non-interactive keyboard: phrase membership and delivered note gates.
use crate::{sequence_notes::contains, timed_sequence::Phrase, SequencePattern};
use eframe::egui::{self, Color32, Rect, Stroke, StrokeKind};
use std::sync::Arc;

#[derive(Clone)]
struct Plan {
    pattern: SequencePattern,
    phrase: Option<Arc<Phrase>>,
    used: [u64; 2],
    range: Option<(u8, u8)>,
}

impl Plan {
    fn new(pattern: SequencePattern, phrase: Option<&Arc<Phrase>>) -> Self {
        let mut used = [0; 2];
        let mut add = |note: u8| used[usize::from(note) / 64] |= 1 << (note % 64);
        match pattern {
            SequencePattern::Steps => crate::seq::NOTES.into_iter().for_each(&mut add),
            SequencePattern::GuitarArpeggio => {
                crate::seq::GUITAR_NOTES.into_iter().for_each(&mut add)
            }
            SequencePattern::Csus4CArpeggio | SequencePattern::Fmaj7G6Arpeggio => {
                let chords = if pattern == SequencePattern::Csus4CArpeggio {
                    crate::seq::CSUS4_C
                } else {
                    crate::seq::FMAJ7_G6
                };
                chords.into_iter().flatten().copied().for_each(&mut add);
            }
            SequencePattern::Custom => {
                if let Some(phrase) = phrase {
                    for (_, message) in &phrase.events {
                        if message[0] & 0xf0 == 0x90 && message[2] > 0 {
                            add(message[1]);
                        }
                    }
                }
            }
            SequencePattern::Off => {}
        }
        let mut notes = (0..128).filter(|&note| contains(used, note));
        let range = notes
            .next()
            .map(|first| (first, notes.next_back().unwrap_or(first)));
        Self {
            pattern,
            phrase: (pattern == SequencePattern::Custom)
                .then(|| phrase.cloned())
                .flatten(),
            used,
            range,
        }
    }

    fn matches(&self, pattern: SequencePattern, phrase: Option<&Arc<Phrase>>) -> bool {
        self.pattern == pattern
            && (pattern != SequencePattern::Custom
                || match (self.phrase.as_ref(), phrase) {
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    (None, None) => true,
                    _ => false,
                })
    }
}

fn black(note: u8) -> bool {
    matches!(note % 12, 1 | 3 | 6 | 8 | 10)
}

/// Diatonic coordinates preserve familiar white/black key geometry, even at edges.
fn span(note: u8) -> (f32, f32) {
    let white = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6][usize::from(note % 12)];
    let x = f32::from(note / 12) * 7.0 + white as f32;
    if black(note) {
        (x + 0.7, x + 1.3)
    } else {
        (x, x + 1.0)
    }
}

fn muted(base: Color32, accent: Color32) -> Color32 {
    let mix = |a, b| (f32::from(a) * 0.7 + f32::from(b) * 0.3) as u8;
    Color32::from_rgb(
        mix(base.r(), accent.r()),
        mix(base.g(), accent.g()),
        mix(base.b(), accent.b()),
    )
}

fn note_name(note: u8) -> String {
    let name = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ][usize::from(note % 12)];
    format!("{name}{}", i16::from(note) / 12 - 1)
}

pub(super) fn show(
    ui: &mut egui::Ui,
    pattern: SequencePattern,
    phrase: Option<&Arc<Phrase>>,
    sounding: [u64; 2],
) {
    // Recompute only on a pattern/phrase change, never on the audio startup path.
    let id = ui.make_persistent_id("sequence_keyboard_plan");
    let mut plan = ui.data(|data| data.get_temp::<Plan>(id));
    if !plan
        .as_ref()
        .is_some_and(|plan| plan.matches(pattern, phrase))
    {
        let next = Plan::new(pattern, phrase);
        ui.data_mut(|data| data.insert_temp(id, next.clone()));
        plan = Some(next);
    }
    let plan = plan.unwrap();
    let Some((first, last)) = plan.range else {
        return;
    };
    let left = span(first).0;
    let right = span(last).1;
    let width = ((right - left) * 7.0).clamp(120.0, 300.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let accent = ui.visuals().selection.stroke.color;
    let white = ui.visuals().widgets.inactive.bg_fill;
    let dark = ui.visuals().extreme_bg_color;
    painter.rect_filled(rect, 0.0, dark);
    // White keys first; black keys overlap their neighbors just as on a piano.
    for is_black in [false, true] {
        for note in first..=last {
            if black(note) != is_black {
                continue;
            }
            let (start, end) = span(note);
            let key = Rect::from_min_max(
                egui::pos2(
                    rect.left() + (start - left) / (right - left) * width,
                    rect.top(),
                ),
                egui::pos2(
                    rect.left() + (end - left) / (right - left) * width,
                    rect.top() + rect.height() * if is_black { 0.62 } else { 1.0 },
                ),
            );
            let active = contains(sounding, note);
            let base = if is_black { dark } else { white };
            let fill = if active {
                accent
            } else if contains(plan.used, note) {
                muted(base, accent)
            } else {
                base
            };
            painter.rect_filled(key.shrink(0.35), 0.5, fill);
            if active {
                painter.rect_stroke(
                    key.shrink(0.7),
                    0.5,
                    Stroke::new(1.0_f32, ui.visuals().text_color()),
                    StrokeKind::Inside,
                );
            }
        }
    }
    response.on_hover_text(format!("{}–{}", note_name(first), note_name(last)));
}

#[cfg(test)]
mod tests;
