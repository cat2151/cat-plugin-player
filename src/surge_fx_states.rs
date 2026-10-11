//! Prepared Surge XT Effects states from cat-music-patterns (ADR 0028).
mod table;

pub(crate) use table::{prepare_state, Table, PLUGIN_ID};

const URL: &str =
    "https://raw.githubusercontent.com/cat2151/cat-music-patterns/main/surge-xt-fx-states.json";

/// Refreshes the saved table from [`URL`]; falls back to the previous file.
pub(crate) fn load() -> Result<Table, String> {
    let destination = crate::config::path()?
        .parent()
        .ok_or("config path has no parent")?
        .join("patch-catalog")
        .join("surge-xt-fx-states.json");
    let path = crate::surge_patch_catalog::refresh(
        || crate::surge_patch_catalog::fetch(URL),
        &destination,
        |bytes, destination| {
            Table::parse(bytes)?;
            crate::state_store::atomic_write(destination, bytes)
        },
    )
    .map_err(|error| format!("Surge XT Effects states: {error}"))?;
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    Table::parse(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests;
