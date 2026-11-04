//! Context-specific inventory; execution stays in the existing shortcut handlers.
use super::Command;
use crate::App;
use eframe::egui::Key;

pub(crate) fn main_commands(app: &App) -> Vec<Command> {
    let ready = !app.actions_busy() && !app.browser_open();
    let main = !app.browser_open();
    let standalone = app.selection.is_none();
    let instrument = app.instrument_id().is_some();
    let mut commands = vec![
        Command::new(
            "Browse patches",
            "音色一覧を開く",
            "音色 一覧 楽器",
            "T",
            &[Key::T],
            main && !app.selecting_master(),
        ),
        Command::new(
            "Browse effects",
            "エフェクト一覧を開く",
            "エフェクト 一覧",
            "E / X",
            &[Key::E],
            main,
        ),
        Command::new(
            "Add favorite",
            "お気に入りに追加",
            "お気に入り 追加",
            "F",
            &[Key::F],
            main && instrument,
        ),
        Command::new(
            "Filter library",
            "ライブラリを検索・絞り込み",
            "検索 絞り込み",
            "/",
            &[Key::Slash],
            main,
        ),
        Command::new(
            "Next library tab",
            "ライブラリの次のタブへ",
            "お気に入り 履歴 プラグイン Favorites History Plugins",
            "L",
            &[Key::L],
            main,
        ),
        Command::new(
            "Bypass effects",
            "エフェクトのバイパスを切り替え",
            "エフェクト バイパス",
            "B",
            &[Key::B],
            ready && app.effect_id().is_some(),
        ),
        Command::new(
            "Show / hide instrument UI",
            "楽器の画面を表示・非表示",
            "楽器 表示 非表示",
            "U",
            &[Key::U],
            ready && instrument,
        ),
        Command::new(
            "Reorder favorites",
            "お気に入りを並べ替え",
            "お気に入り 並べ替え",
            "D → O",
            &[Key::D, Key::O],
            main && app.favorites.show && !app.favorites.show_history,
        ),
        Command::new(
            "Sort favorites by plugin name",
            "お気に入りをプラグイン名順に整列",
            "お気に入り プラグイン 名前 順",
            "D → N",
            &[Key::D, Key::N],
            main && app.can_sort_favorites(),
        ),
        Command::new(
            "Prefer CLAP for duplicate plugins",
            "重複プラグインはCLAPを優先",
            "重複 プラグイン",
            "D → D",
            &[Key::D, Key::D],
            main,
        ),
        Command::new(
            "Rescan plugins...",
            "プラグインを再スキャン…",
            "プラグイン 再スキャン",
            "D → Y",
            &[Key::D, Key::Y],
            main && app.can_rescan(),
        ),
        Command::new(
            "Toggle analysis labels",
            "解析ラベルの表示を切り替え",
            "解析 ラベル",
            "A → A",
            &[Key::A, Key::A],
            main,
        ),
        Command::new(
            "Move analysis right / bottom",
            "解析の配置を右・下に切り替え",
            "解析 右 下 配置",
            "A → P",
            &[Key::A, Key::P],
            main,
        ),
        Command::new(
            "Next scope cycle count",
            "波形の表示周期数を切り替え",
            "波形 表示 周期数",
            "A → C",
            &[Key::A, Key::C],
            main,
        ),
        Command::new(
            "Toggle scope trigger",
            "波形トリガーを切り替え",
            "波形 トリガー",
            "A → Z",
            &[Key::A, Key::Z],
            main,
        ),
        Command::new(
            "Toggle scope guides",
            "波形ガイドの表示を切り替え",
            "波形 赤 ガイド",
            "A → G",
            &[Key::A, Key::G],
            main,
        ),
        Command::new(
            "Keyboard help",
            "キーボード操作のヘルプ",
            "ヘルプ 操作",
            "F1",
            &[Key::F1],
            true,
        ),
    ];
    if standalone {
        commands.extend([
            Command::new(
                "Play / stop sequence",
                "シーケンスを再生・停止",
                "再生 停止",
                "Space",
                &[Key::Space],
                ready && app.instances.iter().any(|i| i.voice.is_some()),
            ),
            Command::new(
                "Next sequence",
                "次のシーケンスへ",
                "シーケンス 次",
                "S",
                &[Key::S],
                ready,
            ),
            Command::new(
                "Next velocity",
                "次のベロシティへ",
                "ベロシティ 次",
                "V",
                &[Key::V],
                ready,
            ),
            Command::new(
                "Next CC1 modulation",
                "次のCC1モジュレーションへ",
                "モジュレーション 次",
                "W",
                &[Key::W],
                ready,
            ),
            Command::new(
                "Edit MML / chord",
                "MML・コードを入力・編集",
                "コード 入力 編集",
                "I",
                &[Key::I],
                ready,
            ),
            Command::new(
                "Toggle all macro sweeps",
                "すべてのマクロのスイープを切り替え",
                "マクロ 全部",
                "M",
                &[Key::M],
                ready && app.macro_lane.controls.targets.iter().any(Option::is_some),
            ),
            Command::new(
                "Random patch",
                "ランダムに音色を選択",
                "ランダム 音色",
                "R",
                &[Key::R],
                main && !app.selecting_master(),
            ),
        ]);
    }
    commands
}

pub(crate) fn patch_commands(browser: &crate::patch_browser::Browser) -> Vec<Command> {
    let ready = browser.snapshot.is_some();
    vec![
        Command::new(
            "Random patch",
            "ランダムに音色を選択",
            "ランダム 音色",
            "R",
            &[Key::R],
            ready && browser.error.is_none() && !browser.results.is_empty(),
        ),
        Command::new(
            "Search patches",
            "音色を検索・絞り込み",
            "検索 絞り込み",
            "/",
            &[Key::Slash],
            ready,
        ),
        Command::new(
            "Instruments Solo / Mute",
            "楽器のソロ・ミュート",
            "楽器 ソロ ミュート",
            "M",
            &[Key::M],
            ready,
        ),
        Command::new(
            "Toggle column width",
            "列幅を切り替え",
            "列 幅",
            "W",
            &[Key::W],
            ready && browser.error.is_none(),
        ),
        Command::new(
            "Previous type",
            "前のタイプへ",
            "前 タイプ",
            "[",
            &[Key::OpenBracket],
            ready && browser.error.is_none(),
        ),
        Command::new(
            "Next type",
            "次のタイプへ",
            "次 タイプ",
            "]",
            &[Key::CloseBracket],
            ready && browser.error.is_none(),
        ),
        Command::new(
            "Load selected heavy patch...",
            "選択した重い音色をロード…",
            "重い 音色 ロード",
            "Space",
            &[Key::Space],
            ready
                && browser
                    .requests
                    .selected
                    .as_ref()
                    .is_some_and(|ticket| ticket.candidate.measurement.is_heavy_offline_load()),
        ),
        Command::new(
            "Add favorite",
            "お気に入りに追加",
            "お気に入り 追加",
            "F",
            &[Key::F],
            ready,
        ),
        Command::new(
            "Close patch browser",
            "音色一覧を閉じる",
            "音色 一覧 閉じる",
            "Enter",
            &[Key::Enter],
            ready,
        ),
        Command::new(
            "Keyboard help",
            "キーボード操作のヘルプ",
            "ヘルプ 操作",
            "F1",
            &[Key::F1],
            true,
        ),
    ]
}

pub(crate) fn effect_commands(browser: &crate::effect_browser::EffectBrowser) -> Vec<Command> {
    vec![
        Command::new(
            "Random effect",
            "ランダムにエフェクトを選択",
            "ランダム エフェクト",
            "R",
            &[Key::R],
            browser.error.is_none() && !browser.results.is_empty(),
        ),
        Command::new(
            "Search effects",
            "エフェクトを検索・絞り込み",
            "検索 絞り込み",
            "/",
            &[Key::Slash],
            !browser.candidates.is_empty(),
        ),
        Command::new(
            "Toggle column width",
            "列幅を切り替え",
            "列 幅",
            "W",
            &[Key::W],
            !browser.candidates.is_empty(),
        ),
        Command::new(
            "Add favorite",
            "お気に入りに追加",
            "お気に入り 追加",
            "F",
            &[Key::F],
            browser.target.is_some(),
        ),
        Command::new(
            "Close effect browser",
            "エフェクト一覧を閉じる",
            "エフェクト 一覧 閉じる",
            "Enter",
            &[Key::Enter],
            true,
        ),
        Command::new(
            "Keyboard help",
            "キーボード操作のヘルプ",
            "ヘルプ 操作",
            "F1",
            &[Key::F1],
            true,
        ),
    ]
}
