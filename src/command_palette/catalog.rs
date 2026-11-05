//! Context-specific inventory; execution stays in the existing shortcut handlers.
use super::Command;
use crate::App;
use eframe::egui::Key;

pub(crate) fn main_commands(app: &App) -> Vec<Command> {
    let ready = !app.actions_busy();
    // Availability after navigating to Main; direct shortcuts keep their own input guard.
    let standalone = app.selection.is_none();
    let instrument = app.instrument_id().is_some();
    let mut commands = vec![
        Command::new(
            "browse-drums",
            "Drum kits",
            "ドラムキット一覧",
            "drum kit note preview",
            "G",
            &[Key::G],
            app.can_open_drums() && !app.drum_browser.open,
        ),
        Command::new(
            "browse-patches",
            "Browse patches",
            "音色一覧を開く",
            "音色 一覧 楽器",
            "T",
            &[Key::T],
            !app.selecting_master() && !app.patch_browser.open,
        ),
        Command::new(
            "browse-effects",
            "Browse effects",
            "エフェクト一覧を開く",
            "エフェクト 一覧",
            "E / X",
            &[Key::E],
            !app.effect_browser.open,
        ),
        Command::new(
            "add-favorite",
            "Add favorite",
            "お気に入りに追加",
            "お気に入り 追加",
            "F",
            &[Key::F],
            instrument,
        ),
        Command::new(
            "filter-library",
            "Filter library",
            "ライブラリを検索・絞り込み",
            "検索 絞り込み",
            "/",
            &[Key::Slash],
            true,
        ),
        Command::new(
            "next-library-tab",
            "Next library tab",
            "ライブラリの次のタブへ",
            "お気に入り 履歴 プラグイン Favorites History Plugins",
            "L",
            &[Key::L],
            true,
        ),
        Command::new(
            "bypass-effects",
            "Bypass effects",
            "エフェクトのバイパスを切り替え",
            "エフェクト バイパス",
            "B",
            &[Key::B],
            ready && app.effect_id().is_some(),
        ),
        Command::new(
            "show-hide-instrument-ui",
            "Show / hide instrument UI",
            "楽器の画面を表示・非表示",
            "楽器 表示 非表示",
            "U",
            &[Key::U],
            ready && instrument,
        ),
        Command::new(
            "show-hide-effect-ui",
            "Show / hide effect UI",
            "エフェクトの画面を表示・非表示",
            "エフェクト 表示 非表示",
            "O",
            &[Key::O],
            ready && app.effect_id().is_some(),
        ),
        Command::new(
            "reorder-favorites",
            "Reorder favorites",
            "お気に入りを並べ替え",
            "お気に入り 並べ替え",
            "D → O",
            &[Key::D, Key::O],
            app.favorites.show && !app.favorites.show_history,
        ),
        Command::new(
            "sort-favorites-by-plugin-name",
            "Sort favorites by plugin name",
            "お気に入りをプラグイン名順に整列",
            "お気に入り プラグイン 名前 順",
            "D → N",
            &[Key::D, Key::N],
            app.can_sort_favorites(),
        ),
        Command::new(
            "prefer-clap-for-duplicate-plugins",
            "Prefer CLAP for duplicate plugins",
            "重複プラグインはCLAPを優先",
            "重複 プラグイン",
            "D → D",
            &[Key::D, Key::D],
            true,
        ),
        Command::new(
            "rescan-plugins",
            "Rescan plugins...",
            "プラグインを再スキャン…",
            "プラグイン 再スキャン",
            "D → Y",
            &[Key::D, Key::Y],
            app.can_rescan(),
        ),
        Command::new(
            "toggle-analysis-labels",
            "Toggle analysis labels",
            "解析ラベルの表示を切り替え",
            "解析 ラベル",
            "A → A",
            &[Key::A, Key::A],
            true,
        ),
        Command::new(
            "move-analysis-right-bottom",
            "Move analysis right / bottom",
            "解析の配置を右・下に切り替え",
            "解析 右 下 配置",
            "A → P",
            &[Key::A, Key::P],
            true,
        ),
        Command::new(
            "next-scope-cycle-count",
            "Next scope cycle count",
            "波形の表示周期数を切り替え",
            "波形 表示 周期数",
            "A → C",
            &[Key::A, Key::C],
            true,
        ),
        Command::new(
            "toggle-scope-trigger",
            "Toggle scope trigger",
            "波形トリガーを切り替え",
            "波形 トリガー",
            "A → Z",
            &[Key::A, Key::Z],
            true,
        ),
        Command::new(
            "toggle-scope-guides",
            "Toggle scope guides",
            "波形ガイドの表示を切り替え",
            "波形 赤 ガイド",
            "A → G",
            &[Key::A, Key::G],
            true,
        ),
        Command::new(
            "keyboard-help",
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
                "play-stop-sequence",
                "Play / stop sequence",
                "シーケンスを再生・停止",
                "再生 停止",
                "Space",
                &[Key::Space],
                ready && app.instances.iter().any(|i| i.voice.is_some()),
            ),
            Command::new(
                "next-sequence",
                "Next sequence",
                "次のシーケンスへ",
                "シーケンス 次",
                "S",
                &[Key::S],
                ready,
            ),
            Command::new(
                "next-velocity",
                "Next velocity",
                "次のベロシティへ",
                "ベロシティ 次",
                "V",
                &[Key::V],
                ready,
            ),
            Command::new(
                "next-cc1-modulation",
                "Next CC1 modulation",
                "次のCC1モジュレーションへ",
                "モジュレーション 次",
                "W",
                &[Key::W],
                ready,
            ),
            Command::new(
                "edit-mml-chord",
                "Edit MML / chord",
                "MML・コードを入力・編集",
                "コード 入力 編集",
                "I",
                &[Key::I],
                ready,
            ),
            Command::new(
                "toggle-all-macro-sweeps",
                "Toggle all macro sweeps",
                "すべてのマクロのスイープを切り替え",
                "マクロ 全部",
                "M",
                &[Key::M],
                ready && app.macro_lane.controls.targets.iter().any(Option::is_some),
            ),
            Command::new(
                "random-patch",
                "Random patch",
                "ランダムに音色を選択",
                "ランダム 音色",
                "R",
                &[Key::R],
                !app.selecting_master(),
            ),
        ]);
    }
    if app.drum_kit_active() {
        commands.retain(|command| {
            !matches!(
                command.id,
                "play-stop-sequence"
                    | "next-sequence"
                    | "next-velocity"
                    | "next-cc1-modulation"
                    | "edit-mml-chord"
                    | "toggle-all-macro-sweeps"
            )
        });
    }
    commands
}

pub(crate) fn patch_commands(browser: &crate::patch_browser::Browser) -> Vec<Command> {
    let ready = browser.snapshot.is_some();
    vec![
        Command::new(
            "random-patch",
            "Random patch",
            "ランダムに音色を選択",
            "ランダム 音色",
            "R",
            &[Key::R],
            ready && browser.error.is_none() && !browser.results.is_empty(),
        ),
        Command::new(
            "search-patches",
            "Search patches",
            "音色を検索・絞り込み",
            "検索 絞り込み",
            "/",
            &[Key::Slash],
            ready,
        ),
        Command::new(
            "instruments-solo-mute",
            "Instruments Solo / Mute",
            "楽器のソロ・ミュート",
            "楽器 ソロ ミュート",
            "M",
            &[Key::M],
            ready,
        ),
        Command::new(
            "toggle-column-width",
            "Toggle column width",
            "列幅を切り替え",
            "列 幅",
            "W",
            &[Key::W],
            ready && browser.error.is_none(),
        ),
        Command::new(
            "previous-type",
            "Previous type",
            "前のタイプへ",
            "前 タイプ",
            "[",
            &[Key::OpenBracket],
            ready && browser.error.is_none(),
        ),
        Command::new(
            "next-type",
            "Next type",
            "次のタイプへ",
            "次 タイプ",
            "]",
            &[Key::CloseBracket],
            ready && browser.error.is_none(),
        ),
        Command::new(
            "load-selected-heavy-patch",
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
            "add-favorite",
            "Add favorite",
            "お気に入りに追加",
            "お気に入り 追加",
            "F",
            &[Key::F],
            ready,
        ),
        Command::new(
            "close-patch-browser",
            "Close patch browser",
            "音色一覧を閉じる",
            "音色 一覧 閉じる",
            "Enter",
            &[Key::Enter],
            ready,
        ),
        Command::new(
            "keyboard-help",
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
            "random-effect",
            "Random effect",
            "ランダムにエフェクトを選択",
            "ランダム エフェクト",
            "R",
            &[Key::R],
            browser.error.is_none() && !browser.results.is_empty(),
        ),
        Command::new(
            "search-effects",
            "Search effects",
            "エフェクトを検索・絞り込み",
            "検索 絞り込み",
            "/",
            &[Key::Slash],
            !browser.candidates.is_empty(),
        ),
        Command::new(
            "toggle-column-width",
            "Toggle column width",
            "列幅を切り替え",
            "列 幅",
            "W",
            &[Key::W],
            !browser.candidates.is_empty(),
        ),
        Command::new(
            "add-favorite",
            "Add favorite",
            "お気に入りに追加",
            "お気に入り 追加",
            "F",
            &[Key::F],
            browser.target.is_some(),
        ),
        Command::new(
            "close-effect-browser",
            "Close effect browser",
            "エフェクト一覧を閉じる",
            "エフェクト 一覧 閉じる",
            "Enter",
            &[Key::Enter],
            true,
        ),
        Command::new(
            "keyboard-help",
            "Keyboard help",
            "キーボード操作のヘルプ",
            "ヘルプ 操作",
            "F1",
            &[Key::F1],
            true,
        ),
    ]
}
