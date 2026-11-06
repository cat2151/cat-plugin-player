use super::*;

impl App {
    pub(super) fn drum_keyboard(&mut self, ctx: &egui::Context) {
        let take = |key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        if let Some(original) = self.drum_browser.editing.clone() {
            let events = ctx.input(|i| i.events.clone());
            let ime = self.drum_browser.composing
                || events.iter().any(|e| matches!(e, egui::Event::Ime(_)));
            for event in &events {
                match event {
                    egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
                        self.drum_browser.composing = !text.is_empty()
                    }
                    egui::Event::Ime(egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled) => {
                        self.drum_browser.composing = false
                    }
                    _ => {}
                }
            }
            let escape = !ime && take(Key::Escape);
            if escape || (!ime && take(Key::Enter)) {
                // Escape cancels only the query editor, never the browser.
                if escape {
                    self.drum_browser.saved.query = original;
                }
                self.drum_browser.editing = None;
                ctx.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id);
                    }
                });
                self.filter_drums();
            }
            return;
        }
        if self.drum_browser.composing
            || ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Ime(_))))
        {
            return;
        }
        if take(Key::Escape) {
            self.close_drum_browser();
            return;
        }
        if take(Key::Enter) {
            if self.resident_kit() {
                self.confirm_resident_kit();
            } else {
                self.close_drum_browser();
            }
            return;
        }
        if take(Key::P) {
            self.stop_drum_preview();
            return;
        }
        if take(Key::V) {
            self.toggle_drum_velocity();
            return;
        }
        if self.drum_browser.saved.focus == Focus::Kits && take(Key::Slash) {
            self.drum_browser.editing = Some(self.drum_browser.saved.query.clone());
            return;
        }
        if take(Key::H) || take(Key::ArrowLeft) {
            self.drum_browser.saved.focus = Focus::Kits;
            self.save_drum_cursor();
        }
        if take(Key::L) || take(Key::ArrowRight) {
            self.drum_browser.saved.focus = Focus::Notes;
            self.save_drum_cursor();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Space)) {
            if self.drum_browser.saved.focus == Focus::Notes {
                self.request_drum_audition(Audition::All, true);
            }
            return;
        }
        if take(Key::Space) {
            let audition = if self.drum_browser.saved.focus == Focus::Notes {
                self.drum_browser
                    .selected()
                    .and_then(|c| self.drum_browser.saved.note(c))
                    .map(Audition::Note)
            } else {
                Some(Audition::All)
            };
            if let Some(audition) = audition {
                self.request_drum_audition(audition, true);
            }
            return;
        }
        let delta = if take(Key::J) || take(Key::ArrowDown) {
            1
        } else if take(Key::K) || take(Key::ArrowUp) {
            -1
        } else if take(Key::PageDown) {
            10
        } else if take(Key::PageUp) {
            -10
        } else if take(Key::Home) {
            isize::MIN
        } else if take(Key::End) {
            isize::MAX
        } else {
            return;
        };
        self.move_drum_cursor(delta);
    }

    pub(super) fn move_drum_cursor(&mut self, delta: isize) {
        if self.drum_browser.saved.focus == Focus::Kits {
            let rows = &self.drum_browser.results;
            if rows.is_empty() {
                return;
            }
            let at = rows
                .iter()
                .position(|&i| Some(i) == self.drum_browser.selected)
                .unwrap_or(0);
            self.select_drum_kit(rows[advance(at, rows.len(), delta)]);
        } else if let Some(candidate) = self.drum_browser.selected().cloned() {
            let available = notes(&candidate).unwrap_or_default();
            if available.is_empty() {
                return;
            }
            let old = self.drum_browser.saved.note(&candidate);
            let at = available.iter().position(|&n| Some(n) == old).unwrap_or(0);
            let note = available[advance(at, available.len(), delta)];
            if Some(note) != old {
                self.drum_browser.saved.set_note(&candidate, note);
                self.drum_browser.scroll = true;
                self.request_drum_audition(Audition::Note(note), false);
            }
        }
        self.save_drum_cursor();
    }
}

fn advance(at: usize, len: usize, delta: isize) -> usize {
    at.saturating_add_signed(delta).min(len.saturating_sub(1))
}
