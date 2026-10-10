//! Browser state and integration with the existing main-thread patch transaction.
use crate::status::{BrowserFilter, PatchIdentity};
use crate::{patch_browser_catalog::Snapshot, patch_filter_store::Store};
use crate::{patch_browser_requests::Requests, random_patch::Preparation, App};
use crate::{patch_browser_ui::RowTint, random_patch_catalog::Candidate};
use cmrt_patch_select::FilterGroup;
use std::sync::mpsc::Receiver;

pub(crate) type SnapshotResult = Result<(Snapshot, Option<Store>), String>;

#[derive(Default)]
pub(crate) struct Browser {
    pub open: bool,
    pub window_position: Option<eframe::egui::Pos2>,
    pub requests: Requests,
    pub preparation: Preparation,
    pub snapshot: Option<Snapshot>,
    pub store: Option<Store>,
    pub build: Option<Receiver<SnapshotResult>>,
    pub generation: u64,
    pub build_generation: u64,
    pub role: usize,
    pub preset: usize,
    pub query: String,
    pub solo_draft: Option<String>,
    pub condition_menu: bool,
    pub results: Vec<usize>,
    pub row_labels: Vec<String>,
    pub row_tints: Vec<RowTint>,
    /// Saved preset label, resolved once the first snapshot names the presets again.
    pub restore_preset: Option<String>,
    /// Saved playing patch, resolved once the first snapshot lists candidates again.
    pub restore_playing: Option<PatchIdentity>,
    pub selected: Option<usize>,
    pub error: Option<String>,
    pub build_error: Option<String>,
    pub scroll_to_selected: bool,
    /// Set on open; the cursor moves to the playing patch once a snapshot exists.
    pub reveal_pending: bool,
    pub list_columns: usize,
    /// Vim-style count typed before a movement key.
    pub count: Option<usize>,
}

impl Browser {
    pub fn restored(filter: BrowserFilter) -> Self {
        Self {
            role: FilterGroup::ALL
                .iter()
                .position(|group| group.label() == filter.role)
                .unwrap_or_default(),
            restore_preset: Some(filter.preset),
            restore_playing: filter.playing,
            query: filter.query,
            ..Default::default()
        }
    }

    pub fn saved_filter(&self) -> BrowserFilter {
        let preset = self.restore_preset.clone().or_else(|| {
            let presets = &self.snapshot.as_ref()?.presets[self.role];
            Some(presets.get(self.preset)?.label.clone())
        });
        BrowserFilter {
            role: FilterGroup::ALL[self.role].label().into(),
            preset: preset.unwrap_or_default(),
            query: self.query.clone(),
            playing: self
                .requests
                .applied
                .as_ref()
                .map(identity)
                .or_else(|| self.restore_playing.clone()),
        }
    }

    /// A patch applied since startup wins over the saved one.
    pub fn resolve_restored_playing(&mut self) {
        let (Some(saved), Some(snapshot)) = (self.restore_playing.take(), &self.snapshot) else {
            return;
        };
        if self.requests.applied.is_none() {
            self.requests.applied = snapshot
                .candidates
                .iter()
                .find(|candidate| identity(candidate) == saved)
                .cloned();
        }
    }

    /// Unknown labels (a deleted user preset) fall back to the role's ALL.
    pub fn resolve_restored_preset(&mut self) {
        let (Some(label), Some(snapshot)) = (&self.restore_preset, &self.snapshot) else {
            return;
        };
        self.preset = snapshot.presets[self.role]
            .iter()
            .position(|preset| preset.label == *label)
            .unwrap_or_default();
        self.restore_preset = None;
    }

    pub fn open_window(&mut self) {
        self.open = true;
        self.reveal_pending = true;
    }

    /// The cursor stays where it was when closed unless the playing patch is listed.
    pub fn close(&mut self) {
        self.open = false;
        self.window_position = None;
        self.solo_draft = None;
        self.condition_menu = false;
        self.count = None;
        self.requests.invalidate();
    }

    /// Cursor only: the playing patch is already live, so nothing is requested.
    pub fn reveal_playing(&mut self) {
        let (Some(snapshot), Some(playing)) = (&self.snapshot, &self.requests.applied) else {
            self.scroll_to_selected = true;
            return;
        };
        let playing = identity(playing);
        if let Some(index) = self
            .results
            .iter()
            .copied()
            .find(|&index| identity(&snapshot.candidates[index]) == playing)
        {
            self.selected = Some(index);
        }
        self.scroll_to_selected = true;
    }

    pub fn select(&mut self, index: usize) {
        if self.error.is_some() {
            return;
        }
        if let Some(candidate) = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.candidates.get(index))
        {
            self.selected = Some(index);
            self.requests.select(candidate.clone(), self.generation);
            self.scroll_to_selected = true;
        }
    }

    pub fn filter(&mut self, changed: bool) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        self.preset = self.preset.min(snapshot.presets[self.role].len() - 1);
        match snapshot.filter(self.role, self.preset, &self.query) {
            Ok(results) => {
                self.error = None;
                self.results = results;
                self.refresh_row_labels();
                if changed {
                    self.scroll_to_selected = true;
                }
                if changed
                    && !self
                        .selected
                        .is_some_and(|index| self.results.contains(&index))
                {
                    self.requests.invalidate();
                    self.selected = None;
                    if let Some(&first) = self.results.first() {
                        self.select(first);
                    }
                }
            }
            Err(error) => {
                self.error = Some(error);
                self.results.clear();
                self.row_labels.clear();
                self.row_tints.clear();
                self.selected = None;
                self.requests.invalidate();
            }
        }
    }
}

/// A rescan may change the bundle path of the same patch, so it is not part of identity.
fn identity(candidate: &Candidate) -> PatchIdentity {
    PatchIdentity {
        format: candidate.format.clone(),
        plugin_id: candidate.plugin_id.clone(),
        path: candidate.path.clone(),
        display: candidate.display.clone(),
    }
}

impl App {
    pub(crate) fn invalidate_browser_live(&mut self) {
        self.patch_browser.requests.invalidate();
        self.patch_browser.requests.applied = None;
        self.patch_browser.restore_playing = None;
        self.patch_browser.selected = None;
    }
    pub(crate) fn patch_operation_name(&self) -> &'static str {
        if self.patch_browser.requests.applying.is_some() {
            "Browse patch"
        } else {
            "Random patch"
        }
    }

    pub(crate) fn poll_browser_patch(&mut self, ctx: &eframe::egui::Context) {
        if let Some(result) = self.patch_browser.preparation.poll() {
            match result {
                Ok(prepared) => {
                    // External live changes invalidate requests. Own preparation has now drained.
                    if self.scanning
                        || self.deferred_scan
                        || self.restoring
                        || self.pending.is_some()
                        || self.random_patch.busy()
                        || self.random_effect.busy()
                    {
                        self.patch_browser.requests.invalidate();
                    }
                    if self.patch_browser.requests.ready().is_some() {
                        self.apply_random_patch(prepared);
                    }
                }
                Err(error) => {
                    let candidate = self
                        .patch_browser
                        .requests
                        .preparing
                        .as_ref()
                        .map(|ticket| ticket.candidate.clone());
                    if self.patch_browser.requests.failed_preparation() {
                        let target = candidate.map_or(String::new(), |candidate| {
                            format!(
                                "{} — {} ({}): ",
                                candidate.plugin_name,
                                candidate.display,
                                candidate.path.display()
                            )
                        });
                        self.status = format!("Browse patch failed: {target}{error}");
                    }
                }
            }
        }
        if !self.patch_browser.open || self.actions_busy() {
            return;
        }
        if let Some(mut ticket) = self.patch_browser.requests.next() {
            let installed = self.plugins.iter().find(|plugin| {
                plugin.format == ticket.candidate.format
                    && plugin.id == ticket.candidate.plugin_id
                    && plugin.kind == crate::plugin_list::PluginKind::Instrument
            });
            let Some(installed) = installed else {
                self.patch_browser.requests.failed_preparation();
                self.status = "Browse patch failed: target plugin no longer available".into();
                return;
            };
            ticket.candidate.bundle_path = installed.bundle_path.clone().into();
            self.status = format!(
                "Preparing browse patch: {} — {}...",
                ticket.candidate.plugin_name, ticket.candidate.display
            );
            self.patch_browser.preparation.start(ticket.candidate, ctx);
        }
    }
}
