//! Drum catalog metadata and persistent cursor, separate from instrument state.
use crate::{patch_catalog::Candidate, status::PatchIdentity};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Focus {
    #[default]
    Kits,
    Notes,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Saved {
    pub query: String,
    pub selected: Option<PatchIdentity>,
    #[serde(default)]
    pub applied: Option<PatchIdentity>,
    pub notes: Vec<(PatchIdentity, u8)>,
    pub focus: Focus,
}

#[derive(Default)]
pub(crate) struct Browser {
    pub open: bool,
    pub prepared: Option<crate::patch_preparation::Prepared>,
    pub confirm: bool,
    pub focus_window: bool,
    search: crate::command_palette::search::SearchQuery,
    pub saved: Saved,
    pub error: Option<String>,
    pub kits: Vec<Candidate>,
    pub results: Vec<usize>,
    pub selected: Option<usize>,
    pub generation: Option<u64>,
    pub editing: Option<String>,
    pub composing: bool,
    pub awaiting: Option<(PatchIdentity, Audition)>,
    pub scroll: bool,
    pub opening_audition: bool,
    pub loading_since: Option<std::time::Instant>,
    pub audition_started: bool,
    pub compare_velocity: bool,
    pub main_audition: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Audition {
    All,
    Note(u8),
}

impl Browser {
    pub fn restored(config: &Result<std::path::PathBuf, String>) -> Self {
        match config
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|p| Saved::load(p))
        {
            Ok(saved) => Self {
                saved,
                ..Default::default()
            },
            Err(error) => Self {
                error: Some(error),
                ..Default::default()
            },
        }
    }

    pub fn selected(&self) -> Option<&Candidate> {
        self.selected.and_then(|index| self.kits.get(index))
    }

    pub fn filter(&mut self) {
        self.search.update(&self.saved.query);
        self.results = self
            .kits
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                self.search
                    .matches(&format!("{} {}", c.plugin_name, c.display))
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = self
            .saved
            .selected
            .as_ref()
            .and_then(|key| {
                self.results
                    .iter()
                    .copied()
                    .find(|&i| same(key, &identity(&self.kits[i])))
            })
            .or_else(|| self.results.first().copied());
        if let Some(candidate) = self.selected() {
            self.saved.selected = Some(identity(candidate));
        }
        self.scroll = true;
    }
}

pub(crate) fn identity(candidate: &Candidate) -> PatchIdentity {
    PatchIdentity {
        format: candidate.format.clone(),
        plugin_id: candidate.plugin_id.clone(),
        path: candidate.path.clone(),
        display: candidate.display.clone(),
    }
}

pub(crate) fn same(a: &PatchIdentity, b: &PatchIdentity) -> bool {
    a.format == b.format && a.plugin_id == b.plugin_id && a.path == b.path
}

pub(crate) fn notes(candidate: &Candidate) -> Option<Vec<u8>> {
    candidate.measurement.drum_kit_notes.as_ref().map(|notes| {
        let mut notes: Vec<_> = notes.iter().copied().filter(|&n| n < 128).collect();
        notes.sort_unstable();
        notes.dedup();
        notes
    })
}

pub(crate) fn kit_metadata(
    candidate: &Candidate,
    layer: u64,
) -> Result<crate::drum_kit::Kit, String> {
    let notes = notes(candidate)
        .filter(|n| !n.is_empty())
        .ok_or("Kit note assignments unavailable")?;
    let kit = crate::drum_kit::Kit {
        layer,
        rows: None,
        muted: vec![],
        swing: Default::default(),
        notes: notes
            .into_iter()
            .map(|note| crate::drum_kit::Note {
                note,
                name: candidate
                    .measurement
                    .drum_kit_note_names
                    .iter()
                    .find(|(n, _)| *n == note)
                    .map_or(String::new(), |(_, name)| name.clone()),
                velocity: 100,
            })
            .collect(),
    };
    kit.validate()?;
    Ok(kit)
}

pub(crate) fn note_name(candidate: &Candidate, note: u8) -> &str {
    candidate
        .measurement
        .drum_kit_note_names
        .iter()
        .find(|(n, _)| *n == note)
        .map_or("名前不明", |(_, name)| name.as_str())
}

pub(crate) fn one_shot(candidate: &Candidate, note: u8) -> &'static str {
    if candidate
        .measurement
        .drum_kit_one_shot_notes
        .contains(&note)
    {
        "one-shot"
    } else if candidate
        .path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sfz"))
    {
        "not one-shot"
    } else {
        // The producer does not measure one-shot behavior for every format.
        "one-shot: 未確認"
    }
}

impl Saved {
    pub fn load(config: &Path) -> Result<Self, String> {
        let path = config.with_file_name("drum-kit-browser.json");
        match std::fs::read(&path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    pub fn save(&self, config: &Path) -> Result<(), String> {
        // A damaged existing file must never be silently replaced with defaults.
        Self::load(config)?;
        crate::state_store::atomic_write(
            &config.with_file_name("drum-kit-browser.json"),
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }

    pub fn note(&self, candidate: &Candidate) -> Option<u8> {
        let key = identity(candidate);
        let available = notes(candidate)?;
        self.notes
            .iter()
            .find(|(id, _)| same(id, &key))
            .map(|(_, note)| *note)
            .filter(|note| available.contains(note))
            .or_else(|| available.first().copied())
    }

    pub fn set_note(&mut self, candidate: &Candidate, note: u8) {
        let key = identity(candidate);
        if let Some((_, previous)) = self.notes.iter_mut().find(|(id, _)| same(id, &key)) {
            *previous = note;
        } else {
            self.notes.push((key, note));
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
