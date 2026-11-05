//! Drum intent feeds the existing latest-only preparation and patch transaction.
use crate::{
    drum_browser::{identity, notes, same, Audition},
    App, SequencePattern,
};

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

    pub(crate) fn drum_main_controls(&mut self, ui: &mut eframe::egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
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

    pub(crate) fn can_open_drums(&self) -> bool {
        self.selection.is_none() && self.resident.is_none() && !self.mml_input.open
    }

    pub(crate) fn open_drum_browser(&mut self) {
        if !self.can_open_drums() {
            return;
        }
        self.patch_browser.close();
        self.effect_browser.close();
        self.patch_browser.requests.invalidate();
        self.auto_start_pending = false;
        self.stop_drum_audio();
        self.drum_browser.open = true;
        self.drum_browser.opening_audition = true;
        self.sync_drum_catalog();
        self.start_opening_drum_audition();
    }

    pub(crate) fn start_opening_drum_audition(&mut self) {
        if !self.drum_browser.opening_audition || self.drum_browser.selected().is_none() {
            return;
        }
        self.drum_browser.opening_audition = false;
        self.request_drum_audition(Audition::All, false);
    }

    pub(crate) fn sync_drum_catalog(&mut self) {
        if self.drum_browser.generation == Some(self.patch_catalog.generation) {
            return;
        }
        self.drum_browser.generation = Some(self.patch_catalog.generation);
        self.drum_browser.kits = self.patch_catalog.drum_candidates().cloned().collect();
        self.drum_browser.filter();
    }

    pub(crate) fn save_drum_cursor(&mut self) {
        let result = self
            .config_path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| self.drum_browser.saved.save(path));
        if let Err(error) = result {
            self.drum_browser.error = Some(error);
        }
    }

    pub(crate) fn stop_drum_audio(&mut self) {
        self.drum_browser.audition_started = false;
        self.auto_start_pending = false;
        self.sequence_pattern = SequencePattern::Off;
        for voice in self.instances.iter().filter_map(|i| i.voice.as_ref()) {
            voice.set_sequence(SequencePattern::Off);
            voice.preview(None);
            voice.drum_preview(None);
        }
    }

    pub(crate) fn stop_drum_preview(&mut self) {
        self.drum_browser.opening_audition = false;
        self.drum_browser.loading_since = None;
        self.drum_browser.awaiting = None;
        self.patch_browser.requests.invalidate();
        self.stop_drum_audio();
    }

    pub(crate) fn close_drum_browser(&mut self) {
        self.stop_drum_preview();
        self.drum_browser.open = false;
        self.drum_browser.editing = None;
        self.drum_browser.composing = false;
        self.save_drum_cursor();
        self.save_session();
    }

    pub(crate) fn select_drum_kit(&mut self, index: usize) {
        if self.drum_browser.selected == Some(index) {
            return;
        }
        self.stop_drum_preview();
        self.drum_browser.selected = Some(index);
        self.drum_browser.scroll = true;
        if let Some(candidate) = self.drum_browser.selected() {
            self.drum_browser.saved.selected = Some(identity(candidate));
        }
        self.save_drum_cursor();
        if self
            .drum_browser
            .selected()
            .is_some_and(|c| !c.measurement.is_heavy_offline_load())
        {
            self.request_drum_audition(Audition::All, false);
        }
    }

    pub(crate) fn request_drum_audition(&mut self, audition: Audition, explicit: bool) {
        let Some(candidate) = self.drum_browser.selected().cloned() else {
            return;
        };
        self.stop_drum_preview();
        let Some(available) = notes(&candidate).filter(|notes| !notes.is_empty()) else {
            self.status = "Drum preview unavailable: note assignments unknown or empty".into();
            return;
        };
        if matches!(audition, Audition::Note(note) if !available.contains(&note)) {
            return;
        }
        let applied = self
            .patch_browser
            .requests
            .applied
            .as_ref()
            .is_some_and(|c| same(&identity(c), &identity(&candidate)));
        if !explicit && matches!(audition, Audition::Note(_)) && !applied {
            return;
        }
        if applied && !self.actions_busy() {
            self.play_drum_audition(audition);
            return;
        }
        if !explicit && candidate.measurement.is_heavy_offline_load() {
            return;
        }
        self.patch_browser
            .requests
            .select(candidate.clone(), self.patch_catalog.generation);
        if explicit {
            // This screen's Space is the explicit load confirmation itself.
            self.patch_browser.requests.space();
            self.patch_browser.requests.enter();
        }
        self.drum_browser.loading_since = Some(std::time::Instant::now());
        self.drum_browser.awaiting = Some((identity(&candidate), audition));
    }

    pub(crate) fn drum_patch_finished(&mut self, success: bool) {
        if success
            && self
                .patch_browser
                .requests
                .applied
                .as_ref()
                .is_some_and(|c| c.measurement.drum_kit)
        {
            self.drum_browser.saved.applied =
                self.patch_browser.requests.applied.as_ref().map(identity);
            self.save_drum_cursor();
        }
        // A previous transaction may finish while a new candidate is queued.
        if success
            && self.drum_browser.awaiting.as_ref().is_some_and(|(key, _)| {
                !self
                    .patch_browser
                    .requests
                    .applied
                    .as_ref()
                    .is_some_and(|c| same(&identity(c), key))
            })
        {
            return;
        }
        if !success && self.patch_browser.requests.desired.is_some() {
            return;
        }
        let Some((key, audition)) = self.drum_browser.awaiting.take() else {
            return;
        };
        self.drum_browser.loading_since = None;
        let current = self.drum_browser.open
            && self
                .drum_browser
                .selected()
                .is_some_and(|c| same(&identity(c), &key));
        if !success || !current {
            return;
        }
        if self
            .patch_browser
            .requests
            .applied
            .as_ref()
            .is_some_and(|c| same(&identity(c), &key))
        {
            self.play_drum_audition(audition);
        }
    }

    pub(crate) fn toggle_drum_velocity(&mut self) {
        self.drum_browser.compare_velocity = !self.drum_browser.compare_velocity;
        if self.drum_browser.compare_velocity {
            if let Some((candidate, note)) = self
                .drum_browser
                .selected()
                .cloned()
                .zip(self.current_drum_note())
            {
                self.drum_browser.saved.set_note(&candidate, note);
                self.drum_browser.scroll = true;
                self.save_drum_cursor();
            }
        }
        // An in-flight load will use the new mode on completion. Otherwise audition
        // the current note if its kit is already loaded, without starting a load.
        if self.drum_browser.awaiting.is_none()
            && (self.drum_browser.audition_started || self.drum_browser.compare_velocity)
        {
            let audition = self
                .drum_browser
                .selected()
                .and_then(|c| self.drum_browser.saved.note(c))
                .map(Audition::Note);
            if let Some(audition) = audition {
                self.request_drum_audition(audition, false);
            }
        }
    }

    fn play_drum_audition(&mut self, audition: Audition) {
        let Some(candidate) = self.drum_browser.selected() else {
            return;
        };
        let notes = match audition {
            Audition::All if self.drum_browser.compare_velocity => self
                .drum_browser
                .saved
                .note(candidate)
                .into_iter()
                .collect(),
            Audition::All => notes(candidate).unwrap_or_default(),
            Audition::Note(note) => vec![note],
        };
        let program = if self.drum_browser.compare_velocity {
            let Some(&note) = notes.first() else {
                return;
            };
            crate::drum_preview::comparison(note)
        } else {
            crate::drum_preview::program(&notes, self.sample_rate())
        };
        self.stop_drum_audio();
        for voice in self.instances.iter().filter_map(|i| i.voice.as_ref()) {
            voice.drum_preview(Some(program));
        }
        self.drum_browser.audition_started = true;
    }
}
