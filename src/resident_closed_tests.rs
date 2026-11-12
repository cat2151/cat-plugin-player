//! Hiding the patch browser tells the editor once; the kit browser keeps its own notice.
use super::*;

fn key(key: egui::Key) -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    }]
}

/// Everything the player sent until its pipe closed.
fn sent_until_closed(app: App, editor_in: &mut Receiver) -> Vec<ToEditor> {
    drop(app);
    let mut sent = Vec::new();
    while let Some(message) = editor_in.receive::<ToEditor>().unwrap() {
        sent.push(message);
    }
    sent
}

fn closed_notices(sent: &[ToEditor]) -> Vec<ToEditor> {
    sent.iter()
        .filter(|m| {
            matches!(
                m,
                ToEditor::BrowserClosed { .. } | ToEditor::KitClosed { .. }
            )
        })
        .cloned()
        .collect()
}

fn hide_patch_browser(test: &str, events: Vec<egui::Event>, close: bool) -> Vec<ToEditor> {
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut editor_in)) = app(test);
    let ctx = egui::Context::default();
    let open = Open::Patch { track: 2 };
    request(&mut app, &mut editor, open);
    frame(&mut app, &ctx, Vec::new(), false);
    frame(&mut app, &ctx, events, close);
    assert_eq!(shown(&app), None);
    // Further frames while hidden must not repeat the notice.
    frame(&mut app, &ctx, Vec::new(), false);
    closed_notices(&sent_until_closed(app, &mut editor_in))
}

#[test]
fn esc_q_and_the_close_button_each_send_browser_closed_once() {
    if crate::native_library::process::isolated() {
        return;
    }
    let expected = vec![ToEditor::BrowserClosed {
        target: Open::Patch { track: 2 },
    }];
    assert_eq!(
        hide_patch_browser("closed esc", key(egui::Key::Escape), false),
        expected
    );
    assert_eq!(
        hide_patch_browser("closed q", key(egui::Key::Q), false),
        expected
    );
    assert_eq!(
        hide_patch_browser("closed button", Vec::new(), true),
        expected
    );
}

#[test]
fn the_kit_browser_sends_only_kit_closed() {
    if crate::native_library::process::isolated() {
        return;
    }
    let _library = crate::native_library::load().unwrap();
    let (mut app, (mut editor, mut editor_in)) = app("closed kit");
    let open = Open::Kit {
        track: 1,
        layer: None,
        song: 2,
        request: 3,
    };
    request(&mut app, &mut editor, open);
    assert!(app.drum_browser.open);
    let ctx = egui::Context::default();
    super::kit::drum_frame(&mut app, &ctx, super::kit::drum_keys(egui::Key::Escape));
    assert_eq!(shown(&app), None);
    assert_eq!(
        closed_notices(&sent_until_closed(app, &mut editor_in)),
        vec![ToEditor::KitClosed { target: open }]
    );
}
