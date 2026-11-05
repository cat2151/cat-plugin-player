use super::*;

fn plugin_command() -> Command {
    // No romaji aliases: the visible Japanese description must be searchable.
    Command::new(
        "rescan",
        "Rescan",
        "プラグインを再スキャン…",
        "",
        "F5",
        &[Key::F5],
        true,
    )
}

#[test]
fn migemo_searches_japanese_description_and_preserves_literal_and_search() {
    let command = plugin_command();
    let mut query = search::SearchQuery::default();
    for text in [
        "pura",
        "PURA",
        "puraguin",
        "プラグイン",
        "pura rescan",
        "pura F5",
        "",
    ] {
        query.update(text);
        assert!(command.matches(&query), "{text}");
    }
    for text in ["pura velocity", "[", ".*", "(", "😀"] {
        query.update(text);
        assert!(!command.matches(&query), "{text}");
    }
    let save = Command::new("save", "Save", "設定を保存", "", "S", &[Key::S], true);
    query.update("hozon");
    assert!(save.matches(&query));
    assert!(!command.matches(&query));
    query.update("pura");
    assert!(command.matches(&query));
}

#[test]
fn typing_romaji_filters_visible_rows_and_enter_dispatches_the_match() {
    let ctx = egui::Context::default();
    let frame = |events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                let suspended = begin(ctx, vec![plugin_command()], true);
                egui::CentralPanel::default().show(ctx, |_| {});
                end(ctx, suspended);
            },
        )
    };
    save(
        &ctx,
        State {
            open: true,
            ..Default::default()
        },
    );
    for _ in 0..3 {
        frame(vec![]);
    }
    frame(vec![Event::Text("pura".into())]);
    let output = frame(vec![]);
    assert_eq!(state(&ctx).query, "pura");
    assert!(output.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Text(text)
            if text.galley.job.text == "Main: Rescan / プラグインを再スキャン…")
    }));
    frame(vec![Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!(!state(&ctx).open);
    assert_eq!(state(&ctx).pending, Some("Rescan"));
}
