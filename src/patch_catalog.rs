//! One-shot, audio-first catalog loading. No native host crosses the worker boundary.
use crate::ffi::PluginInfo;
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub(crate) const SURGE_ID: &str = "org.surge-synth-team.surge-xt";

pub(crate) fn busy(scanning: bool, restoring: bool, generating: bool, preparing: bool) -> bool {
    scanning || restoring || generating || preparing
}

#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub format: String,
    pub plugin_id: String,
    pub plugin_name: String,
    pub display: String,
    pub path: PathBuf,
    pub bundle_path: PathBuf,
    pub entry: cmrt_patch_select::PatchCatalogEntry,
    pub measurement: cmrt_tui_core::patch_load::PatchLoadMeasurement,
}

type LoadResult = Result<Vec<Candidate>, String>;

#[derive(Default)]
enum State {
    #[default]
    NotStarted,
    Loading(Receiver<LoadResult>),
    Ready(Vec<Candidate>),
    Unavailable(String),
}

#[derive(Default)]
pub(crate) struct Catalog {
    state: State,
    eligible: Vec<usize>,
    pub generation: u64,
}

impl Catalog {
    #[cfg(test)]
    pub(crate) fn from_candidates(candidates: Vec<Candidate>, plugins: &[PluginInfo]) -> Self {
        let mut catalog = Self {
            state: State::Ready(candidates),
            eligible: Vec::new(),
            generation: 0,
        };
        catalog.reconcile(plugins);
        catalog
    }
    pub fn update(&mut self, rendered: bool, ctx: &egui::Context) -> Option<String> {
        self.update_with(rendered, load, {
            let ctx = ctx.clone();
            move || ctx.request_repaint()
        })
    }

    fn update_with(
        &mut self,
        rendered: bool,
        loader: impl FnOnce() -> LoadResult + Send + 'static,
        repaint: impl FnOnce() + Send + 'static,
    ) -> Option<String> {
        if rendered && matches!(self.state, State::NotStarted) {
            let (tx, rx) = mpsc::channel();
            self.state = State::Loading(rx);
            std::thread::spawn(move || {
                let _ = tx.send(loader());
                repaint();
            });
        }
        let State::Loading(rx) = &self.state else {
            return None;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("catalog worker disconnected".into()),
        };
        match result {
            Ok(candidates) => {
                self.state = State::Ready(candidates);
                Some(String::new())
            }
            Err(error) => {
                self.state = State::Unavailable(error.clone());
                Some(format!("Patch catalog unavailable: {error}"))
            }
        }
    }

    /// Call when the scan list changes, and when a worker result arrives.
    pub fn reconcile(&mut self, plugins: &[PluginInfo]) {
        self.generation += 1;
        self.eligible.clear();
        if let State::Ready(candidates) = &self.state {
            self.eligible.extend(
                candidates
                    .iter()
                    .enumerate()
                    .filter_map(|(index, candidate)| {
                        plugins
                            .iter()
                            .any(|plugin| {
                                plugin.format == candidate.format
                                    && plugin.id == candidate.plugin_id
                                    && plugin.kind == crate::plugin_list::PluginKind::Instrument
                            })
                            .then_some(index)
                    }),
            );
        }
    }

    pub fn candidates(&self) -> impl Iterator<Item = &Candidate> {
        let candidates = match &self.state {
            State::Ready(candidates) => candidates.as_slice(),
            _ => &[],
        };
        self.eligible.iter().map(move |&index| &candidates[index])
    }

    pub fn notice(&self) -> Option<&str> {
        match &self.state {
            State::NotStarted => Some("Catalog will load after the first audio callback."),
            State::Loading(_) => Some("Loading catalog..."),
            State::Unavailable(error) => Some(error),
            State::Ready(_) => None,
        }
    }
}

/// cmrt's own cache when it exists; otherwise the Surge XT catalog (ADR 0025).
fn loaded() -> Result<cmrt_tui_core::patch_catalog_cache::LoadedPatchCatalogCache, String> {
    use cmrt_tui_core::patch_catalog_cache;
    let cmrt_cache_exists = !cfg!(feature = "recording-mode")
        && patch_catalog_cache::cache_file_path().is_some_and(|path| path.exists());
    let loaded = if cmrt_cache_exists {
        patch_catalog_cache::load()
    } else {
        patch_catalog_cache::load_from(&crate::surge_patch_catalog::path()?)
    };
    loaded.map_err(|error| format!("{error:#}"))
}

pub(crate) fn load() -> LoadResult {
    let mut candidates = candidates(loaded()?)?;
    crate::plugin_specific::apply_measured_patch_categories(&mut candidates);
    Ok(candidates)
}

/// The directories each plugin's candidate paths are built from, by plugin ID.
pub(crate) fn roots() -> Result<Vec<(String, Vec<String>)>, String> {
    let (snapshot, _) = loaded()?.into_parts();
    Ok(snapshot
        .patch_plugins()
        .plugins()
        .iter()
        .filter_map(|plugin| {
            let roots = match &plugin.base {
                cmrt_core::PatchBase::Shared(base) => vec![base.clone()],
                cmrt_core::PatchBase::PerRoot(roots) => roots.clone(),
                cmrt_core::PatchBase::None => plugin.dirs.clone(),
            };
            Some((plugin.plugin_id.clone()?, roots))
        })
        .collect())
}

pub(crate) fn candidates(
    loaded: cmrt_tui_core::patch_catalog_cache::LoadedPatchCatalogCache,
) -> LoadResult {
    let (snapshot, _) = loaded.into_parts();
    let mut candidates = Vec::new();
    for patch in snapshot.audio_patches() {
        let plugin = snapshot
            .patch_plugins()
            .audio_info_for_ref(&patch.reference)
            .map_err(|error| error.to_string())?;
        let Some(id) = plugin.plugin_id.as_deref() else {
            continue;
        };
        if !cmrt_patches::supports_catalog_clap_plugin(id)
            || !std::path::Path::new(&plugin.plugin_path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("clap"))
        {
            continue;
        }
        candidates.push(Candidate {
            format: "CLAP".into(),
            plugin_id: id.into(),
            plugin_name: plugin.name.clone(),
            display: patch.reference.display.clone(),
            path: plugin.patch_base.resolve(&patch.reference.display).into(),
            bundle_path: plugin.plugin_path.clone().into(),
            entry: cmrt_patch_select::PatchCatalogEntry::new(
                patch.reference.display.clone(),
                patch.normalized_display.clone(),
                plugin.name.clone(),
                patch.selector_category.clone(),
            )
            .with_merged(
                patch.merged.as_ref().map_or(1, |merged| merged.count),
                patch
                    .merged
                    .as_ref()
                    .map_or_else(Vec::new, |merged| merged.names.clone()),
            ),
            measurement: snapshot
                .load_measurements()
                .get(&patch.reference.display)
                .cloned()
                .unwrap_or_default(),
        });
    }
    Ok(candidates)
}

#[cfg(test)]
mod tests;
