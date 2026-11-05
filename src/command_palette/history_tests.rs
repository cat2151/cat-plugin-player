use super::{history, *};

struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cat-palette-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn inventory() -> Vec<Command> {
    ["A", "B", "C", "D"]
        .into_iter()
        .map(|s| Command::new(s, s, "", "", s, &[Key::A], true))
        .collect()
}

#[test]
fn query_choices_outrank_global_recency_and_survive_a_new_context() {
    let root = Directory::new();
    let ctx = egui::Context::default();
    let config = root.0.join("config.toml");
    history::configure(&ctx, Some(&config));
    let commands = inventory();
    let mut h = history::load(&ctx);
    h.record("v", &commands[0].identity());
    h.record("other", &commands[1].identity());
    history::save(&ctx, &h);
    // Restart: a new context has no in-memory history.
    let next = egui::Context::default();
    history::configure(&next, Some(&config));
    let loaded = history::load(&next);
    assert_eq!(loaded.rank("v", &commands[0].identity()), (0, 0));
    assert_eq!(loaded.rank("", &commands[1].identity()), (1, 0));
    assert_eq!(loaded.rank("V", &commands[0].identity()), (1, 1));
    assert_eq!(loaded.rank("v ", &commands[0].identity()), (1, 1));
    let mut state = State {
        history: loaded,
        ..Default::default()
    };
    assert_eq!(
        filter(&commands, &state)
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        ["B", "A", "C", "D"]
    );
    state.query = "v".into();
    // Search matching is orthogonal to history ranking; use a matching inventory.
    let mut candidates = commands.clone();
    for c in &mut candidates {
        c.words = "v";
    }
    state.search.update("v");
    assert_eq!(
        filter(&candidates, &state)
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        ["A", "B", "C", "D"]
    );
    let before = std::fs::read(root.0.join("command-palette-history.json")).unwrap();
    super::tests::open(&next);
    super::tests::frame(
        &next,
        vec![super::tests::key(Key::Escape, Modifiers::NONE)],
        true,
    );
    assert_eq!(
        std::fs::read(root.0.join("command-palette-history.json")).unwrap(),
        before
    );
}

#[test]
fn invalid_dispatch_and_unreadable_history_never_overwrite_history() {
    let root = Directory::new();
    let ctx = egui::Context::default();
    history::configure(&ctx, Some(&root.0.join("config.toml")));
    let path = root.0.join("command-palette-history.json");
    std::fs::write(&path, b"broken").unwrap();
    let mut h = history::load(&ctx);
    assert!(history::take_error(&ctx).unwrap().contains("Cannot read"));
    h.record("v", "Main/A");
    history::save(&ctx, &h);
    assert_eq!(std::fs::read(&path).unwrap(), b"broken");
    save(
        &ctx,
        State {
            pending: Some("Disabled action"),
            ..Default::default()
        },
    );
    assert!(super::tests::frame(&ctx, vec![], true).is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), b"broken");
}

#[test]
fn recording_a_choice_deduplicates_each_list_and_keeps_screen_identity() {
    let mut h = history::History::default();
    h.record("v", "Main/A");
    h.record("v", "Patch/A");
    h.record("v", "Main/A");
    assert_eq!(h.rank("v", "Main/A"), (0, 0));
    assert_eq!(h.rank("v", "Patch/A"), (0, 1));
    assert_eq!(h.rank("", "Patch/A"), (1, 1));
    assert_eq!(h.rank("", "unseen"), (2, 0));
}

#[test]
fn an_unhandled_shortcut_does_not_become_successful_history() {
    let ctx = egui::Context::default();
    save(
        &ctx,
        State {
            pending: Some("Next velocity"),
            pending_query: "v".into(),
            ..Default::default()
        },
    );
    let keys = super::tests::frame(&ctx, vec![], true);
    assert!(matches!(keys.as_slice(), [Event::Key { key: Key::V, .. }]));
    assert_eq!(history::load(&ctx).rank("v", "Main/next-velocity"), (2, 0));
    assert!(state(&ctx).open);
}
