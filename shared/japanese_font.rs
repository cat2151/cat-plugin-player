//! egui's built-in fonts have no Japanese glyphs; dialogs with Japanese text add one on first show.
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};

/// Tried in order.
const FONTS: [&str; 2] = [
    r"C:\Windows\Fonts\meiryo.ttc",
    r"C:\Windows\Fonts\msgothic.ttc",
];

static ADDED: AtomicBool = AtomicBool::new(false);

/// Takes effect from the next frame.
pub(crate) fn add(ctx: &egui::Context) {
    if ADDED.swap(true, Ordering::Relaxed) {
        return;
    }
    let Some(bytes) = FONTS.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };
    ctx.add_font(egui::epaint::text::FontInsert::new(
        "japanese",
        egui::FontData::from_owned(bytes),
        vec![egui::epaint::text::InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: egui::epaint::text::FontPriority::Lowest,
        }],
    ));
}
