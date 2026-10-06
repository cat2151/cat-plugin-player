# Windows VST3 ローダーの補修

## 状態

- 上流への報告: **未報告**。送信用の文面は [UPSTREAM_REPORT.md](UPSTREAM_REPORT.md)。報告後はここに Issue / PR のリンクを追記する。
- 対象: UAPMD `source/remidy/src/vst3/ClassModuleInfo.cpp`。
- 調査時の上流リビジョン: `bc39b50faf2f2b02668de82d3d95f2bdbafd3c34`。
- 検証結果: 下記「今回の検証結果」に記録。

## 原因と補修

上流の `getPluginCodeFile()` は、VST3 バンドルの ABI ディレクトリ内で最初に列挙された項目を返す。
Reason Rack Plugin の `Contents/x86_64-win` には `.vst3` 本体のほか、`cacert.pem`、フォント、サブディレクトリ等があり、証明書を `LoadLibraryW()` に渡すと Windows の「正しくないイメージ」（`0xc000012f`）ダイアログが表示される。列挙順に依存するため、最初の項目が必ずバイナリとは限らない。

Windows のローダーと、このアプリのカテゴリ取得処理で同じ選択処理を使う。

1. バンドル名と同名の通常ファイル（拡張子 `.vst3`）を優先する。
2. 同名ファイルがなければ、ABI ディレクトリの通常ファイルで拡張子 `.vst3` のものが一つだけある場合に採用する。拡張子は大文字小文字を区別しない。
3. 候補なし、候補複数、ディレクトリを読み取れない場合は空のパスを返す。証明書・フォント・補助 DLL・ディレクトリは読み込まない。
4. 単一ファイル形式の VST3 でも、通常ファイルと拡張子を確認する。

バイナリそのものの破損や、そのプラグイン固有の初期化エラーを修復するものではない。

## 適用場所と再 clone

`shim/CMakeLists.txt` がこのディレクトリの `CMakeLists.cmake` を読み込む。Windows の CMake configure 時に `apply.cmake` が上流ソースを読み、適用条件を確認して、`target/shim/generated/windows-vst3-loader/ClassModuleInfo.cpp` に補修済みのコピーを生成する。remidy のコンパイル対象をそのコピーへ切り替える。UAPMD checkout のファイルは変更しない。

同じ適用条件を満たす UAPMD を clone し直しても、configure 時に再適用される。生成済みキャッシュだけに依存した修正ではない。Cargo は補修ディレクトリと対象ソースを監視し、CMake も対象ソースと適用スクリプトの変更時に再 configure する。通常は既定の `cargo build --release` を使う。`UH_SKIP_CMAKE` は既存 DLL を流用するため、補修の適用・検証には使わない。

## 上流変更で適用できない場合

次の条件をすべて満たす場合だけ適用する。リビジョンの完全一致は要求しない。

- remidy のソース一覧に `src/vst3/ClassModuleInfo.cpp` が一つある。
- 想定するヘッダー include 二つ、単一ファイル分岐、最初の項目を返すコードが、それぞれ一回だけ存在する。CRLF / LF の違いは許容する。

条件を満たさなければ、`[cat-plugin-player: windows-vst3-loader] Patch was NOT applied.` とともに対象パス、失敗条件、上流リビジョン（取得できなければ unknown）、ソース SHA256、この説明書へのパスを出して停止する。補修をスキップして壊れたローダーをビルドすることはしない。

1. エラーの対象パスが、意図した `UAPMD_DIR` の checkout か確認する。
2. 表示されたリビジョンとソースを、上記の調査時リビジョンと比較する。ローカル変更も確認する。
3. 上流が同じ不具合を直したのか、それ以外の変更なのかを切り分ける。
4. 未修正なら変更内容に合わせて補修と回帰テストを更新する。検証条件の削除や依存キャッシュの書き換えで迂回しない。
5. 上流修正済みなら、下記の撤去条件を満たしたうえで補修を撤去する。

## 上流対応後の撤去条件

上流修正リビジョンを記録し、リソースが混在する Reason Rack Plugin のバンドル、単一ファイル形式、候補の欠落・曖昧さについて検証する。このアプリが使用する UAPMD に修正が入ったことを確認してから、CMake include、Cargo の補修監視、補修用の生成ソースを撤去する。カテゴリ取得処理で使うヘッダーの参照も合わせて処理し、AGENTS.md の補修一覧を更新する。古い UAPMD もサポートする場合は、無条件撤去せずサポート範囲を先に決める。

## 回帰検証

リポジトリ直下で、同じビルド領域を使う操作は順番に実行する。

```powershell
python patches/uapmd/windows-vst3-loader/test_apply.py
cargo fmt --check
cargo build --release
cmake --build target/shim --config Release --target windows_vst3_binary_test
& ./target/shim/Release/windows_vst3_binary_test.exe
# Reason Rack Plugin がインストール済みの場合:
& ./target/shim/Release/windows_vst3_binary_test.exe "$env:COMMONPROGRAMFILES/VST3/Reason Rack Plugin.vst3"
python tests/native_catalog_smoke.py
cargo clippy --all-targets -- -D warnings
```

適用テストは既定の `target/shim/patch-tests` 内に一時的な入力を作り、終了時に削除する。別の target ディレクトリは使用しない。バイナリ選択テストは、共有ヘルパーと実際にコンパイルされた remidy の `getPluginCodeFile()` を検証する。カタログテストは既存キャッシュを使うため、未登録プラグインの新規探索を検証したことにはならない。

### 今回の検証結果

2026-10-06、Windows x64、上記リビジョンで検証。対象ソースの SHA256 は `2354fe27eec1c1a40b20dc5ce52da54af9c7b3f4b075850b1bd807851bb62e7c`。UAPMD 対象ソースに差分なし。

- `test_apply.py`: 新しい模擬 checkout 二つからの生成、同じ入力への再適用、入力の非変更、上流変更・重複・include 変更・対象欠落時の診断を確認。
- `cargo fmt --check`、`cargo build --release`、`cargo clippy --all-targets -- -D warnings`: 成功。
- `cargo package --list --allow-dirty`: このディレクトリの7ファイルが配布対象に含まれることを確認。既存の `.gitignore` の `uapmd/` は `/uapmd/` に限定し、補修ディレクトリが除外されないようにした。commit / push / publish は実行していない。
- `windows_vst3_binary_test`: 共有ヘルパーとコンパイル済み remidy ローダーを検証。実際にインストールされた Reason バンドルでも `Reason Rack Plugin.vst3` 本体を選択。
- `native_catalog_smoke.py`: 二回の探索で22プラグインを取得。ログ上、両方の探索で Reason バンドルの slow scan を実行し、Instrument / Effect 両方を取得した。今回の Bad Image ダイアログで停止せず完了。

ビルドは既存の依存キャッシュを使用。模擬 checkout からの再適用は確認したが、依存全体を新規 clone した環境でのフルビルドは未検証。

最初のビルドは、既存 ImTimeline キャッシュに UAPMD の `external/patches/imtimeline.patch` を二重適用する既存の configure 問題で停止した。ログ・設定・差分を確認し、依存差分全体が当該パッチと完全一致すること、逆適用可能であることを確認した後、そのパッチだけを逆適用し、既存の configure 手順で再適用して成功した。追加のビルド設定変更や検証のスキップは行っていない。依存ソースは元のパッチ適用済み状態に戻り、失敗時の `.rej` 四つは除去済み。同じ問題が発生しても、由来不明の依存差分を無条件に復元しないこと。
