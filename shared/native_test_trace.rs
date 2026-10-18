//! Opt-in test-only boundary diagnostics; does not change host scheduling.
pub(crate) fn mark(phase: &str) {
    if std::env::var_os("CAT_NATIVE_TEST_TRACE").is_some() {
        let thread = std::thread::current();
        eprintln!(
            "native-test {:?} {}: {phase}",
            thread.id(),
            thread.name().unwrap_or("unnamed")
        );
    }
}
