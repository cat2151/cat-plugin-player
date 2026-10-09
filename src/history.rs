//! History timestamps and compatibility with automatic favorite metadata.
use crate::favorites_store::Library;
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn age(registered: u64, now: u64) -> String {
    if registered == 0 {
        return "?".into();
    }
    let seconds = now.saturating_sub(registered);
    for (unit, label) in [
        (31_536_000, "y"),
        (2_592_000, "mon"),
        (604_800, "w"),
        (86_400, "d"),
        (3_600, "h"),
        (60, "m"),
        (1, "s"),
    ] {
        if seconds >= unit {
            return format!("{}{label}", seconds / unit);
        }
    }
    "1s".into()
}

pub(crate) fn migrate(
    library: &mut Library,
    metadata: &toml::Value,
    config: &Path,
) -> Result<bool, String> {
    let Some(entries) = metadata.get("entries").and_then(toml::Value::as_array) else {
        return Ok(false);
    };
    let mut changed = false;
    for entry in &mut library.entries {
        let Some(old) = entries
            .iter()
            .find(|old| old.get("id").and_then(toml::Value::as_str) == Some(&entry.id))
        else {
            continue;
        };
        if old.get("history").is_some() {
            continue;
        }
        // The old index had no origin flag. Include every legacy snapshot in history
        // so renamed automatic saves are kept; preserve possible manual favorites too.
        entry.history = true;
        entry.favorite = !entry.name.starts_with("auto ");
        if entry.history {
            entry.registered_at = entry
                .id
                .split('-')
                .next()
                .and_then(|id| u128::from_str_radix(id, 16).ok())
                .and_then(|nanos| u64::try_from(nanos / 1_000_000_000).ok())
                .filter(|seconds| *seconds > 0 && *seconds <= now())
                .or_else(|| {
                    config.parent().and_then(|parent| {
                        std::fs::metadata(
                            parent.join("favorites").join(format!("{}.bin", entry.id)),
                        )
                        .ok()?
                        .modified()
                        .ok()?
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .map(|time| time.as_secs())
                    })
                })
                .unwrap_or(0);
        }
        changed = true;
    }
    Ok(changed)
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
