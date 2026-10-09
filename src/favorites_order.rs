//! Explicit order changes affect Favorites slots only, never snapshot contents.
use super::Library;
use std::path::Path;

impl Library {
    pub(crate) fn move_favorite(
        &mut self,
        config: &Path,
        id: &str,
        up: bool,
    ) -> Result<(), String> {
        let slots = self.favorite_slots();
        let position = slots
            .iter()
            .position(|&i| self.entries[i].id == id)
            .ok_or("favorite not found")?;
        let neighbor = if up {
            position.checked_sub(1)
        } else {
            position.checked_add(1).filter(|&i| i < slots.len())
        };
        let Some(neighbor) = neighbor else {
            return Ok(());
        };
        let (a, b) = (slots[position], slots[neighbor]);
        self.entries.swap(a, b);
        if let Err(error) = self.save(config) {
            self.entries.swap(a, b);
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn sort_favorites_by_plugin(&mut self, config: &Path) -> Result<(), String> {
        let slots = self.favorite_slots();
        let mut ordered: Vec<_> = slots.iter().map(|&i| self.entries[i].clone()).collect();
        // Stable, case-insensitive ascending order; equal names retain manual order.
        ordered.sort_by_cached_key(|entry| entry.plugin.name.to_lowercase());
        let previous = self.entries.clone();
        for (i, entry) in slots.into_iter().zip(ordered) {
            self.entries[i] = entry;
        }
        if let Err(error) = self.save(config) {
            self.entries = previous;
            return Err(error);
        }
        Ok(())
    }

    fn favorite_slots(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| entry.favorite.then_some(i))
            .collect()
    }
}

#[cfg(test)]
#[path = "favorites_order_tests.rs"]
mod tests;
