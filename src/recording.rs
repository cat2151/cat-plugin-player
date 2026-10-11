//! Fresh application-owned data per recording launch (ADR 0026).
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static CONFIG: OnceLock<Result<PathBuf, String>> = OnceLock::new();

pub(crate) fn config_path() -> Result<PathBuf, String> {
    CONFIG
        .get_or_init(|| {
            let root = dirs::config_local_dir()
                .ok_or("could not locate the local config directory")?
                .join("cat-plugin-player-recordings");
            let config = create_session(&root)?;
            eprintln!("Recording session: {}", config.parent().unwrap().display());
            Ok(config)
        })
        .clone()
}

fn create_session(root: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    // create_dir, rather than create_dir_all, refuses to reuse an old session.
    for attempt in 0..100 {
        let directory = root.join(format!("{time}-{}-{attempt}", std::process::id()));
        match std::fs::create_dir(&directory) {
            Ok(()) => return Ok(directory.join("config.toml")),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("could not create a fresh recording session".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launches_get_empty_sibling_sessions_without_reusing_previous_data() {
        let root = std::env::temp_dir().join(format!(
            "cat-recording-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = create_session(&root).unwrap();
        std::fs::write(&first, "old settings").unwrap();
        std::fs::write(first.parent().unwrap().join("surge-xt.json"), "old catalog").unwrap();
        let second = create_session(&root).unwrap();
        assert_ne!(first, second);
        assert_eq!(second.parent().unwrap().parent(), Some(root.as_path()));
        assert_eq!(
            std::fs::read_dir(second.parent().unwrap()).unwrap().count(),
            0
        );
        assert_eq!(std::fs::read_to_string(first).unwrap(), "old settings");
        std::fs::remove_dir_all(root).unwrap();
    }
}
