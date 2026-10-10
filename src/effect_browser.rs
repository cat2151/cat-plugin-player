//! Effect browser state: Category / Kind / search filtering and the target slot.
//! Filtering follows the clap-mml-render-tui effect selector; applying reuses the
//! Random effect preparation and replacement path.
use crate::{
    random_effect_catalog::Candidate,
    status::{EffectBrowserFilter, EffectPresetKey},
    App,
};
use cmrt_tui_core::text_filter;
use eframe::egui;

pub(crate) const ALL: &str = "ALL";

#[derive(Default)]
pub(crate) struct EffectBrowser {
    pub open: bool,
    pub window_position: Option<egui::Pos2>,
    /// Installed candidates in catalog order, copied when the window opens.
    pub candidates: Vec<Candidate>,
    /// `None` until candidates are copied; opening clears it to pick up a rescan.
    catalog_generation: Option<u64>,
    /// `ALL` first, then sorted category names.
    pub categories: Vec<String>,
    pub category: usize,
    /// `ALL` first, then the kinds of the chosen category.
    pub kinds: Vec<String>,
    pub kind: usize,
    pub query: String,
    pub error: Option<String>,
    /// Candidate indices shown in the list.
    pub results: Vec<usize>,
    pub selected: Option<usize>,
    pub scroll_to_selected: bool,
    pub list_columns: usize,
    /// Vim-style count typed before a movement key.
    pub count: Option<usize>,
    /// Effect slot the next selection replaces; `None` adds an effect.
    pub target: Option<i32>,
    /// Latest selection not yet sent to preparation.
    pub desired: Option<Candidate>,
    /// The slot this browser last wrote and the preset now in it.
    last_applied: Option<(i32, Candidate)>,
    /// Saved Category / Kind / cursor, resolved once candidates name them again.
    restore: Option<EffectBrowserFilter>,
    /// Saved target position, resolved on the first open after effects are restored.
    restore_slot: Option<usize>,
}

pub(crate) fn same(a: &Candidate, b: &Candidate) -> bool {
    a.plugin_id == b.plugin_id && a.preset.path == b.preset.path && a.preset.value == b.preset.value
}

fn key(candidate: &Candidate) -> EffectPresetKey {
    EffectPresetKey {
        plugin_id: candidate.plugin_id.clone(),
        path: candidate.preset.path.clone(),
        value: candidate.preset.value.clone(),
    }
}

impl EffectBrowser {
    pub fn restored(filter: EffectBrowserFilter) -> Self {
        Self {
            query: filter.query.clone(),
            restore_slot: filter.slot,
            restore: Some(filter),
            ..Default::default()
        }
    }

    pub fn saved_filter(&self, effect_ids: &[i32]) -> EffectBrowserFilter {
        let name = |names: &[String], at: usize| {
            names
                .get(at)
                .filter(|&name| name != ALL)
                .cloned()
                .unwrap_or_default()
        };
        let (category, kind, selected) = match &self.restore {
            Some(filter) => (
                filter.category.clone(),
                filter.kind.clone(),
                filter.selected.clone(),
            ),
            None => (
                name(&self.categories, self.category),
                name(&self.kinds, self.kind),
                self.selected.map(|index| key(&self.candidates[index])),
            ),
        };
        EffectBrowserFilter {
            category,
            kind,
            query: self.query.clone(),
            slot: self
                .target
                .and_then(|id| effect_ids.iter().position(|&slot| slot == id))
                .or(self.restore_slot),
            selected,
        }
    }

    /// Previously browsed slot, else the last effect, else adding one.
    pub fn open_window(&mut self, effect_ids: &[i32]) {
        self.open = true;
        self.catalog_generation = None;
        if let Some(slot) = self.restore_slot.take() {
            self.target = effect_ids.get(slot).copied().or(self.target);
        }
        if !self.target.is_some_and(|id| effect_ids.contains(&id)) {
            self.target = self
                .last_applied
                .as_ref()
                .map(|(id, _)| *id)
                .filter(|id| effect_ids.contains(id))
                .or_else(|| effect_ids.last().copied());
        }
        self.reveal_applied();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.window_position = None;
        self.count = None;
        self.desired = None;
    }

    /// A removed or replaced slot falls back to the last effect, or to adding one.
    pub fn sync_target(&mut self, effect_ids: &[i32]) {
        if self.target.is_some_and(|id| !effect_ids.contains(&id)) {
            self.target = effect_ids.last().copied();
        }
    }

    pub fn applied(&mut self, id: i32, candidate: Candidate) {
        self.target = Some(id);
        self.last_applied = Some((id, candidate));
    }

    /// The preset in the target slot, if this browser put it there.
    pub fn applied_in_target(&self) -> Option<&Candidate> {
        self.last_applied
            .as_ref()
            .filter(|(id, _)| Some(*id) == self.target)
            .map(|(_, candidate)| candidate)
    }

    /// Cursor only: the slot already holds that preset, so nothing is requested.
    fn reveal_applied(&mut self) {
        if let Some(applied) = self.applied_in_target() {
            let found = self
                .results
                .iter()
                .copied()
                .find(|&index| same(&self.candidates[index], applied));
            if found.is_some() {
                self.selected = found;
            }
        }
        self.scroll_to_selected = true;
    }

    pub fn stale(&self, generation: u64) -> bool {
        self.catalog_generation != Some(generation)
    }

    /// Conditions and the cursor survive by name and identity.
    pub fn refresh(&mut self, generation: u64, candidates: Vec<Candidate>) {
        self.catalog_generation = Some(generation);
        let restore = self
            .restore
            .take_if(|_| !candidates.is_empty())
            .map(|filter| (Some(filter.category), Some(filter.kind), filter.selected));
        let (category, kind, selected) = restore.unwrap_or_else(|| {
            (
                self.categories.get(self.category).cloned(),
                self.kinds.get(self.kind).cloned(),
                self.selected.map(|index| key(&self.candidates[index])),
            )
        });
        self.candidates = candidates;
        self.categories = names(self.candidates.iter().map(|c| c.category.as_str()));
        self.category = position(&self.categories, category);
        self.rebuild_kinds();
        self.kind = position(&self.kinds, kind);
        self.selected = selected.and_then(|selected| {
            self.candidates
                .iter()
                .position(|candidate| key(candidate) == selected)
        });
        self.filter(false);
        self.reveal_applied();
    }

    pub fn choose_category(&mut self, category: usize) {
        if self.category != category {
            self.category = category;
            self.rebuild_kinds();
            self.filter(true);
        }
    }

    pub fn choose_kind(&mut self, kind: usize) {
        if self.kind != kind {
            self.kind = kind;
            self.filter(true);
        }
    }

    fn chosen(names: &[String], at: usize) -> Option<&str> {
        names
            .get(at)
            .map(String::as_str)
            .filter(|&name| name != ALL)
    }

    fn rebuild_kinds(&mut self) {
        let category = Self::chosen(&self.categories, self.category);
        self.kinds = names(
            self.candidates
                .iter()
                .filter(|c| category.is_none_or(|category| c.category == category))
                .map(|c| c.kind.as_str()),
        );
        self.kind = 0;
    }

    /// `changed` moves the cursor to the first result when the selection was filtered out.
    pub fn filter(&mut self, changed: bool) {
        let condition = match text_filter::compile_condition(&self.query) {
            Ok(condition) => condition,
            Err(error) => {
                // Keep the previous list while a pattern is half typed.
                self.error = Some(error);
                return;
            }
        };
        self.error = None;
        let category = Self::chosen(&self.categories, self.category);
        let kind = Self::chosen(&self.kinds, self.kind);
        self.results = (0..self.candidates.len())
            .filter(|&index| {
                let c = &self.candidates[index];
                category.is_none_or(|category| c.category == category)
                    && kind.is_none_or(|kind| c.kind == kind)
                    && text_filter::matches_any_field(
                        &condition,
                        &[&c.name, &c.category, &c.kind, &c.plugin_name],
                    )
            })
            .collect();
        if changed {
            self.scroll_to_selected = true;
            if !self
                .selected
                .is_some_and(|index| self.results.contains(&index))
            {
                self.selected = None;
                if let Some(&first) = self.results.first() {
                    self.select(first);
                }
            }
        }
    }

    pub fn select(&mut self, index: usize) {
        let Some(candidate) = self.candidates.get(index) else {
            return;
        };
        self.selected = Some(index);
        self.scroll_to_selected = true;
        if self
            .applied_in_target()
            .is_some_and(|applied| same(applied, candidate))
        {
            self.desired = None;
        } else {
            self.desired = Some(candidate.clone());
        }
    }

    pub fn select_random(&mut self) {
        if let Some(index) =
            crate::random_patch::choose(self.results.iter().copied(), &mut rand::thread_rng())
        {
            self.select(index);
        }
    }
}

fn names<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut names: Vec<String> = values.map(str::to_owned).collect();
    names.sort();
    names.dedup();
    names.insert(0, ALL.into());
    names
}

fn position(names: &[String], name: Option<String>) -> usize {
    name.and_then(|name| names.iter().position(|n| *n == name))
        .unwrap_or_default()
}

impl App {
    pub(crate) fn open_effect_browser(&mut self) {
        let ids = self.effect_ids();
        self.effect_browser.open_window(&ids);
    }

    /// Sends the latest selection once the previous native work has finished.
    pub(crate) fn poll_effect_browser(&mut self, ctx: &egui::Context) {
        let ids = self.effect_ids();
        self.effect_browser.sync_target(&ids);
        let generation = self.random_effect_catalog.generation;
        if self.effect_browser.open && self.effect_browser.stale(generation) {
            let candidates = self
                .random_effect_catalog
                .installed(&self.plugins)
                .cloned()
                .collect();
            self.effect_browser.refresh(generation, candidates);
        }
        if !self.effect_browser.open || self.effect_browser.desired.is_none() || self.actions_busy()
        {
            return;
        }
        let candidate = self.effect_browser.desired.take().unwrap();
        let target = self.effect_browser.target;
        if self.instrument_id().is_none() {
            self.status = "Browse effect unavailable: load an instrument first".into();
        } else if self.effect_plugin_taken(&candidate.plugin_id, target) {
            self.status = format!(
                "Browse effect unavailable: {} is already connected in another slot",
                candidate.plugin_name
            );
        } else {
            self.start_effect(candidate, target, true, ctx);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
