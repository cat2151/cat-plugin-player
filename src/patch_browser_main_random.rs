//! Main-window `r`: the browser's own Random over its saved filter, with the window closed.
use super::*;

/// The closed browser builds its catalog and starts native work only for this request.
#[derive(Default, PartialEq, Debug)]
pub(crate) enum MainRandom {
    #[default]
    Idle,
    /// Waiting for the catalog snapshot.
    Catalog,
    /// A patch is chosen; waiting for its preparation to start.
    Start,
}

impl App {
    pub(crate) fn request_main_random(&mut self) {
        if !self.patch_browser.open {
            self.patch_browser.main_random = MainRandom::Catalog;
            self.poll_main_random();
        }
    }

    /// Call after the snapshot update, before the patch requests are polled.
    pub(crate) fn poll_main_random(&mut self) {
        if let Some(status) = self.patch_browser.poll_main_random() {
            self.status = status;
        }
    }
}

impl Browser {
    /// Returns the status to show when no sound will start.
    pub(crate) fn poll_main_random(&mut self) -> Option<String> {
        if self.main_random == MainRandom::Start && self.requests.desired.is_none() {
            self.main_random = MainRandom::Idle;
        }
        if self.main_random != MainRandom::Catalog {
            return None;
        }
        if let Some(error) = &self.build_error {
            self.main_random = MainRandom::Idle;
            return Some(format!("Random patch failed: {error}"));
        }
        self.snapshot.as_ref()?;
        self.select_random();
        self.main_random = MainRandom::Idle;
        if self.requests.desired.is_some() {
            self.main_random = MainRandom::Start;
            return None;
        }
        if let Some(error) = &self.error {
            return Some(format!(
                "Random patch failed: invalid browser search: {error}"
            ));
        }
        let Some(ticket) = self
            .requests
            .selected
            .as_ref()
            .filter(|_| !self.results.is_empty())
        else {
            return Some("Random patch: no patch matches the browser filter".into());
        };
        let candidate = &ticket.candidate;
        Some(if candidate.measurement.is_heavy_offline_load() {
            format!(
                "Random chose {} — {}: open Browse patches (T) and press Space to load it",
                candidate.plugin_name, candidate.display
            )
        } else {
            format!(
                "Random chose the current patch again: {} — {}",
                candidate.plugin_name, candidate.display
            )
        })
    }
}
