//! Report named patch cursors separately from pane filters and audio selections.
use crate::{
    resident_ipc::{Open, ToEditor},
    App,
};

impl App {
    pub(crate) fn report_resident_cursor(&mut self) {
        let Some(resident) = &self.resident else {
            return;
        };
        let (Some(Open::Patch { .. } | Open::Layer { .. }), Some((context, _, _))) =
            (resident.shown, &resident.panes)
        else {
            return;
        };
        let context = context.clone();
        let patch = self.patch_browser.saved_cursor();
        if resident.cursor == patch {
            return;
        }
        let resident = self.resident.as_mut().unwrap();
        match resident.sender.send(&ToEditor::Cursor {
            context: context.clone(),
            patch: patch.clone(),
        }) {
            Ok(()) => {
                resident.cursor_cache.insert(context, patch.clone());
                resident.cursor = patch;
            }
            Err(error) => self.status = format!("Cannot save patch cursor: {error}"),
        }
    }
}
