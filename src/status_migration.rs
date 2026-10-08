//! One-time extraction of old session fields, without reserializing settings.
use crate::{config::Config, status::Status};
use std::io::Write;
use std::path::Path;

const SESSION_KEYS: &[&str] = &[
    "sequence_pattern",
    "selected_sequence",
    "sequence_velocity",
    "sequence_modulation",
    "last_played",
    "effect",
    "effects",
    "effect_bypassed",
    "favorites",
];

pub(crate) fn migrate(config: &Path) -> Result<(), String> {
    let existing = Status::read(config)?;
    let original = match std::fs::read_to_string(config) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    // Validate user settings before making any changes.
    let _: Config = toml::from_str(&original).map_err(|error| error.to_string())?;
    let mut document = original
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| error.to_string())?;
    if !SESSION_KEYS.iter().any(|key| document.contains_key(key)) {
        return Ok(());
    }
    // An existing JSON file is authoritative, even when migration was interrupted.
    let imported = if existing.is_none() {
        let mut status: Status = toml::from_str(&original).map_err(|error| error.to_string())?;
        status.migrate_effects();
        Some(status)
    } else {
        None
    };
    for key in SESSION_KEYS {
        document.remove(key);
    }
    let cleaned = document.to_string();
    // Keep every original comment, including comments attached to removed status.
    // Setting comments remain in place; detached status comments go at the end.
    let mut cleaned = cleaned;
    let mut remaining_comments: Vec<&str> = cleaned
        .lines()
        .filter(|line| line.trim_start().starts_with('#'))
        .collect();
    let mut detached = Vec::new();
    for line in original
        .lines()
        .filter(|line| line.trim_start().starts_with('#'))
    {
        if let Some(index) = remaining_comments
            .iter()
            .position(|comment| *comment == line)
        {
            remaining_comments.remove(index);
        } else {
            detached.push(line);
        }
    }
    if !detached.is_empty() {
        if !cleaned.ends_with('\n') {
            cleaned.push('\n');
        }
        cleaned.push_str(&detached.join("\n"));
        cleaned.push('\n');
    }
    // Record the exact source before either replacement. Never overwrite a backup.
    let backup = config.with_extension("toml.before-status-migration.bak");
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)
    {
        Ok(mut file) => {
            if let Err(error) = file
                .write_all(original.as_bytes())
                .and_then(|()| file.sync_all())
            {
                drop(file);
                let _ = std::fs::remove_file(&backup);
                return Err(format!("{}: {error}", backup.display()));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if std::fs::read(&backup).map_err(|error| error.to_string())? != original.as_bytes() {
                return Err(format!("{} differs from config.toml; preserve both files and resolve the migration backup before retrying", backup.display()));
            }
        }
        Err(error) => return Err(format!("{}: {error}", backup.display())),
    }
    if let Some(status) = imported {
        status.write(config)?;
    }
    // Refuse to replace edits made since we read the file.
    if std::fs::read_to_string(config).map_err(|error| error.to_string())? != original {
        return Err("config.toml changed during status migration; retry on next launch".into());
    }
    crate::state_store::atomic_write(config, cleaned.as_bytes())
}
