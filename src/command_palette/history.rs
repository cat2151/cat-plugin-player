//! Query-specific choices, then global recency, matching editor selection_history.
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct History {
    queries: BTreeMap<String, Vec<String>>,
    recent: Vec<String>,
    #[serde(skip)]
    unreadable: bool,
}

impl History {
    pub fn record(&mut self, query: &str, id: &str) {
        for list in [
            self.queries.entry(query.into()).or_default(),
            &mut self.recent,
        ] {
            list.retain(|previous| previous != id);
            list.insert(0, id.into());
        }
    }

    pub fn rank(&self, query: &str, id: &str) -> (u8, usize) {
        if let Some(position) = self
            .queries
            .get(query)
            .and_then(|list| list.iter().position(|s| s == id))
        {
            return (0, position);
        }
        self.recent
            .iter()
            .position(|s| s == id)
            .map_or((2, 0), |p| (1, p))
    }
}

#[derive(Clone, Default)]
struct Store {
    path: Option<PathBuf>,
    error: Option<String>,
}

fn id() -> egui::Id {
    egui::Id::new("command-palette-history")
}

pub(super) fn configure(ctx: &egui::Context, config: Option<&std::path::Path>) {
    let mut store = ctx.data(|d| d.get_temp::<Store>(id())).unwrap_or_default();
    store.path = config.map(|p| p.with_file_name("command-palette-history.json"));
    ctx.data_mut(|d| d.insert_temp(id(), store));
}

fn error(ctx: &egui::Context, message: String) {
    let mut store = ctx.data(|d| d.get_temp::<Store>(id())).unwrap_or_default();
    store.error = Some(message);
    ctx.data_mut(|d| d.insert_temp(id(), store));
}

pub(super) fn take_error(ctx: &egui::Context) -> Option<String> {
    let mut store = ctx.data(|d| d.get_temp::<Store>(id())).unwrap_or_default();
    let error = store.error.take();
    ctx.data_mut(|d| d.insert_temp(id(), store));
    error
}

pub(super) fn load(ctx: &egui::Context) -> History {
    let path = ctx.data(|d| d.get_temp::<Store>(id())).and_then(|s| s.path);
    let Some(path) = path else {
        return ctx
            .data(|d| d.get_temp::<History>(id()))
            .unwrap_or_default();
    };
    let result = match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return History::default(),
        Err(e) => Err(e.to_string()),
    };
    result.unwrap_or_else(|e| {
        error(ctx, format!("Cannot read {}: {e}", path.display()));
        History {
            unreadable: true,
            ..Default::default()
        }
    })
}

pub(super) fn save(ctx: &egui::Context, history: &History) {
    ctx.data_mut(|d| d.insert_temp(id(), history.clone()));
    if history.unreadable {
        return;
    }
    let path = ctx.data(|d| d.get_temp::<Store>(id())).and_then(|s| s.path);
    let Some(path) = path else {
        return;
    };
    let result = serde_json::to_vec_pretty(history)
        .map_err(|e| e.to_string())
        .and_then(|bytes| crate::state_store::atomic_write(&path, &bytes));
    if let Err(e) = result {
        error(ctx, format!("Cannot save {}: {e}", path.display()));
    }
}
