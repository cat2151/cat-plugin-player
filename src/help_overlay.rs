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
    Settings,
    Analysis,
    Drum,
}

/// A function's binds share one color; the colors alone mark the groups.
type Group = &'static [(&'static str, &'static str)];

const MOVE: Group = &[
    ("h j k l  ← ↓ ↑ →", "移動"),
    ("PgUp  PgDn", "10行"),
    ("Home  End", "先頭 / 末尾"),
    ("数字 → 移動", "回数"),
];

const DRUM: &[Group] = &[
    &[
        ("h / ←   l / →", "Kit / Note focus"),
        ("j k / ↓ ↑", "1行移動"),
        ("PgUp PgDn", "10行・端で停止"),
        ("Home End", "先頭 / 末尾"),
    ],
    &[
        ("Space (Kit)", "ロード・全note一周"),
        ("Space (Note)", "単音試聴・重いkitもロード"),
        ("Shift+Space (Note)", "全note一周"),
        ("V", "Velocity loop"),
        ("P", "試聴停止（残響は鳴り切る場合あり）"),
    ],
    &[
        ("/ (Kit)", "検索編集"),
        ("Enter / Esc (検索)", "確定 / queryを戻す"),
        ("Enter / Esc", "適用済みkitを保持して閉じる"),
        ("Ctrl+Shift+P", "コマンドパレット"),
    ],
];

const LIBRARY: Group = &[
    ("Ctrl+Shift+P", "コマンドパレット"),
    ("L", "Favorites / History / Plugins"),
    ("D", "Settings メニュー"),
    ("A", "解析表示メニュー"),
];

const SETTINGS: &[Group] = &[
    &[
        ("O", "お気に入りの並べ替え"),
        ("N", "お気に入りを plugin 名順に"),
        ("D", "重複 plugin は CLAP だけ"),
        ("Y", "plugin 再スキャン"),
    ],
    &[("Esc", "メニューを閉じる")],
];

const ANALYSIS: &[Group] = &[
    &[
        ("A", "解析ラベル"),
        ("P", "解析を右 / 下へ"),
        ("C", "表示周期数"),
        ("Z", "トリガー"),
        ("G", "赤ガイド"),
    ],
    &[("Esc", "メニューを閉じる")],
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
        ("G", "Drum kits（通常モード）"),
        ("R", "ランダム（音色一覧の絞り込み内）"),
        ("F", "お気に入りに追加"),
        ("/", "絞り込み"),
    ],
    &[
        ("E  X", "エフェクト一覧"),
        ("B", "バイパス"),
        ("U", "楽器 UI 表示 / 非表示"),
        ("O → a–z", "エフェクト UI 切替（1つなら O のみ）"),
    ],
    LIBRARY,
];

const SMF: &[Group] = &[
    &[
        ("T", "音色一覧"),
        ("G", "Drum kits（通常モード）"),
        ("F", "お気に入りに追加"),
        ("/", "絞り込み"),
    ],
    &[
        ("E  X", "エフェクト一覧"),
        ("B", "バイパス"),
        ("U", "楽器 UI 表示 / 非表示"),
        ("O → a–z", "エフェクト UI 切替（1つなら O のみ）"),
    ],
    LIBRARY,
    &[("Esc  Q", "保存して戻る")],
];

const PATCH: &[Group] = &[
    &[("Ctrl+Shift+P", "コマンドパレット")],
    MOVE,
    &[
        ("R  右クリック", "ランダム"),
        ("Space", "重い音色を読む"),
        ("Enter  Esc", "閉じる"),
    ],
    &[
        ("/", "検索"),
        ("M", "Solo / Mute（a-z Solo, Shift+A-Z Mute）"),
        ("W", "列幅"),
    ],
    &[("[  ]", "前 / 次の Type")],
    &[("F", "お気に入りに追加")],
];

const EFFECT: &[Group] = &[
    &[("Ctrl+Shift+P", "コマンドパレット")],
    MOVE,
    &[("R", "ランダム"), ("Enter  Esc", "閉じる")],
    &[("/", "検索"), ("W", "列幅")],
    &[("F", "お気に入りに追加")],
];

fn id(ctx: &egui::Context) -> egui::Id {
    egui::Id::new("help_overlay").with(ctx.viewport_id())
}

pub(crate) fn open(ctx: &egui::Context, page: Page) {
    let id = id(ctx);
    ctx.data_mut(|data| {
        data.insert_temp(id, true);
        data.insert_temp(id.with("page"), page);
    });
    ctx.request_repaint();
}

pub(crate) fn is_open(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(id(ctx)))
        .unwrap_or(false)
}

/// Call before the window's other key handling, so Esc and every other key go to the overlay.
pub(crate) fn show(ctx: &egui::Context, page: Page) {
    let menu = if matches!(page, Page::Main | Page::Smf) {
        crate::main_menu::Menu::current(ctx)
    } else {
        None
    };
    let page = menu.map_or(page, |menu| menu.help_page());
    let id = id(ctx);
    let mut open = ctx.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    if (open || menu.is_some() || !editing_text(ctx)) && ctx.input_mut(take_toggle) {
        open = !open;
        if open {
            ctx.data_mut(|data| data.insert_temp(id.with("page"), page));
        }
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
    let page = ctx
        .data(|data| data.get_temp::<Page>(id.with("page")))
        .unwrap_or(page);
    let groups = match page {
        Page::Main => MAIN,
        Page::Smf => SMF,
        Page::Patch => PATCH,
        Page::Effect => EFFECT,
        Page::Settings => SETTINGS,
        Page::Analysis => ANALYSIS,
        Page::Drum => DRUM,
    };
    let dark = ctx.style().visuals.dark_mode;
    egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                match page {
                    Page::Settings => {
                        ui.heading("Settings");
                    }
                    Page::Analysis => {
                        ui.heading("Analysis display settings");
                    }
                    _ => {}
                }
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
