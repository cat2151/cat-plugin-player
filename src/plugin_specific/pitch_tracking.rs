//! Measured keyboard tracking moves melodic percussion to lead (ADR 0040).
use crate::patch_catalog::Candidate;
use cmrt_patches::pitch_tracking::{
    pitched_percussion_indices, PitchTrackingTable, TrackingCandidate, PITCHED_PERCUSSION,
    PITCH_TRACKING_FILE, PITCH_TRACKING_URL,
};
use cmrt_patches::PatchRoleInput;
use std::{path::Path, time::Duration};

pub(super) fn apply(candidates: &mut [Candidate]) {
    let result = crate::config::path().and_then(|config| {
        let destination = config
            .parent()
            .ok_or("config has no parent")?
            .join("patch-catalog")
            .join(PITCH_TRACKING_FILE);
        load(&destination)
    });
    match result {
        Ok(table) => apply_table(candidates, &table),
        Err(error) => eprintln!("Pitch tracking categories unavailable: {error}"),
    }
}

fn load(destination: &Path) -> Result<PitchTrackingTable, String> {
    // Called on the existing catalog worker, after audio has started.
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(3)))
            .build(),
    );
    let fetched = agent
        .get(PITCH_TRACKING_URL)
        .call()
        .and_then(|mut r| r.body_mut().read_to_vec())
        .map_err(|error| error.to_string())
        .and_then(|bytes| {
            let table = PitchTrackingTable::parse(&bytes)?;
            crate::state_store::atomic_write(destination, &bytes)?;
            Ok(table)
        });
    fetched.or_else(|_| {
        PitchTrackingTable::parse(&std::fs::read(destination).map_err(|e| e.to_string())?)
    })
}

/// Candidates on the drum percussion row whose pitch follows the note move to lead.
pub(crate) fn apply_table(candidates: &mut [Candidate], table: &PitchTrackingTable) {
    let normalized: Vec<_> = candidates
        .iter()
        .map(|candidate| candidate.display.to_lowercase())
        .collect();
    // Grouped like the patch browser, which classifies each plugin separately.
    let inputs: Vec<_> = candidates
        .iter()
        .zip(&normalized)
        .map(|(candidate, normalized)| TrackingCandidate {
            group: (candidate.format.as_str(), candidate.plugin_id.as_str()),
            plugin_id: Some(candidate.plugin_id.as_str()),
            role: PatchRoleInput {
                display: &candidate.display,
                normalized_display: normalized,
                selector_category: candidate.entry.selector_category(),
                plugin: Some(candidate.entry.plugin_sort_key()),
            },
        })
        .collect();
    let moved = pitched_percussion_indices(&inputs, table);
    for index in moved {
        let candidate = &mut candidates[index];
        candidate.entry = candidate
            .entry
            .clone()
            .with_selector_category(PITCHED_PERCUSSION.into());
    }
}

#[cfg(test)]
mod tests;
