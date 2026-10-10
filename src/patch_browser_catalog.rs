//! An immutable selector snapshot; indices never cross snapshot generations.
use crate::random_patch_catalog::Candidate;
use cmrt_patch_select::{
    filter_candidates, sort_for_selector, FilterGroup, FilterPreset, PatchCatalogEntry,
    PreparedPresets,
};
use cmrt_patches::{PatchRoleIndex, PatchRoleInput};
use std::collections::{BTreeMap, HashMap, VecDeque};

pub(crate) struct Snapshot {
    pub candidates: Vec<Candidate>,
    pub entries: Vec<PatchCatalogEntry>,
    pub presets: Vec<Vec<FilterPreset>>,
    pub instrument_names: Vec<String>,
}

impl Snapshot {
    pub fn build(candidates: Vec<Candidate>, users: &[(String, String)]) -> Result<Self, String> {
        // Preserve identity even when distinct plugins have identical display/name/category.
        let key = |entry: &PatchCatalogEntry| {
            (
                entry.display().to_owned(),
                entry.plugin_sort_key().to_owned(),
                entry.selector_category().map(str::to_owned),
            )
        };
        let mut by_entry: HashMap<_, VecDeque<Candidate>> = HashMap::new();
        let mut entries = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            entries.push(candidate.entry.clone());
            by_entry
                .entry(key(&candidate.entry))
                .or_default()
                .push_back(candidate);
        }
        sort_for_selector(&mut entries);
        let candidates: Vec<_> = entries
            .iter()
            .map(|entry| by_entry.get_mut(&key(entry)).unwrap().pop_front().unwrap())
            .collect();
        let mut groups: BTreeMap<_, Vec<usize>> = BTreeMap::new();
        for (index, candidate) in candidates.iter().enumerate() {
            groups
                .entry((&candidate.format, &candidate.plugin_id))
                .or_default()
                .push(index);
        }
        // PatchRoleIndex keys on display: isolate plugins before using the shared classifier.
        let empty = PreparedPresets::build(&[], users, &PatchRoleIndex::default())?;
        let mut presets: Vec<_> = (0..FilterGroup::ALL.len())
            .map(|role| empty.for_role(role).to_vec())
            .collect();
        let mut matches: Vec<Vec<Vec<usize>>> = presets
            .iter()
            .map(|role| vec![Vec::new(); role.len()])
            .collect();
        for indices in groups.values() {
            let local: Vec<_> = indices
                .iter()
                .map(|&index| entries[index].clone())
                .collect();
            let normalized: Vec<_> = local
                .iter()
                .map(|entry| entry.display().to_lowercase())
                .collect();
            let roles = PatchRoleIndex::build(
                local
                    .iter()
                    .zip(&normalized)
                    .map(|(entry, normalized)| PatchRoleInput {
                        display: entry.display(),
                        normalized_display: normalized,
                        selector_category: entry.selector_category(),
                        plugin: Some(entry.plugin_sort_key()),
                    }),
                users,
            );
            let prepared = PreparedPresets::build(&local, users, &roles)?;
            for (role, role_matches) in matches.iter_mut().enumerate() {
                for (preset, target) in prepared.for_role(role).iter().zip(role_matches) {
                    target.extend(preset.matches.iter().map(|&index| indices[index]));
                }
            }
        }
        for (role, role_matches) in presets.iter_mut().zip(matches) {
            for (preset, mut indices) in role.iter_mut().zip(role_matches) {
                indices.sort_unstable();
                preset.matches = indices.into();
            }
        }
        let instrument_names = candidates
            .iter()
            .map(|candidate| candidate.plugin_name.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(Self {
            candidates,
            entries,
            presets,
            instrument_names,
        })
    }

    pub fn filter(&self, role: usize, preset: usize, query: &str) -> Result<Vec<usize>, String> {
        filter_candidates(&self.entries, &self.presets[role][preset].matches, query)
    }
}

#[cfg(test)]
pub(crate) mod tests;
