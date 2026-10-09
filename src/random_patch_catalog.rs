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
}

type LoadResult = Result<Vec<Candidate>, String>;

#[derive(Default)]
enum State {
    #[default]
    NotStarted,
    Loading(Receiver<LoadResult>),
    Ready(Vec<Candidate>),
    Unavailable,
}

#[derive(Default)]
pub(crate) struct Catalog {
    state: State,
    eligible: Vec<usize>,
}

impl Catalog {
    #[cfg(test)]
    pub(crate) fn from_candidates(candidates: Vec<Candidate>, plugins: &[PluginInfo]) -> Self {
        let mut catalog = Self {
            state: State::Ready(candidates),
            eligible: Vec::new(),
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
                self.state = State::Unavailable;
                Some(format!("Random patch unavailable: {error}"))
            }
        }
    }

    /// Call when the scan list changes, and when a worker result arrives.
    pub fn reconcile(&mut self, plugins: &[PluginInfo]) {
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
                            })
                            .then_some(index)
                    }),
            );
        }
    }

    pub fn button(&self, ui: &mut egui::Ui, busy: bool) -> bool {
        self.button_response(ui, busy)
            .is_some_and(|response| response.clicked())
    }

    pub fn candidates(&self) -> impl Iterator<Item = &Candidate> {
        let candidates = match &self.state {
            State::Ready(candidates) => candidates.as_slice(),
            _ => &[],
        };
        self.eligible.iter().map(move |&index| &candidates[index])
    }

    fn button_response(&self, ui: &mut egui::Ui, busy: bool) -> Option<egui::Response> {
        if self.eligible.is_empty() {
            return None;
        }
        let sample = self.candidates().next()?;
        Some(
            ui.add_enabled(!busy, egui::Button::new("Random patch"))
                .on_hover_text(format!(
                    "Choose from {} {} patches and play the selected sequence.\nExample: {}\n{}",
                    self.eligible.len(),
                    sample.plugin_name,
                    sample.display,
                    sample.path.display()
                )),
        )
    }
}

fn load() -> LoadResult {
    let (snapshot, _) = clap_mml_render_tui::patch_catalog_cache::load()
        .map_err(|error| format!("{error:#}"))?
        .into_parts();
    let mut candidates = Vec::new();
    for patch in snapshot.audio_patches() {
        let plugin = snapshot
            .patch_plugins()
            .audio_info_for_ref(&patch.reference)
            .map_err(|error| error.to_string())?;
        if plugin.plugin_id.as_deref() != Some(SURGE_ID)
            || !std::path::Path::new(&plugin.plugin_path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("clap"))
        {
            continue;
        }
        candidates.push(Candidate {
            format: "CLAP".into(),
            plugin_id: SURGE_ID.into(),
            plugin_name: plugin.name.clone(),
            display: patch.reference.display.clone(),
            path: plugin.patch_base.resolve(&patch.reference.display).into(),
        });
    }
    Ok(candidates)
}

#[cfg(test)]
mod tests;
