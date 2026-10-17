//! Apply the editor's program-name search once the asynchronous catalog is ready.
use crate::patch_browser::Browser;

impl Browser {
    pub fn set_initial_filter(&mut self, name: &str) {
        let terms: Vec<_> = name
            .split_whitespace()
            .map(|word| regex::escape(word).replace(':', "\\x3a"))
            .collect();
        self.role = 0;
        self.preset = 0;
        self.restore_preset = None;
        self.query = terms.join(" ");
        self.initial_filter_fallback = Some((self.query.clone(), terms.join("|")));
        self.filter(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch_browser_catalog::{tests::candidate, Snapshot};

    fn snapshot(names: &[&str]) -> Snapshot {
        Snapshot::build(
            names
                .iter()
                .map(|name| candidate("a", "Alpha", name, None))
                .collect(),
            &[],
        )
        .unwrap()
    }

    #[test]
    fn waits_for_catalog_and_falls_back_only_once_without_changing_sound() {
        let mut browser = Browser::default();
        browser.set_initial_filter("church organ");
        assert_eq!(browser.query, "church organ");
        assert!(browser.initial_filter_fallback.is_some());
        browser.snapshot = Some(snapshot(&["Bright Organ", "Church Bells", "Piano"]));
        browser.filter(false);
        assert_eq!(browser.query, "church|organ");
        assert_eq!(browser.results.len(), 2);
        assert!(browser.selected.is_none());
        browser.query = "unknown words".into();
        browser.filter(false);
        assert_eq!(browser.query, "unknown words");
    }

    #[test]
    fn matching_and_query_is_retained_and_program_names_are_literal() {
        let mut browser = Browser {
            snapshot: Some(snapshot(&["Church Organ", "Organ"])),
            ..Default::default()
        };
        browser.set_initial_filter("church organ");
        assert_eq!(browser.query, "church organ");
        assert_eq!(browser.results.len(), 1);
        browser.snapshot = Some(snapshot(&["Guitar (Nylon)", "Guitar Steel"]));
        browser.set_initial_filter("guitar (nylon)");
        assert_eq!(browser.query, "guitar \\(nylon\\)");
        assert_eq!(browser.results.len(), 1);
    }
}
