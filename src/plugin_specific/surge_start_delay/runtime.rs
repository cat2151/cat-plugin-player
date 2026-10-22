use super::*;
use crate::{config::PluginKey, patch_preparation::Prepared};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::{self, Receiver, TryRecvError},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Permit {
    pub plugin_format: String,
    pub plugin_id: String,
    pub patch: String,
    pub fxp_sha256: String,
    pub state_sha256: String,
    pub dataset_sha256: String,
    pub wait_ms: u32,
}

#[derive(Default)]
pub(crate) struct Runtime {
    table: Option<(String, Table)>,
    live: HashMap<i32, Permit>,
    exposed: HashSet<i32>,
    saved: Option<Permit>,
    refresh: Option<Receiver<Result<(String, Table), String>>>,
    started: bool,
    changed: bool,
}

pub(super) fn refresh_with(
    fetch: impl FnOnce() -> Result<Vec<u8>, String>,
    destination: &Path,
) -> Result<(String, Table), String> {
    let bytes = fetch()?;
    let table = Table::parse(&bytes)?;
    crate::state_store::atomic_write(destination, &bytes)?;
    Ok((hash(&bytes), table))
}

impl Runtime {
    pub(crate) fn load(config: &Path, mut saved: Option<Permit>) -> Self {
        let table = std::fs::read(local_path(config))
            .ok()
            .and_then(|b| Table::parse(&b).ok().map(|t| (hash(&b), t)));
        if let (Some((revision, table)), Some(permit)) = (&table, &mut saved) {
            if table
                .entries
                .get(&permit.patch)
                .is_some_and(|e| e.zero_wait_allowed && e.fxp_sha256 == permit.fxp_sha256)
            {
                permit.dataset_sha256 = revision.clone();
            }
        }
        Self {
            table,
            saved,
            ..Self::default()
        }
    }

    fn accepted(&self, p: &Permit) -> bool {
        p.plugin_format == "CLAP"
            && p.plugin_id == ID
            && p.wait_ms == 0
            && self.table.as_ref().is_some_and(|(revision, table)| {
                revision == &p.dataset_sha256
                    && table
                        .entries
                        .get(&p.patch)
                        .is_some_and(|e| e.zero_wait_allowed && e.fxp_sha256 == p.fxp_sha256)
            })
    }

    pub(crate) fn snapshot(&self, id: i32) -> Option<Permit> {
        self.live.get(&id).filter(|p| self.accepted(p)).cloned()
    }

    pub(crate) fn replace(&mut self, id: i32, permit: Option<Permit>) -> Option<Permit> {
        let old = self.live.remove(&id);
        if let Some(p) = permit {
            self.live.insert(id, p);
        }
        old
    }

    pub(crate) fn authorize(&self, id: i32, prepared: &Prepared) -> Option<Permit> {
        if self.exposed.contains(&id)
            || prepared.candidate.format != "CLAP"
            || prepared.candidate.plugin_id != ID
        {
            return None;
        }
        let (revision, table) = self.table.as_ref()?;
        let e = table.entries.get(&prepared.candidate.display)?;
        let state_sha256 = hash(&prepared.state);
        if !e.zero_wait_allowed || e.state_sha256.as_deref() != Some(&state_sha256) {
            return None;
        }
        Some(Permit {
            plugin_format: "CLAP".into(),
            plugin_id: ID.into(),
            patch: prepared.candidate.display.clone(),
            fxp_sha256: e.fxp_sha256.clone(),
            state_sha256,
            dataset_sha256: revision.clone(),
            wait_ms: 0,
        })
    }

    pub(crate) fn restore(&mut self, id: i32, plugin: &PluginKey, state: &[u8]) {
        self.live.remove(&id);
        if plugin.format != "CLAP" || plugin.id != ID {
            return;
        }
        if let Some(p) = self
            .saved
            .as_ref()
            .filter(|p| self.accepted(p) && p.state_sha256 == hash(state))
            .cloned()
        {
            self.live.insert(id, p);
        }
    }

    pub(crate) fn saved_state(&mut self, id: i32, state: &[u8]) {
        if let Some(p) = self.live.get_mut(&id) {
            p.state_sha256 = hash(state);
            self.saved = Some(p.clone());
        }
    }

    pub(crate) fn expose_editor(&mut self, id: i32) {
        self.live.remove(&id);
        self.exposed.insert(id);
        self.saved = None;
    }

    pub(crate) fn editor_closed(&mut self, id: i32) {
        self.exposed.remove(&id);
    }

    pub(crate) fn forget(&mut self, id: i32) {
        self.live.remove(&id);
        self.exposed.remove(&id);
    }

    pub(crate) fn delay(
        &self,
        id: i32,
        plugin: &PluginKey,
        fallback: std::time::Duration,
    ) -> std::time::Duration {
        if plugin.format == "CLAP" && plugin.id == ID && self.snapshot(id).is_some() {
            std::time::Duration::ZERO
        } else {
            fallback
        }
    }

    pub(crate) fn poll(&mut self, config: &Path, ctx: &eframe::egui::Context) -> Option<String> {
        if !self.started {
            self.started = true;
            let destination = local_path(config);
            let (tx, rx) = mpsc::channel();
            self.refresh = Some(rx);
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let result = refresh_with(|| crate::surge_patch_catalog::fetch(URL), &destination);
                let _ = tx.send(result);
                ctx.request_repaint();
            });
        }
        let result = match self.refresh.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                Err("Start-delay refresh worker disconnected".into())
            }
        };
        self.refresh = None;
        match result {
            Ok(table) => {
                // Do not restart audio. A still-authorized patch may keep its new provenance.
                for p in self.live.values_mut() {
                    if table
                        .1
                        .entries
                        .get(&p.patch)
                        .is_some_and(|e| e.zero_wait_allowed && e.fxp_sha256 == p.fxp_sha256)
                    {
                        p.dataset_sha256 = table.0.clone();
                    }
                }
                self.table = Some(table);
                self.changed = true;
                None
            }
            Err(error) => Some(format!(
                "Surge start-delay table refresh failed; keeping local data: {error}"
            )),
        }
    }

    pub(crate) fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }
}
