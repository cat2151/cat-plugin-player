//! Monokai tints that tell rows apart: plugin, left-pane preset, and parent folder.
use super::*;
use crate::patch_catalog::Candidate;
use cmrt_patch_select::FilterPreset;
use std::collections::HashMap;

/// Palette slots per row; `None` keeps the ordinary text or cell color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowTint {
    pub prefix_len: usize,
    pub plugin: Option<usize>,
    pub name: Option<usize>,
    pub background: Option<usize>,
}

/// Monokai without its pink, which already marks heavy sample loads. (dark, light)
const MONOKAI: [(Color32, Color32); 5] = [
    (
        Color32::from_rgb(102, 217, 239),
        Color32::from_rgb(0, 125, 155),
    ),
    (
        Color32::from_rgb(166, 226, 46),
        Color32::from_rgb(70, 125, 0),
    ),
    (
        Color32::from_rgb(253, 151, 31),
        Color32::from_rgb(185, 90, 0),
    ),
    (
        Color32::from_rgb(174, 129, 255),
        Color32::from_rgb(105, 55, 200),
    ),
    (
        Color32::from_rgb(230, 219, 116),
        Color32::from_rgb(135, 115, 0),
    ),
];

pub(crate) fn color(slot: usize, dark: bool) -> Color32 {
    let (on_dark, on_light) = MONOKAI[slot % MONOKAI.len()];
    if dark {
        on_dark
    } else {
        on_light
    }
}

/// Plugin prefix by plugin when names are shown; background by the left pane's preset
/// (only under a role's ALL), else by category; name by the patch's parent folder.
pub(super) fn tints(
    candidates: &[Candidate],
    results: &[usize],
    presets: Option<&[FilterPreset]>,
    mixed: bool,
) -> Vec<RowTint> {
    let rows: Vec<_> = results.iter().map(|&index| &candidates[index]).collect();
    let plugins = mixed
        .then(|| {
            slots(
                rows.iter()
                    .map(|c| Some((&c.format, &c.plugin_id)))
                    .collect(),
            )
        })
        .flatten();
    let backgrounds = presets
        .and_then(|presets| {
            slots(
                results
                    .iter()
                    .map(|&index| first_preset(presets, index))
                    .collect(),
            )
        })
        .or_else(|| slots(rows.iter().map(|c| c.entry.selector_category()).collect()));
    let names = slots(rows.iter().map(|c| parent_folder(&c.display)).collect());
    let pick = |slots: &Option<Vec<Option<usize>>>, at: usize| slots.as_ref().and_then(|s| s[at]);
    (0..results.len())
        .map(|at| RowTint {
            plugin: pick(&plugins, at),
            name: pick(&names, at),
            background: pick(&backgrounds, at),
            ..Default::default()
        })
        .collect()
}

/// The folder directly above the patch, if any.
fn parent_folder(display: &str) -> Option<&str> {
    let mut parts = display.rsplit(['/', '\\']);
    parts.next();
    parts.next()
}

/// The first named preset holding the row; ALL and Favorite are not kinds of patch.
fn first_preset(presets: &[FilterPreset], index: usize) -> Option<usize> {
    presets.iter().enumerate().skip(1).find_map(|(at, preset)| {
        (!preset.is_favorite && preset.matches.binary_search(&index).is_ok()).then_some(at)
    })
}

/// Colors in order of first appearance, or `None` when every row has the same key.
pub(crate) fn slots<K: Eq + std::hash::Hash>(keys: Vec<Option<K>>) -> Option<Vec<Option<usize>>> {
    if keys.windows(2).all(|pair| pair[0] == pair[1]) {
        return None;
    }
    let mut seen = HashMap::new();
    Some(
        keys.into_iter()
            .map(|key| {
                let next = seen.len();
                key.map(|key| *seen.entry(key).or_insert(next))
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests;
