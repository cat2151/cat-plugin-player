//! Sidecar filter exchange and a separate, read-only all-track preview.
use crate::{
    effect_browser::EffectBrowser,
    patch_browser::Browser,
    status::{BrowserFilter, EffectBrowserFilter},
    App,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

impl App {
    pub(crate) fn configure_selection_filter(
        &mut self,
        path: Option<PathBuf>,
    ) -> Result<(), String> {
        if let Some(path) = &path {
            match std::fs::read(path) {
                Ok(bytes) => {
                    let filter: BrowserFilter =
                        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    self.patch_browser = Browser {
                        open: self.patch_browser.open,
                        ..Browser::restored(filter)
                    };
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
            }
        }
        self.selection
            .as_mut()
            .ok_or("Selection mode required")?
            .filter_path = path;
        Ok(())
    }

    pub(crate) fn configure_effect_selection_filter(
        &mut self,
        path: Option<PathBuf>,
    ) -> Result<(), String> {
        if let Some(path) = &path {
            match std::fs::read(path) {
                Ok(bytes) => {
                    let filter: EffectBrowserFilter =
                        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    let open = self.effect_browser.open;
                    self.effect_browser = EffectBrowser::restored(filter);
                    self.effect_browser.open = open;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
            }
        }
        self.selection
            .as_mut()
            .ok_or("Selection mode required")?
            .effect_filter_path = path;
        Ok(())
    }

    pub(crate) fn save_selection_filter(&self) -> Result<(), String> {
        let selection = self.selection.as_ref();
        if let Some(path) = selection.and_then(|s| s.filter_path.as_ref()) {
            let mut filter = self.patch_browser.saved_filter();
            filter.playing = None;
            let bytes = serde_json::to_vec(&filter).map_err(|e| e.to_string())?;
            crate::state_store::atomic_write(path, &bytes)?;
        }
        if let Some(path) = selection.and_then(|s| s.effect_filter_path.as_ref()) {
            let mut filter = self.effect_browser.saved_filter(&self.effect_ids());
            // A slot position belongs to this track's chain, not to the next one.
            filter.slot = None;
            let bytes = serde_json::to_vec(&filter).map_err(|e| e.to_string())?;
            crate::state_store::atomic_write(path, &bytes)?;
        }
        Ok(())
    }

    pub(crate) fn configure_selection_preview(&mut self, path: &Path) -> Result<(), String> {
        let music = crate::smf::io::load(path)?;
        let timeline = all_tracks(&music, self.sample_rate())?;
        self.selection
            .as_mut()
            .ok_or("Selection mode required")?
            .timeline = Arc::new(timeline);
        Ok(())
    }
}

fn all_tracks(
    music: &crate::smf::model::Music,
    rate: u32,
) -> Result<crate::smf_timeline::Timeline, String> {
    let mut events = Vec::new();
    let mut duration = 0;
    for track in 0..music.tracks.len() {
        let extracted = crate::smf_extract::extract(music, track)?;
        if extracted.tracks[0]
            .events
            .last()
            .is_none_or(|e| e.tick == 0)
        {
            continue;
        }
        let timeline = crate::smf_timeline::Timeline::new(&extracted, rate)?;
        events.extend(timeline.events);
        duration = duration.max(timeline.duration);
    }
    if duration == 0 {
        return Err("Selection SMF has no duration".into());
    }
    events.sort_by_key(|e| e.0);
    Ok(crate::smf_timeline::Timeline { events, duration })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smf::model::{Event, Music, Track};

    #[test]
    fn preview_includes_every_track_and_conductor_end_and_format_two_tempos() {
        let event = |tick, status, meta_type, data| Event {
            tick,
            delta: 0,
            status,
            meta_type,
            data,
        };
        let mut music = Music {
            format: 1,
            division: 480,
            tracks: vec![
                Track {
                    events: vec![
                        event(0, 0xff, 0x51, vec![3, 208, 144]),
                        event(1920, 0xff, 0x2f, vec![]),
                    ],
                },
                Track {
                    events: vec![
                        event(480, 0x90, 0, vec![60, 80]),
                        event(960, 0x80, 0, vec![60, 0]),
                    ],
                },
                Track {
                    events: vec![
                        event(960, 0x91, 0, vec![65, 90]),
                        event(1440, 0x81, 0, vec![65, 0]),
                    ],
                },
            ],
        };
        let timeline = all_tracks(&music, 48000).unwrap();
        assert_eq!(timeline.duration, 48000);
        assert!(timeline.events.contains(&(12000, 0x2090_3c50)));
        assert!(timeline.events.contains(&(24000, 0x2091_415a)));
        music.format = 2;
        let timeline = all_tracks(&music, 48000).unwrap();
        assert!(timeline.events.contains(&(24000, 0x2090_3c50)));
        assert!(timeline.events.contains(&(48000, 0x2091_415a)));
        assert_eq!(timeline.duration, 72000);
    }

    #[test]
    fn exact_filter_roundtrip_retains_empty_query_and_does_not_reapply_word_fallback() {
        let root = crate::selection_tests::directory("filter sidecar");
        let input = root.join("track.mid");
        crate::selection_tests::input(&input, &Default::default());
        let _library = crate::native_library::load().unwrap();
        let mut app = App::prepare_mode(true);
        app.config_path = Ok(root.join("config.toml"));
        app.deferred_scan = false;
        app.configure_selection(input, crate::cli::Browser::Patch)
            .unwrap();
        let sidecar = root.join("filter.json");
        let filter = BrowserFilter {
            role: cmrt_patch_select::FilterGroup::ALL[1].label().into(),
            preset: "ALL".into(),
            query: String::new(),
            playing: None,
            selected: None,
        };
        std::fs::write(&sidecar, serde_json::to_vec(&filter).unwrap()).unwrap();
        app.patch_browser.set_initial_filter("church organ");
        app.configure_selection_filter(Some(sidecar.clone()))
            .unwrap();
        assert_eq!(app.patch_browser.query, "");
        assert!(app.patch_browser.initial_filter_fallback.is_none());
        app.patch_browser.query = "organ|church -bright".into();
        app.save_selection_filter().unwrap();
        let saved: BrowserFilter =
            serde_json::from_slice(&std::fs::read(sidecar).unwrap()).unwrap();
        assert_eq!(saved.query, "organ|church -bright");
        assert_eq!(saved.role, cmrt_patch_select::FilterGroup::ALL[1].label());
        assert!(saved.playing.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn effect_filter_sidecar_restores_category_kind_and_cursor_in_the_next_selection() {
        if crate::native_library::process::isolated() {
            return;
        }
        use crate::effect_browser::tests::candidate;
        let root = crate::selection_tests::directory("effect filter sidecar");
        let input = root.join("track.mid");
        crate::selection_tests::input(&input, &Default::default());
        let _library = crate::native_library::load().unwrap();
        let sidecar = root.join("effect-filter.json");
        let candidates = vec![
            candidate("Surge", "Big Hall", "Space", "Reverb"),
            candidate("Surge", "Tape Echo", "Time", "Delay"),
            candidate("Dragonfly", "Small Room", "Space", "Reverb"),
        ];
        let launch = || {
            let mut app = App::prepare_mode(true);
            app.config_path = Ok(root.join("config.toml"));
            app.deferred_scan = false;
            app.configure_selection(input.clone(), crate::cli::Browser::Effect)
                .unwrap();
            app.configure_effect_selection_filter(Some(sidecar.clone()))
                .unwrap();
            assert!(app.effect_browser.open);
            app.effect_browser.refresh(1, candidates.clone());
            app
        };
        let mut app = launch();
        assert!(app.effect_browser.selected.is_none());
        app.effect_browser.choose_category(1);
        app.effect_browser.choose_kind(1);
        app.effect_browser.select(2);
        app.save_selection_filter().unwrap();
        let saved: EffectBrowserFilter =
            serde_json::from_slice(&std::fs::read(&sidecar).unwrap()).unwrap();
        assert_eq!(saved.slot, None);

        let app = launch();
        let browser = &app.effect_browser;
        assert_eq!(browser.categories[browser.category], "Space");
        assert_eq!(browser.kinds[browser.kind], "Reverb");
        assert_eq!(
            browser.candidates[browser.selected.unwrap()].name,
            "Small Room"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
