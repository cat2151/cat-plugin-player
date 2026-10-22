//! `?` / F1 keybind overlay, one per window; while open it takes every key.
use crate::patch_browser_ui::{editing_text, row_colors};
use eframe::egui::{self, Event, Key};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub(crate) enum Page {
    Main,
    /// The main window in SMF selection mode, where the sequence controls are replaced.
    Smf,
    Patch,
    Effect,
}

/// A function's binds share one color; the colors alone mark the groups.
type Group = &'static [(&'static str, &'static str)];

const MOVE: Group = &[
    ("h j k l  ← ↓ ↑ →", "移動"),
    ("PgUp  PgDn", "10行"),
    ("数字 → 移動", "回数"),
];

const LIBRARY: Group = &[
    ("L", "Favorites / History / Plugins"),
    ("O", "お気に入りの並べ替え"),
    ("N", "お気に入りを plugin 名順に"),
    ("D", "重複 plugin は CLAP だけ"),
    ("Y", "plugin 再スキャン"),
];

const ANALYSIS: Group = &[
    ("A", "解析ラベル"),
    ("P", "解析を右 / 下へ"),
    ("C", "表示周期数"),
    ("Z", "トリガー"),
    ("G", "赤ガイド"),
];

const MAIN: &[Group] = &[
    &[
        ("Space", "再生 / 停止"),
        ("S", "次の Sequence"),
        ("V", "次の Velocity"),
        ("W", "次の CC1 modulation"),
        ("I", "MML"),
        ("M", "Macro 全部 sweep / -"),
    ],
    &[
        ("T", "音色一覧"),
        ("R", "ランダム（音色一覧の絞り込み内）"),
        ("F", "お気に入りに追加"),
        ("/", "絞り込み"),
    ],
    &[
        ("E  X", "エフェクト一覧"),
        ("B", "バイパス"),
        ("U", "楽器 UI 表示 / 非表示"),
    ],
    LIBRARY,
    ANALYSIS,
];

const SMF: &[Group] = &[
    &[
        ("T", "音色一覧"),
        ("F", "お気に入りに追加"),
        ("/", "絞り込み"),
    ],
    &[
        ("E  X", "エフェクト一覧"),
        ("B", "バイパス"),
        ("U", "楽器 UI 表示 / 非表示"),
    ],
    LIBRARY,
    ANALYSIS,
    &[("Esc  Q", "保存して戻る")],
];

const PATCH: &[Group] = &[
    MOVE,
    &[
        ("R", "ランダム"),
        ("Space", "重い音色を読む"),
        ("Enter  Esc", "閉じる"),
    ],
    &[
        ("/", "検索"),
        ("M", "Solo / Mute（a-z Solo, Shift+A-Z Mute）"),
    ],
    &[("F", "お気に入りに追加")],
];

const EFFECT: &[Group] = &[
    MOVE,
    &[("R", "ランダム"), ("Enter  Esc", "閉じる")],
    &[("/", "検索"), ("W", "列幅")],
    &[("F", "お気に入りに追加")],
];

/// Call before the window's other key handling, so Esc and every other key go to the overlay.
pub(crate) fn show(ctx: &egui::Context, page: Page) {
    let id = egui::Id::new("help_overlay").with(ctx.viewport_id());
    let mut open = ctx.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    if (open || !editing_text(ctx)) && ctx.input_mut(take_toggle) {
        open = !open;
    }
    if !open {
        ctx.data_mut(|data| data.insert_temp(id, false));
        return;
    }
    open = !ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape));
    ctx.input_mut(|input| {
        input
            .events
            .retain(|event| !matches!(event, Event::Key { .. } | Event::Text(_)))
    });
    ctx.data_mut(|data| data.insert_temp(id, open));
    if !open {
        return;
    }
    crate::japanese_font::add(ctx);
    let groups = match page {
        Page::Main => MAIN,
        Page::Smf => SMF,
        Page::Patch => PATCH,
        Page::Effect => EFFECT,
    };
    let dark = ctx.style().visuals.dark_mode;
    egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                egui::Grid::new(id.with("grid"))
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        for (slot, binds) in groups.iter().enumerate() {
                            let color = row_colors::color(slot, dark);
                            for (keys, words) in *binds {
                                ui.label(egui::RichText::new(*keys).monospace().color(color));
                                ui.label(*words);
                                ui.end_row();
                            }
                        }
                    });
                ui.weak("? F1 Esc : 閉じる");
            });
        });
}

/// `?` arrives as Shift+`/` on most layouts, so its modifiers are not checked.
fn take_toggle(input: &mut egui::InputState) -> bool {
    let mut toggle = false;
    input.events.retain(|event| match event {
        Event::Key {
            key: Key::Questionmark | Key::F1,
            pressed,
            repeat,
            ..
        } => {
            toggle |= *pressed && !repeat;
            false
        }
        Event::Text(text) => text != "?",
        _ => true,
    });
    toggle
}
