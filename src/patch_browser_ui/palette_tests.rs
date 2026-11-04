//! Palette integration with the patch browser's real input handlers.
use super::*;

#[test]
fn palette_runs_browser_action_without_typing_into_browser_search_or_closing_it() {
    let ctx = egui::Context::default();
    let mut browser = browser();
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![key(Key::Slash)]);
    let mut toggle = key(Key::P);
    if let egui::Event::Key { modifiers, .. } = &mut toggle {
        *modifiers = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    }
    frame(&mut browser, &ctx, vec![toggle]);
    frame(&mut browser, &ctx, vec![]);
    frame(&mut browser, &ctx, vec![egui::Event::Text("column".into())]);
    let before = browser.fit_columns;
    frame(&mut browser, &ctx, vec![key(Key::Enter)]);
    assert_eq!(browser.fit_columns, before);
    frame(&mut browser, &ctx, vec![]);
    assert_ne!(browser.fit_columns, before);
    assert!(browser.query.is_empty());
    assert!(browser.open);
    frame(&mut browser, &ctx, vec![]);
    assert_ne!(browser.fit_columns, before);
}
