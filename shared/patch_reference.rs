//! Restoration inputs, separate from the resulting opaque plugin state.
use crate::host::PluginKey;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchReference {
    pub plugin: PluginKey,
    pub source: Source,
    pub display: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Source {
    /// The original catalog path includes any program selector (e.g. Dexed).
    Catalog {
        path: PathBuf,
        file: PathBuf,
    },
    Effect {
        path: PathBuf,
        value: String,
    },
}

impl PatchReference {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_identity()?;
        if self.plugin.bundle_path.is_empty() {
            return Err("Invalid patch plugin identity".into());
        }
        let path = match &self.source {
            Source::Catalog { path, file } => {
                if file.as_os_str().is_empty() || file.to_str().is_none_or(|s| s.contains('\0')) {
                    return Err("Invalid patch source file".into());
                }
                path
            }
            Source::Effect { path, .. } => path,
        };
        if path.as_os_str().is_empty()
            || path
                .to_str()
                .is_none_or(|p| p.len() > 32768 || p.contains('\0'))
        {
            return Err("Invalid patch source path".into());
        }
        Ok(())
    }

    /// Everything but the paths, whose form differs between the SMF and runtime.
    pub fn validate_identity(&self) -> Result<(), String> {
        let key = &self.plugin;
        if !matches!(key.format.as_str(), "CLAP" | "VST3")
            || key.id.is_empty()
            || [
                &key.id,
                &key.name,
                &key.vendor,
                &key.bundle_path,
                &self.display,
            ]
            .iter()
            .any(|s| s.len() > 32768 || s.contains('\0'))
        {
            return Err("Invalid patch plugin identity".into());
        }
        if matches!(&self.source, Source::Effect { value, .. }
            if value.len() > 32768 || value.contains('\0'))
        {
            return Err("Invalid effect preset selector".into());
        }
        Ok(())
    }
}
