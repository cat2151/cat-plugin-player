//! Main-window kit audition omits the pitched copies at either end of a kit.
use crate::{
    drum_browser::{identity, notes, same},
    patch_catalog::Candidate,
    App,
};

pub(crate) fn main_notes(candidate: &Candidate) -> Vec<u8> {
    let mut notes = notes(candidate).unwrap_or_default();
    let name = |note| {
        candidate
            .measurement
            .drum_kit_note_names
            .iter()
            .find(|(n, _)| *n == note)
            .map(|(_, name)| name.as_str())
            .filter(|name| !name.trim().is_empty())
    };
    let same_name = |a, b| name(a).zip(name(b)).is_some_and(|(a, b)| a == b);
    let mut start = 0;
    while start + 1 < notes.len()
        && notes[start] + 1 == notes[start + 1]
        && same_name(notes[start], notes[start + 1])
    {
        start += 1;
    }
    notes.drain(..start);
    while notes.len() > 1 {
        let end = notes.len() - 1;
        if notes[end - 1] + 1 != notes[end] || !same_name(notes[end - 1], notes[end]) {
            break;
        }
        notes.pop();
    }
    notes
}

impl App {
    pub(crate) fn drum_kit_active(&self) -> bool {
        if self.selection.is_some() || self.resident.is_some() {
            return false;
        }
        if let Some(candidate) = &self.patch_browser.requests.applied {
            return candidate.measurement.drum_kit;
        }
        self.drum_browser
            .saved
            .applied
            .as_ref()
            .zip(self.patch_browser.restore_playing.as_ref())
            .is_some_and(|(kit, playing)| same(kit, playing))
    }

    pub(crate) fn main_drum_candidate(&self) -> Option<&Candidate> {
        self.patch_browser.requests.applied.as_ref().or_else(|| {
            let playing = self.patch_browser.restore_playing.as_ref()?;
            self.patch_catalog
                .drum_candidates()
                .find(|c| same(&identity(c), playing))
        })
    }

    pub(crate) fn play_main_drum_preview(&mut self) {
        if !self.drum_kit_active() {
            return;
        }
        let Some(candidate) = self.main_drum_candidate() else {
            return;
        };
        let notes = main_notes(candidate);
        self.stop_drum_audio();
        if notes.is_empty() {
            return;
        }
        let program = crate::drum_preview::program(&notes, self.sample_rate());
        for voice in self.instances.iter().filter_map(|i| i.voice.as_ref()) {
            voice.drum_preview(Some(program));
        }
        self.drum_browser.main_audition = true;
    }

    pub(crate) fn drum_main_controls(
        &mut self,
        ui: &mut eframe::egui::Ui,
        transport: &crate::playback_transport::Transport,
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                if transport.button(ui) {
                    self.toggle_main_playback();
                }
                if ui
                    .add_enabled(
                        self.can_open_drums() && !self.drum_browser.open,
                        eframe::egui::Button::new("preview drum kit"),
                    )
                    .on_hover_ui(|ui| {
                        crate::command_palette::shortcut_hint(ui, "preview drum kit", "G")
                    })
                    .clicked()
                {
                    self.open_drum_browser();
                }
            });
            if let Some(candidate) = &self.patch_browser.requests.applied {
                ui.add(eframe::egui::Label::new(&candidate.display).truncate())
                    .on_hover_text(&candidate.display);
            }
        });
    }
}

#[cfg(test)]
mod tests;
