//! Resident kit browsing keeps preview state separate from the editor song.
use crate::{
    drum_browser::{identity, same},
    resident_ipc::Open,
    App,
};
impl App {
    pub(crate) fn resident_kit(&self) -> bool {
        self.resident
            .as_ref()
            .is_some_and(|r| matches!(r.shown, Some(Open::Kit { .. })))
    }
    pub(crate) fn open_resident_kit(&mut self) {
        self.stop_drum_preview();
        self.drum_browser.prepared = None;
        self.drum_browser.open = true;
        self.drum_browser.focus_window = true;
        self.sync_drum_catalog();
    }
    pub(crate) fn confirm_resident_kit(&mut self) {
        if !self.resident_kit() || self.resident_waiting() {
            return;
        }
        let ready = self.drum_browser.prepared.as_ref().is_some_and(|p| {
            self.drum_browser
                .selected()
                .is_some_and(|c| same(&identity(c), &identity(&p.candidate)))
                && self
                    .patch_browser
                    .requests
                    .applied
                    .as_ref()
                    .is_some_and(|c| same(&identity(c), &identity(&p.candidate)))
        });
        if ready {
            self.drum_browser.confirm = false;
            let prepared = self.drum_browser.prepared.take().unwrap();
            self.offer_patch(prepared);
            self.stop_drum_audio();
        } else if !self.actions_busy() {
            self.request_drum_confirmation();
            self.drum_browser.confirm = self.drum_browser.awaiting.is_some();
        } else {
            self.drum_browser.confirm = true;
        }
    }
}
