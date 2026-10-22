//! Catalog-derived permission to omit Surge's provisional settling delay (ADR 0008).
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

pub(crate) mod measure;
mod runtime;
#[cfg(test)]
mod tests;
pub(crate) use runtime::{Permit, Runtime};
pub(crate) const ID: &str = "org.surge-synth-team.surge-xt";
const FILE: &str = "surge-xt-start-delay.json";
const URL: &str =
    "https://raw.githubusercontent.com/cat2151/cat-music-patterns/main/surge-xt-start-delay.json";

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Measurement {
    pub candidate_wait_ms: u32,
    pub reference_wait_ms: u32,
    pub capture_ms: u32,
    pub sample_rate: u32,
    pub note: u8,
    pub velocity: u8,
    pub cc1: u8,
    pub onset_ratio: f64,
    pub onset_window_frames: usize,
    pub trials_per_wait: u32,
    pub plugin_sha256: String,
    pub renderer_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Table {
    pub format_version: u32,
    pub plugin_format: String,
    pub plugin_id: String,
    pub measurement: Measurement,
    pub max_onset_delta_ms: f64,
    pub entries: BTreeMap<String, Entry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub fxp_sha256: String,
    pub state_sha256: Option<String>,
    pub candidate_onset_ms: Option<f64>,
    pub reference_onset_ms: Option<f64>,
    pub outcome: Outcome,
    pub zero_wait_allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    Measured,
    Silent,
    Error,
}

impl Table {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err("Start-delay table too large".into());
        }
        let table: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        table.validate()?;
        Ok(table)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        let m = &self.measurement;
        let digest = |s: &str| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit());
        if self.format_version != 1
            || self.plugin_format != "CLAP"
            || self.plugin_id != ID
            || !self.max_onset_delta_ms.is_finite()
            || self.max_onset_delta_ms < 0.0
            || !m.onset_ratio.is_finite()
            || !(0.0..=1.0).contains(&m.onset_ratio)
            || m.onset_ratio == 0.0
            || m.capture_ms != 200
            || m.sample_rate != 48000
            || m.onset_window_frames != 48
            || m.note != 48
            || m.velocity != 100
            || m.cc1 != 0
            || m.trials_per_wait != 1
            || !digest(&m.plugin_sha256)
            || !digest(&m.renderer_sha256)
            || self.entries.len() > 100_000
        {
            return Err("Unsupported start-delay table contract".into());
        }
        for (path, e) in &self.entries {
            let times = [e.candidate_onset_ms, e.reference_onset_ms];
            if !path.starts_with("patches_factory/") && !path.starts_with("patches_3rdparty/")
                || path.split('/').any(|p| p == ".." || p.is_empty())
                || path.contains(['\\', ':', '\0'])
                || (!digest(&e.fxp_sha256) && e.outcome != Outcome::Error)
                || e.state_sha256.as_ref().is_some_and(|s| !digest(s))
                || times
                    .iter()
                    .flatten()
                    .any(|t| !t.is_finite() || !(0.0..200.0).contains(t))
                || (e.outcome == Outcome::Measured
                    && (times.iter().any(Option::is_none) || e.state_sha256.is_none()))
                || (e.zero_wait_allowed
                    && (e.outcome != Outcome::Measured || e.state_sha256.is_none()))
            {
                return Err(format!("Invalid start-delay entry: {path}"));
            }
        }
        Ok(())
    }

    pub(crate) fn select(&mut self, max_delta: f64) -> Result<(), String> {
        if !max_delta.is_finite() || max_delta < 0.0 {
            return Err("Invalid onset tolerance".into());
        }
        self.max_onset_delta_ms = max_delta;
        for e in self.entries.values_mut() {
            e.zero_wait_allowed = e.outcome == Outcome::Measured
                && e.candidate_onset_ms
                    .zip(e.reference_onset_ms)
                    .is_some_and(|(a, b)| (a - b).abs() <= max_delta);
        }
        self.validate()
    }
}

pub(crate) fn local_path(config: &Path) -> std::path::PathBuf {
    config.with_file_name("patch-catalog").join(FILE)
}
