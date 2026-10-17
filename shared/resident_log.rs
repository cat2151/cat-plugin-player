//! Trace of the resident link, one file per side under `%TEMP%\cat-resident-log`.
//!
//! Lines start with milliseconds since the Unix epoch, so the editor's and the
//! player's files can be merged by sorting.
use std::io::Write;
use std::path::PathBuf;

pub fn path(side: &str) -> PathBuf {
    std::env::temp_dir()
        .join("cat-resident-log")
        .join(format!("{side}.log"))
}

/// Empties this side's file.
pub fn start(side: &str) {
    let path = path(side);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, "");
}

pub fn log(side: &str, message: impl std::fmt::Display) {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path(side))
    {
        let _ = writeln!(file, "{millis} {side} {message}");
    }
}
