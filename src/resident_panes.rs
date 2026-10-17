//! Pane labels travel independently of sound selection and preparation.
use crate::{
    resident_ipc::{Open, ToEditor},
    App,
};

impl App {
    pub(crate) fn restore_resident_panes(&mut self, open: Open, panes: [String; 2], query: String) {
        match open {
            Open::Patch { .. } | Open::Layer { .. } => {
                // Retain the built catalog and preparation worker across targets.
                let restored =
                    crate::patch_browser::Browser::restored(crate::status::BrowserFilter {
                        role: panes[0].clone(),
                        preset: panes[1].clone(),
                        query: query.clone(),
                        ..Default::default()
                    });
                let browser = &mut self.patch_browser;
                browser.requests.invalidate();
                browser.requests.applied = None;
                browser.restore_playing = None;
                browser.restore_selected = None;
                browser.selected = None;
                browser.role = restored.role;
                browser.preset = 0;
                browser.restore_preset = restored.restore_preset;
                browser.query = query;
                browser.initial_filter_fallback = None;
                browser.resolve_restored_preset();
                browser.filter(false);
            }
            Open::Effect { .. } | Open::MasterEffect { .. } => {
                self.effect_browser.restore_panes(panes, query);
            }
        }
    }

    pub(crate) fn report_resident_panes(&mut self) {
        let Some(resident) = &self.resident else {
            return;
        };
        let (Some(open), Some((context, _, _))) = (resident.shown, &resident.panes) else {
            return;
        };
        let context = context.clone();
        if matches!(open, Open::Patch { .. } | Open::Layer { .. }) {
            if let Some(playing) = self.patch_browser.saved_filter().playing {
                self.resident
                    .as_mut()
                    .unwrap()
                    .playing_cache
                    .insert(context.clone(), playing);
            }
        }
        let resident = self.resident.as_ref().unwrap();
        let (_, previous, previous_query) = resident.panes.as_ref().unwrap();
        let (panes, query) = match open {
            Open::Patch { .. } | Open::Layer { .. } => {
                let saved = self.patch_browser.saved_filter();
                ([saved.role, saved.preset], saved.query)
            }
            Open::Effect { .. } | Open::MasterEffect { .. } => {
                let saved = self.effect_browser.saved_filter(&[]);
                ([saved.category, saved.kind], saved.query)
            }
        };
        if &panes != previous || &query != previous_query {
            let resident = self.resident.as_mut().unwrap();
            let (context, previous, previous_query) = resident.panes.as_mut().unwrap();
            if let Err(error) = resident.sender.send(&ToEditor::Panes {
                context: context.clone(),
                panes: panes.clone(),
                query: query.clone(),
            }) {
                self.status = format!("Cannot save browser panes: {error}");
            } else {
                resident
                    .pane_cache
                    .insert(context.clone(), (panes.clone(), query.clone()));
                *previous = panes;
                *previous_query = query;
            }
        }
        self.report_resident_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch_browser_catalog::{tests::candidate, Snapshot};

    #[test]
    fn restores_panes_after_catalog_load_and_never_requests_a_sound() {
        let _library = crate::native_library::load().unwrap();
        let mut app = App::prepare_mode(true);
        let patch = Open::Patch { track: 1 };
        app.restore_resident_panes(patch, ["Bass track".into(), "ALL".into()], String::new());
        assert_eq!(app.patch_browser.saved_filter().role, "Bass track");
        app.patch_browser.snapshot = Some(
            Snapshot::build(vec![candidate("a", "Alpha", "Bass", Some("Bass"))], &[]).unwrap(),
        );
        app.patch_browser.resolve_restored_preset();
        app.patch_browser.filter(false);
        assert_eq!(app.patch_browser.saved_filter().role, "Bass track");
        assert_eq!(app.patch_browser.results.len(), 1);
        assert!(app.patch_browser.requests.desired.is_none());
        app.restore_resident_panes(patch, ["deleted".into(), "deleted".into()], String::new());
        assert_eq!(app.patch_browser.role, 0);
        assert_eq!(app.patch_browser.preset, 0);

        let candidates = vec![crate::effect_browser::tests::candidate(
            "a", "Hall", "Space", "Reverb",
        )];
        app.restore_resident_panes(
            Open::Effect {
                track: 1,
                slot: None,
            },
            ["Space".into(), "Reverb".into()],
            "hall -plate".into(),
        );
        assert_eq!(app.effect_browser.saved_filter(&[]).category, "Space");
        app.effect_browser.refresh(1, candidates.clone());
        assert_eq!(app.effect_browser.saved_filter(&[]).kind, "Reverb");
        assert_eq!(app.effect_browser.saved_filter(&[]).query, "hall -plate");
        assert!(app.effect_browser.desired.is_none());
        app.restore_resident_panes(
            Open::MasterEffect { slot: None },
            Default::default(),
            String::new(),
        );
        app.effect_browser.refresh(1, candidates.clone());
        assert_eq!(app.effect_browser.category, 0);
        assert_eq!(app.effect_browser.kind, 0);
        app.restore_resident_panes(
            Open::Effect {
                track: 1,
                slot: None,
            },
            ["deleted".into(), "deleted".into()],
            String::new(),
        );
        app.effect_browser.refresh(1, candidates);
        assert_eq!(app.effect_browser.category, 0);
        assert_eq!(app.effect_browser.kind, 0);
        assert!(app.effect_browser.desired.is_none());
    }
}
