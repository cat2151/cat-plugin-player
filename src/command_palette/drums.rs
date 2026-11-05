use super::Command;
use crate::App;
use eframe::egui::Key;

pub(super) fn commands(app: &App) -> Vec<Command> {
    let selected = app.drum_browser.selected().is_some();
    vec![
        Command::new(
            "drum-velocity",
            "Toggle velocity comparison",
            "強弱比較を切り替え",
            "drum velocity compare 強弱 比較",
            "V",
            &[Key::V],
            true,
        ),
        Command::new(
            "drum-preview",
            "Preview selected",
            "選択を試聴",
            "drum kit note play",
            "Space",
            &[Key::Space],
            selected,
        ),
        Command::new(
            "drum-stop",
            "Stop preview",
            "試聴停止",
            "drum stop",
            "P",
            &[Key::P],
            true,
        ),
        Command::new(
            "drum-filter",
            "Search kits",
            "キットを検索",
            "drum filter",
            "/",
            &[Key::Slash],
            app.drum_browser.saved.focus == crate::drum_browser::Focus::Kits,
        ),
        Command::new(
            "close-drum-browser",
            "Close drum kits",
            "ドラム一覧を閉じる",
            "drum close",
            "Esc",
            &[Key::Escape],
            true,
        ),
        Command::new(
            "keyboard-help",
            "Keyboard help",
            "キー操作のヘルプ",
            "help",
            "F1",
            &[Key::F1],
            true,
        ),
    ]
}
