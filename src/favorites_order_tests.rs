use super::*;
use crate::{config::PluginKey, sequence_pattern::SequencePattern};

struct Fixture {
    dir: std::path::PathBuf,
    config: std::path::PathBuf,
    library: Library,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "cat-favorite-order-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        Self {
            config: dir.join("config.toml"),
            dir,
            library: Library::default(),
        }
    }

    fn add(&mut self, name: &str, state: u8, automatic: bool) -> String {
        let plugin = PluginKey {
            name: name.into(),
            id: name.to_lowercase(),
            format: "CLAP".into(),
            vendor: String::new(),
            bundle_path: "X:/plugins/test.clap".into(),
        };
        let entry = if automatic {
            self.library.add_automatic(
                &self.config,
                plugin,
                false,
                &[state],
                SequencePattern::Steps,
            )
        } else {
            self.library.add(
                &self.config,
                plugin,
                false,
                &[state],
                SequencePattern::Steps,
            )
        }
        .unwrap();
        entry.id
    }

    fn ids(&self) -> Vec<String> {
        self.library
            .entries
            .iter()
            .filter(|entry| entry.favorite)
            .map(|entry| entry.id.clone())
            .collect()
    }

    fn persisted_ids(&self) -> Vec<String> {
        Library::load(&self.config)
            .unwrap()
            .entries
            .into_iter()
            .filter(|entry| entry.favorite)
            .map(|entry| entry.id)
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn moves_cross_history_only_slots_and_survive_restart_without_touching_snapshots() {
    let mut f = Fixture::new();
    let a = f.add("Alpha", 1, false);
    let h = f.add("History", 2, true);
    let b = f.add("Beta", 3, false);
    let metadata: Vec<_> = f
        .library
        .entries
        .iter()
        .map(|entry| {
            (
                entry.id.clone(),
                toml::to_string(entry).unwrap(),
                f.library.state(&f.config, &entry.id).unwrap(),
            )
        })
        .collect();
    f.library.move_favorite(&f.config, &a, true).unwrap();
    assert_eq!(f.ids(), [a.clone(), b.clone()]);
    assert_eq!(f.persisted_ids(), f.ids());
    assert_eq!(f.library.entries[1].id, h);
    f.library.move_favorite(&f.config, &a, true).unwrap(); // Top boundary.
    f.library.move_favorite(&f.config, &b, false).unwrap(); // Bottom boundary.
    assert_eq!(f.ids(), [a.clone(), b.clone()]);
    assert!(f.library.move_favorite(&f.config, &h, true).is_err());
    f.library.move_favorite(&f.config, &a, false).unwrap();
    assert_eq!(f.persisted_ids(), [b, a]);
    for (id, text, state) in metadata {
        let entry = f
            .library
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .unwrap();
        assert_eq!(toml::to_string(entry).unwrap(), text);
        assert_eq!(f.library.state(&f.config, &id).unwrap(), state);
    }
}

#[test]
fn plugin_sort_is_stable_case_insensitive_and_leaves_history_slots_untouched() {
    let mut f = Fixture::new();
    let z = f.add("Zulu", 1, false);
    let a1 = f.add("Alpha", 2, false);
    let h = f.add("History", 3, true);
    let a2 = f.add("alpha", 4, false);
    f.library.rename(&f.config, &a1, "AAA favorite").unwrap();
    f.library.rename(&f.config, &a2, "ZZZ favorite").unwrap();
    f.library.sort_favorites_by_plugin(&f.config).unwrap();
    assert_eq!(f.ids(), [a2.clone(), a1.clone(), z.clone()]);
    assert_eq!(f.persisted_ids(), f.ids());
    assert_eq!(f.library.entries[1].id, h);
    f.library.move_favorite(&f.config, &z, true).unwrap();
    assert_eq!(f.persisted_ids(), [a2, z, a1]);
}

#[test]
fn resave_and_history_registration_preserve_order_but_new_membership_goes_first() {
    let mut f = Fixture::new();
    let a = f.add("Alpha", 1, false);
    let h = f.add("History", 2, true);
    let b = f.add("Beta", 3, false);
    let before = f.ids();
    assert_eq!(f.add("Alpha", 1, false), a);
    assert_eq!(f.ids(), before);
    assert_eq!(f.add("Alpha", 1, true), a);
    assert_eq!(f.persisted_ids(), before);
    let a_entry = f
        .library
        .entries
        .iter()
        .find(|entry| entry.id == a)
        .unwrap();
    assert!(a_entry.history && a_entry.registered_at > 0);
    assert_eq!(f.add("History", 2, false), h);
    assert_eq!(f.persisted_ids(), [h, b, a]);
    let new = f.add("New", 4, false);
    assert_eq!(f.ids()[0], new);
}

#[test]
fn failed_order_save_rolls_back_moves_sort_and_promotion() {
    let mut f = Fixture::new();
    let a = f.add("Alpha", 1, false);
    f.add("History", 2, true);
    f.add("Zulu", 3, false);
    let before = toml::to_string(&f.library).unwrap();
    let index = f.dir.join("favorites/index.toml");
    std::fs::remove_file(&index).unwrap();
    std::fs::create_dir(&index).unwrap();
    assert!(f.library.move_favorite(&f.config, &a, true).is_err());
    assert_eq!(toml::to_string(&f.library).unwrap(), before);
    assert!(f.library.sort_favorites_by_plugin(&f.config).is_err());
    assert_eq!(toml::to_string(&f.library).unwrap(), before);
    let plugin = f
        .library
        .entries
        .iter()
        .find(|entry| !entry.favorite)
        .unwrap()
        .plugin
        .clone();
    assert!(f
        .library
        .add(&f.config, plugin, false, &[2], SequencePattern::Steps)
        .is_err());
    assert_eq!(toml::to_string(&f.library).unwrap(), before);
    assert_eq!(f.library.state(&f.config, &a).unwrap(), [1]);
}
