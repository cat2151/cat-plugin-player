//! Compile Migemo once per query, preserving literal and whitespace-separated AND search.
use regex::{Regex, RegexBuilder};
use std::sync::Arc;

#[derive(Clone, Default)]
pub(crate) struct SearchQuery {
    text: String,
    terms: Arc<[(String, Option<Regex>)]>,
}

impl SearchQuery {
    pub(crate) fn update(&mut self, query: &str) {
        let text = query.to_lowercase();
        if self.text == text {
            return;
        }
        self.text = text;
        self.terms = self
            .text
            .split_whitespace()
            .map(|term| {
                let pattern = term
                    .bytes()
                    .all(|b| b.is_ascii_alphabetic())
                    .then(|| super::migemo::pattern(term));
                let regex =
                    pattern.and_then(|p| RegexBuilder::new(&p).case_insensitive(true).build().ok());
                (term.to_owned(), regex)
            })
            .collect();
    }

    pub(crate) fn matches(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        self.terms.iter().all(|(term, regex)| {
            lower.contains(term) || regex.as_ref().is_some_and(|r| r.is_match(text))
        })
    }
}
