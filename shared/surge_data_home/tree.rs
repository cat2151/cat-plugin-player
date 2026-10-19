use super::{MARKER, OMITTED};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const FACTORY: &str = "patches_factory";
const THIRD: &str = "patches_3rdparty";
static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) fn templates(root: &Path) -> Result<(), String> {
    let path = root.join(FACTORY).join("Templates");
    let mut entries = fs::read_dir(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !entries.any(|e| {
        e.ok().is_some_and(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("fxp"))
        })
    }) {
        return Err(
            "Surge Templates must contain initial patches; leaving data home unchanged".into(),
        );
    }
    Ok(())
}

pub(super) fn shared_entries(source: &Path) -> Result<Vec<OsString>, String> {
    let mut names = names(source)?;
    names.retain(|name| name != FACTORY && name != THIRD);
    if names.iter().any(|n| n == MARKER) {
        return Err("Unexpected Surge source marker".into());
    }
    Ok(names)
}

fn names(root: &Path) -> Result<Vec<OsString>, String> {
    let mut names = fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .map(|e| e.map(|e| e.file_name()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    names.sort();
    Ok(names)
}

pub(super) fn validate(
    source: &Path,
    minimal: &Path,
    marker: &str,
    minimal_resources: bool,
) -> Result<(), String> {
    templates(minimal)?;
    let shared = shared_entries(source)?;
    let mut expected = shared.clone();
    expected.extend([FACTORY.into(), THIRD.into(), marker.into()]);
    expected.sort();
    let identity = |p: &Path| fs::canonicalize(p).map_err(|e| e.to_string());
    if names(minimal)? != expected
        || names(&minimal.join(FACTORY))? != [OsString::from("Templates")]
        || !names(&minimal.join(THIRD))?.is_empty()
        || identity(&minimal.join(FACTORY).join("Templates"))?
            != identity(&source.join(FACTORY).join("Templates"))?
    {
        return Err(format!(
            "Invalid minimal Surge data tree: {}",
            minimal.display()
        ));
    }
    let recorded = fs::read_to_string(minimal.join(marker)).map_err(|e| e.to_string())?;
    if identity(Path::new(recorded.trim()))? != identity(source)? {
        return Err("Surge data source changed".into());
    }
    for name in shared {
        let original = source.join(&name);
        let copy = minimal.join(&name);
        let matches = if minimal_resources && OMITTED.iter().any(|n| name == *n) {
            copy.is_dir() && names(&copy)?.is_empty()
        } else if original.is_dir() {
            identity(&copy)? == identity(&original)?
        } else {
            fs::read(copy).map_err(|e| e.to_string())?
                == fs::read(original).map_err(|e| e.to_string())?
        };
        if !matches {
            return Err("Surge sound resource changed".into());
        }
    }
    Ok(())
}

/// Track only newly created entries, so failure cleanup never traverses a junction.
#[derive(Default)]
struct Staging(Vec<(PathBuf, bool)>);
impl Staging {
    fn directory(&mut self, path: PathBuf) -> Result<(), String> {
        fs::create_dir(&path).map_err(|e| e.to_string())?;
        self.0.push((path, true));
        Ok(())
    }
    fn link(&mut self, path: PathBuf, target: &Path) -> Result<(), String> {
        link(&path, target)?;
        self.0.push((path, cfg!(windows)));
        Ok(())
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        for (path, directory) in self.0.iter().rev() {
            if *directory {
                let _ = fs::remove_dir(path);
            } else {
                let _ = fs::remove_file(path);
            }
        }
    }
}

pub(super) fn build(
    source: &Path,
    root: &Path,
    destination: &Path,
    entries: &[OsString],
) -> Result<(), String> {
    let stage = root.join(format!(
        ".{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut created = Staging::default();
    created.directory(stage.clone())?;
    for name in entries {
        let original = source.join(name);
        let target = stage.join(name);
        if OMITTED.iter().any(|n| name.as_os_str() == *n) {
            created.directory(target)?;
        } else if original.is_dir() {
            created.link(target, &original)?;
        } else {
            fs::copy(original, &target).map_err(|e| e.to_string())?;
            created.0.push((target, false));
        }
    }
    created.directory(stage.join(FACTORY))?;
    created.link(
        stage.join(FACTORY).join("Templates"),
        &source.join(FACTORY).join("Templates"),
    )?;
    created.directory(stage.join(THIRD))?;
    let marker = stage.join(MARKER);
    fs::write(&marker, source.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
    created.0.push((marker, false));
    validate(source, &stage, MARKER, true)?;
    match fs::rename(&stage, destination) {
        Ok(()) => created.0.clear(),
        Err(error) => {
            validate(source, destination, MARKER, true).map_err(|_| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn link(path: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // Env expansion occurs inside quotes, without re-expanding '%' in the values.
    // /V:OFF also preserves literal '!'. No path is interpolated into shell code.
    let output = std::process::Command::new("cmd")
        .args(["/d", "/v:off", "/c"])
        .raw_arg("mklink /J \"%CAT_SMF_SURGE_LINK%\" \"%CAT_SMF_SURGE_TARGET%\"")
        .env("CAT_SMF_SURGE_LINK", path)
        .env("CAT_SMF_SURGE_TARGET", target)
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Cannot link Surge data: {}",
            String::from_utf8_lossy(&output.stdout)
        ))
    }
}

#[cfg(not(windows))]
fn link(path: &Path, target: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, path).map_err(|e| e.to_string())
}
