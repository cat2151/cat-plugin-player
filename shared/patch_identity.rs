//! Stable patch names and source identity, independent of list order.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The catalog patch the restored instrument state came from.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PatchIdentity {
    pub format: String,
    pub plugin_id: String,
    pub path: PathBuf,
    pub display: String,
}
