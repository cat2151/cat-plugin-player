//! Cat-owned user conditions. A failed read makes the original file read-only.
use std::path::{Path, PathBuf};

pub(crate) struct Store {
    pub presets: Vec<(String, String)>,
    pub error: Option<String>,
    path: Result<PathBuf, String>,
    writable: bool,
}

impl Store {
    pub fn load(config: &Result<PathBuf, String>) -> Self {
        let path = config.as_ref().map_err(Clone::clone).and_then(|path| {
            path.parent()
                .map(|parent| parent.join("patch-filter-presets.json"))
                .ok_or_else(|| "config path has no parent".into())
        });
        let result = path
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|path| read(path));
        match result {
            Ok(presets) => Self {
                presets,
                error: None,
                path,
                writable: true,
            },
            Err(error) => Self {
                presets: Vec::new(),
                error: Some(error),
                path,
                writable: false,
            },
        }
    }

    pub fn writable(&self) -> bool {
        self.writable
    }

    pub fn replace(&mut self, presets: Vec<(String, String)>) -> bool {
        let result = (|| {
            if !self.writable {
                return Err(
                    "Conditions were not read successfully; original file is protected.".into(),
                );
            }
            let presets = cmrt_patch_select::prepare_user_presets(presets);
            let bytes = serde_json::to_vec_pretty(&presets).map_err(|error| error.to_string())?;
            crate::state_store::atomic_write(self.path.as_ref().map_err(Clone::clone)?, &bytes)?;
            self.presets = presets;
            Ok::<_, String>(())
        })();
        self.error = result.as_ref().err().cloned();
        result.is_ok()
    }
}

fn read(path: &Path) -> Result<Vec<(String, String)>, String> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let presets = serde_json::from_slice(&bytes)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            Ok(cmrt_patch_select::prepare_user_presets(presets))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

#[cfg(test)]
mod tests;
