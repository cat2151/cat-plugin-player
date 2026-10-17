//! Public SMF references carry root-relative `/` paths; each machine supplies the roots
//! (cat-smf-editor ADR 0008). Runtime references stay absolute.
use crate::{
    host::PluginKey,
    patch_reference::{PatchReference, Source},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Where one machine installed each plugin and keeps its patches or presets.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub plugins: Vec<Plugin>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    pub format: String,
    pub id: String,
    pub bundle_path: String,
    /// A root may be a file: presets built into the plugin are the plugin itself.
    pub roots: Vec<String>,
}

impl Table {
    pub fn plugin(&self, key: &PluginKey) -> Result<Plugin, String> {
        self.plugins
            .iter()
            .find(|p| p.format == key.format && p.id == key.id)
            .cloned()
            .ok_or_else(|| format!("{}: plugin is not installed", key.name))
    }
}

pub trait Roots {
    /// `refresh` rediscovers the roots; it is asked only after a miss.
    fn plugin(&mut self, key: &PluginKey, refresh: bool) -> Result<Plugin, String>;
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

impl Roots for Table {
    fn plugin(&mut self, key: &PluginKey, _: bool) -> Result<Plugin, String> {
        Table::plugin(self, key)
    }
}

/// Windows paths compare without `\\?\`, separators or case.
fn components(path: &str) -> Vec<String> {
    let path = path.trim();
    let path = path
        .strip_prefix(r"\\?\")
        .or_else(|| path.strip_prefix("//?/"))
        .unwrap_or(path);
    path.split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != ".")
        .map(str::to_owned)
        .collect()
}

fn same(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}

/// `path` relative to `root` with `/` separators; empty when `path` is the root itself.
pub fn relative(path: &Path, root: &str) -> Option<String> {
    let path = components(path.to_str()?);
    let root = components(root);
    if root.is_empty()
        || path.len() < root.len()
        || !root.iter().zip(&path).all(|(r, p)| same(r, p))
    {
        return None;
    }
    let rest = &path[root.len()..];
    if rest.iter().any(|part| part == "..") {
        return None;
    }
    Some(rest.join("/"))
}

pub fn join(root: &str, relative: &str) -> PathBuf {
    let mut path = PathBuf::from(root);
    path.extend(relative.split('/').filter(|part| !part.is_empty()));
    path
}

fn check_relative(path: &Path, allow_root: bool) -> Result<&str, String> {
    let text = path.to_str().ok_or("Patch path is not UTF-8")?;
    let invalid = (text.is_empty() && !allow_root)
        || text.len() > 32768
        || text.starts_with('/')
        || text.ends_with('/')
        || text.contains(['\\', ':', '\0'])
        || (!text.is_empty()
            && text
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == ".."));
    if invalid {
        return Err(format!("Patch path must be relative to its root: {text}"));
    }
    Ok(text)
}

/// The SMF form: no bundle path, and source paths relative to a root.
pub fn validate_portable(reference: &PatchReference) -> Result<(), String> {
    reference.validate_identity()?;
    if !reference.plugin.bundle_path.is_empty() {
        return Err("SMF patch references must not name the plugin location".into());
    }
    match &reference.source {
        Source::Catalog { path, file } => {
            check_relative(path, false)?;
            check_relative(file, false)?;
        }
        Source::Effect { path, .. } => {
            check_relative(path, true)?;
        }
    }
    Ok(())
}

/// The one root holding `relative`. Several matches are refused, not guessed between.
fn locate<'a>(plugin: &'a Plugin, relative: &str, roots: &dyn Roots) -> Result<&'a str, String> {
    let mut found = plugin
        .roots
        .iter()
        .filter(|root| roots.exists(&join(root, relative)));
    match (found.next(), found.next()) {
        (Some(root), None) => Ok(root),
        (None, _) => Err(format!("{relative}: not found under the patch roots")),
        (Some(_), Some(_)) => Err(format!("{relative}: found under several patch roots")),
    }
}

fn source_file(source: &Source) -> &Path {
    match source {
        Source::Catalog { file, .. } => file,
        Source::Effect { path, .. } => path,
    }
}

fn to_portable(
    reference: &PatchReference,
    plugin: &Plugin,
    roots: &dyn Roots,
) -> Result<PatchReference, String> {
    let file = source_file(&reference.source);
    let (root, relative_file) = plugin
        .roots
        .iter()
        .find_map(|root| Some((root, relative(file, root)?)))
        .ok_or_else(|| format!("{}: outside the patch roots", file.display()))?;
    if locate(plugin, &relative_file, roots)? != root {
        return Err(format!("{relative_file}: found under several patch roots"));
    }
    let source = match &reference.source {
        Source::Catalog { path, .. } => Source::Catalog {
            path: relative(path, root)
                .ok_or("Patch selector is outside its file's root")?
                .into(),
            file: relative_file.into(),
        },
        Source::Effect { value, .. } => Source::Effect {
            path: relative_file.into(),
            value: value.clone(),
        },
    };
    let portable = PatchReference {
        plugin: PluginKey {
            bundle_path: String::new(),
            ..reference.plugin.clone()
        },
        source,
        display: reference.display.clone(),
    };
    validate_portable(&portable)?;
    Ok(portable)
}

fn to_absolute(
    reference: &PatchReference,
    plugin: &Plugin,
    roots: &dyn Roots,
) -> Result<PatchReference, String> {
    let text = |path: &Path| path.to_str().unwrap_or_default().to_owned();
    let root = locate(plugin, &text(source_file(&reference.source)), roots)?;
    let source = match &reference.source {
        Source::Catalog { path, file } => Source::Catalog {
            path: join(root, &text(path)),
            file: join(root, &text(file)),
        },
        Source::Effect { path, value } => Source::Effect {
            path: join(root, &text(path)),
            value: value.clone(),
        },
    };
    let absolute = PatchReference {
        plugin: PluginKey {
            bundle_path: plugin.bundle_path.clone(),
            ..reference.plugin.clone()
        },
        source,
        display: reference.display.clone(),
    };
    absolute.validate()?;
    Ok(absolute)
}

fn with_refresh(
    key: &PluginKey,
    roots: &mut dyn Roots,
    convert: impl Fn(&Plugin, &dyn Roots) -> Result<PatchReference, String>,
) -> Result<PatchReference, String> {
    roots
        .plugin(key, false)
        .and_then(|plugin| convert(&plugin, roots))
        .or_else(|_| convert(&roots.plugin(key, true)?, roots))
        .map_err(|error| format!("{}: {error}", key.name))
}

/// The SMF form of a runtime reference.
pub fn portable(
    reference: &PatchReference,
    roots: &mut dyn Roots,
) -> Result<PatchReference, String> {
    reference.validate()?;
    with_refresh(&reference.plugin, roots, |plugin, roots| {
        to_portable(reference, plugin, roots)
    })
}

/// The runtime form of an SMF reference on this machine.
pub fn resolve(
    reference: &PatchReference,
    roots: &mut dyn Roots,
) -> Result<PatchReference, String> {
    validate_portable(reference)?;
    with_refresh(&reference.plugin, roots, |plugin, roots| {
        to_absolute(reference, plugin, roots)
    })
}

/// Roots whose files exist only in `files`, for tests without a filesystem.
#[cfg(test)]
pub(crate) struct Fixture {
    pub table: Table,
    pub files: Vec<String>,
    pub refreshed: usize,
}

#[cfg(test)]
impl Roots for Fixture {
    fn plugin(&mut self, key: &PluginKey, refresh: bool) -> Result<Plugin, String> {
        self.refreshed += usize::from(refresh);
        self.table.plugin(key)
    }
    fn exists(&self, path: &Path) -> bool {
        self.files
            .iter()
            .any(|file| relative(path, file).is_some_and(|rest| rest.is_empty()))
    }
}

#[cfg(test)]
#[path = "patch_roots_tests.rs"]
mod tests;
