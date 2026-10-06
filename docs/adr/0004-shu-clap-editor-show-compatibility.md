# ADR 0004: Shu の Windows CLAP GUI の表示失敗を限定対応する

- 状態: 採用（実装・実機回帰テスト済み。userの画面操作による確認は未実施）
- 日付: 2026-10-07
- 関連: [ADR 0002](0002-plugin-specific-favorite-state-comparison.md)

## 問題と確認した事実

effect の Shu で Show UI を押すと、一瞬表示された後にウィンドウが消える。
userからは、中身は表示されていたが小さいウィンドウで見切れていたように見えた、との補足があった。

既存の shim DLL とインストール済み Shu を使って再現したところ、GUI作成は成功し、
`uh_ui_show()` が表示失敗 `UH_ERR_SHOW_UI (-4)` を返した。
ホストは親ウィンドウを先に表示し、`showUI()` が失敗するとその親を隠すため、一瞬で消える。

CLAP API の直接呼び出しでも以下を確認した。

- `gui.create("win32", false)` と `gui.set_parent()` は成功する。
- 作成直後の `get_size()` は `400×232`、埋め込み時に `request_resize(530, 307)`、
  埋め込み後の `get_size()` は `530×307` を返す。
- `gui.show()` は初回・再呼び出しとも `false` を返す。
  `set_scale(1)`、取得したサイズの `set_size()` 後も結果は同じ。
- 親の表示をGUI作成・表示より後に移しても、shim の表示失敗は変わらない。

[CLAP GUI仕様](https://github.com/free-audio/clap/blob/main/include/clap/ext/gui.h) は
`show()` の成功時に `true` を返すことを要求する。
Shu の内部ソースは確認していないため、失敗を返す内部原因までは断定しない。

## 決定

Windows / CLAP / plugin ID `audio.mikey.Shu` に限定して、`showUI()` が失敗しても、
ホストの親ウィンドウと、その直下の非ゼロサイズの子ウィンドウが実際に可視なら表示成功と扱う。
GUI作成失敗、子ウィンドウのない場合、他の製品・形式は従来どおりエラーを返して親を隠す。
通常の `showUI()` が成功した場合は例外処理を使わない。

固有処理は [shu_ui.h](../../shim/plugin_specific/shu_ui.h) に集約し、
共通の `uh_ui_show()` から呼ぶ。HWNDの検査が必要なため、Rustの固有処理とは別に
ネイティブの `shim/plugin_specific/` を設ける。
依存checkout、UAPMD、プラグイン本体、ユーザー設定・stateは変更しない。

親の表示順序や、取得したサイズをプラグインへ再設定する試行は改善しなかったため撤回した。
サイズ調整は既存のresize通知と `getUISize()` による親ウィンドウの調整を引き続き使う。
Hide UI・閉じるボタンでは親を隠し、再表示時には同じGUIを使う。

## 検証・制限・撤去条件

- 検証対象: Shu CLAP `1.0.0`、Windows x64。
  バイナリSHA256: `c2fc736adb50d18af666054396193bdaf7c6367135ee68775d2da8e4eb3cb3dd`。
- [実機回帰テスト](../../tests/native_shu_ui.py): 初回表示、2秒間の表示維持、
  可視な子GUIが親のclient領域内に収まること、Hide UI、WM_CLOSE、各操作後の再表示。
  既定の `target/shim/out/uapmd_shim.dll` とインストール済み Shu を使用する。
- 自動テストは可視性と座標を確認する。描画内容、ノブ操作、音声との同時動作は未確認。
- Windows CLAPに限定し、版による分岐はしない。将来版の表示失敗も同じ条件なら成功扱いになる。
  可視な子ウィンドウが内部エラー画面である可能性は、この検査では区別できない。
- Shuの更新時には、互換処理を使わない状態で `gui.show()` と開閉・再表示を確認する。
  標準APIだけで正常表示できることが確認できれば、固有処理を撤去し回帰テストは維持する。
- 作者への報告: 未報告。外部送信はしていない。
- ビルド検証は既定の既存キャッシュを使用。新規環境でのビルド再現性は未確認。
- `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、
  `cargo build --release` 成功。`cargo test` は67件成功、実機依存6件は通常実行では除外。
