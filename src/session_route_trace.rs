//! Test-only evidence for the stopped-renderer transaction boundary.
use std::cell::RefCell;

thread_local! {
    static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn record(event: &'static str) {
    EVENTS.with_borrow_mut(|events| events.push(event));
}

pub(crate) fn take() -> Vec<&'static str> {
    EVENTS.with_borrow_mut(std::mem::take)
}
