//! Cursor persistence uses the complete patch name and source identity, never a row.
use super::*;

impl Browser {
    pub fn saved_cursor(&self) -> Option<PatchIdentity> {
        self.restore_selected.clone().or_else(|| {
            let snapshot = self.snapshot.as_ref()?;
            snapshot.candidates.get(self.selected?).map(identity)
        })
    }

    /// Returns true when a saved cursor takes precedence over the playing patch.
    pub(super) fn reveal_saved_cursor(&mut self) -> bool {
        let Some(saved) = &self.restore_selected else {
            return false;
        };
        let Some(snapshot) = &self.snapshot else {
            return true;
        };
        self.selected = self
            .results
            .iter()
            .copied()
            .find(|&index| identity(&snapshot.candidates[index]) == *saved);
        self.restore_selected = None;
        self.scroll_to_selected = true;
        true
    }
}
