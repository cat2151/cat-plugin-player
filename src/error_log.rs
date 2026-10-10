//! Append failure status lines to `error.log` beside config.toml.
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const FILE_NAME: &str = "error.log";

/// A patch named like "Error Bass" can be logged too; extra lines are harmless.
pub(crate) fn is_error(status: &str) -> bool {
    let status = status.to_ascii_lowercase();
    [
        "failed",
        "could not",
        "error",
        "not found",
        "no audio",
        "suppressed",
        "unavailable",
    ]
    .iter()
    .any(|term| status.contains(term))
}

pub(crate) fn append(config: &Path, message: &str) -> Result<(), String> {
    let path = config.with_file_name(FILE_NAME);
    let line = format!("{} {message}\n", utc_now());
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| file.write_all(line.as_bytes()))
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn utc_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let (days, time) = (seconds / 86_400, seconds % 86_400);
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time / 3600,
        time / 60 % 60,
        time % 60
    )
}

/// Howard Hinnant's days-to-civil conversion for the proleptic Gregorian calendar.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(20_736), (2026, 10, 10));
    }

    #[test]
    fn appends_failures_beside_config() {
        let dir = std::env::temp_dir().join(format!("cpp-error-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let config = dir.join("config.toml");
        let _ = std::fs::remove_file(dir.join(FILE_NAME));
        assert!(is_error("Browse patch failed: malformed"));
        assert!(!is_error("Loading Surge XT..."));
        append(&config, "first").unwrap();
        append(&config, "second").unwrap();
        let log = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        let lines: Vec<_> = log.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("Z first") && lines[1].ends_with("Z second"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
