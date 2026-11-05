//! Prominent transport feedback and content-aware pane sizing.
use super::*;

pub(super) fn loading_color(elapsed: f32) -> egui::Color32 {
    if elapsed < 1.0 {
        return egui::Color32::from_rgb(255, 215, 100);
    }
    let phase = elapsed - 1.0;
    let brightness = 0.85 + 0.15 * (phase * std::f32::consts::TAU / 3.0).cos();
    egui::ecolor::Hsva::new((phase / 10.0).fract(), 0.55, brightness, 1.0).into()
}

pub(super) fn pitch(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NAMES[note as usize % 12], i16::from(note) / 12 - 1)
}

impl App {
    pub(crate) fn current_drum_note(&self) -> Option<u8> {
        self.drum_browser
            .audition_started
            .then(|| {
                self.instances
                    .iter()
                    .filter_map(|i| i.voice.as_ref())
                    .find_map(|v| v.drum_note())
            })
            .flatten()
    }

    pub(super) fn drum_left_width(&self, ui: &egui::Ui, total: f32) -> f32 {
        super::layout::pane_widths(&self.drum_browser, ui)[0].min(total * 0.8)
    }

    pub(super) fn drum_feedback(&self, ui: &mut egui::Ui) {
        let candidate = self.drum_browser.selected();
        let (text, color) = if let Some((key, _)) = &self.drum_browser.awaiting {
            let elapsed = self
                .drum_browser
                .loading_since
                .map_or(0.0, |t| t.elapsed().as_secs_f32());
            (format!("Loading  {}", key.display), loading_color(elapsed))
        } else if let Some(note) = self.current_drum_note() {
            let velocity = self
                .instances
                .iter()
                .filter_map(|i| i.voice.as_ref())
                .find(|v| v.drum_note() == Some(note))
                .map_or(0, |v| v.current_values().0);
            preview_row(
                ui,
                note,
                velocity,
                candidate.map_or("", |c| crate::drum_browser::note_name(c, note)),
            );
            return;
        } else if candidate.is_some_and(|c| c.measurement.is_heavy_offline_load())
            && !self
                .patch_browser
                .requests
                .applied
                .as_ref()
                .zip(candidate)
                .is_some_and(|(a, b)| same(&identity(a), &identity(b)))
        {
            (
                "このkitはSpaceでロードします（10秒以上かかる可能性があります）".into(),
                egui::Color32::from_rgb(255, 135, 190),
            )
        } else if self.drum_browser.audition_started {
            ("Finished".into(), ui.visuals().text_color())
        } else {
            ("Stopped".into(), ui.visuals().text_color())
        };
        let response = ui.add(
            egui::Label::new(egui::RichText::new(&text).size(20.0).strong().color(color)).wrap(),
        );
        if text == "Finished" || text == "Stopped" {
            response
                .on_hover_ui(|ui| crate::command_palette::shortcut_hint(ui, "Preview", "Space"));
        }
    }
}

fn preview_row(ui: &mut egui::Ui, note: u8, velocity: u8, name: &str) {
    let font = egui::FontId::proportional(20.0);
    let color = egui::Color32::LIGHT_GREEN;
    let measure = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.into(), font.clone(), color)
            .size()
            .x
    };
    let widths = [
        measure("Preview"),
        (0..128).map(|n| measure(&pitch(n))).fold(0.0_f32, f32::max),
        measure("Note 127"),
        measure("Vel 127"),
    ];
    let texts = [
        "Preview".to_owned(),
        pitch(note),
        format!("Note {note}"),
        format!("Vel {velocity}"),
        name.to_owned(),
    ];
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::hover());
    let mut x = rect.left();
    for (index, text) in texts.iter().enumerate() {
        let width = widths
            .get(index)
            .copied()
            .unwrap_or((rect.right() - x).max(0.0));
        let cell =
            egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(width, rect.height()));
        ui.painter()
            .with_clip_rect(cell.intersect(rect).intersect(ui.clip_rect()))
            .text(
                egui::pos2(x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                font.clone(),
                color,
            );
        x += width + 16.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn text_positions(note: u8, velocity: u8, name: &str) -> Vec<(String, f32)> {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| preview_row(ui, note, velocity, name));
        });
        output
            .shapes
            .into_iter()
            .filter_map(|s| match s.shape {
                egui::Shape::Text(t) => Some((t.galley.job.text.clone(), t.pos.x)),
                _ => None,
            })
            .collect()
    }
    #[test]
    fn preview_columns_do_not_shift_with_pitch_velocity_or_name_length() {
        let a = text_positions(69, 40, "Snare");
        let b = text_positions(70, 127, "Long percussion name");
        assert_eq!(a.len(), 5);
        assert_eq!(b.len(), 5);
        assert_eq!(a[1].0, "A4");
        assert_eq!(b[1].0, "A#4");
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(a.1, b.1);
        }
    }
}
