# ADR 0003: 依存ライブラリの補修をpatchesに集約し、ビルド用コピーへ適用する

- 状態: 採用（Windows VST3ローダーで実装済み）
- 記録範囲: 現在の `patches/` の構成と適用方式。新しい補修の導入を承認するものではない。

## 何の話か

UAPMDのWindows VST3ローダーは、バンドルのABIディレクトリで最初に列挙された項目を
プラグイン本体として返していた。Reason Rack Pluginには証明書やフォントも同居しており、
`cacert.pem` を `LoadLibraryW()` に渡すと「正しくないイメージ」（`0xc000012f`）で探索が止まる。

上流修正を取り込むまでローカル補修が必要だが、依存checkoutや生成済みキャッシュへの手作業の修正は、
再cloneで失われ、適用状態や上流との差分を追跡しにくい。
補修の理由、処理、検証、撤去条件をリポジトリに残し、同じ条件のソースに再適用できるようにする。

## 決定

- 依存ライブラリへのローカル補修は `patches/<依存名>/<補修名>/` にまとめる。
  UAPMDでは `patches/uapmd/<補修名>/` に適用処理、回帰テスト、説明書、作者向け報告文を置く。
- 依存checkoutを直接書き換えず、既定の `target/shim` 内に補修済みソースを生成して
  コンパイル対象を切り替える。別のtargetディレクトリは作成・使用しない。
- 補修前に対象と想定するコードの存在・出現数を検証する。
  条件が一致しなければ、補修名、対象、失敗条件、取得可能な上流リビジョン、説明書の場所を示して停止する。
  上流修正済みの可能性も調査し、黙ってスキップしたり検証条件を削除して通したりしない。
- 説明書に原因、適用条件、検証した上流リビジョン、再適用の仕組み、失敗時の調査手順、
  撤去条件、上流への報告状態と報告済みの場合のリンクを残す。
  作者向け報告文の作成は外部送信の許可として扱わない。
- 補修の追加・撤去時は `AGENTS.md` の補修一覧を更新する。
  導入前には目的、適用場所、checkoutへの影響、再clone・既存キャッシュ・上流更新時の挙動を説明し、
  必要な変更範囲の合意を得る。

## 決定時点の補修：1件

| 対象 | 場所 | 適用範囲・根拠 |
| --- | --- | --- |
| UAPMD / remidyのWindows VST3ローダー | [windows-vst3-loader](../../patches/uapmd/windows-vst3-loader/README.md) | Windowsの `source/remidy/src/vst3/ClassModuleInfo.cpp`。検証した上流リビジョンは `bc39b50faf2f2b02668de82d3d95f2bdbafd3c34` |

### 現在の適用経路

[shim/CMakeLists.txt](../../shim/CMakeLists.txt) が補修の
[CMakeLists.cmake](../../patches/uapmd/windows-vst3-loader/CMakeLists.cmake) を読み込む。
Windowsのconfigure時に、remidyのソース一覧に対象が一つだけあることを確認し、
[apply.cmake](../../patches/uapmd/windows-vst3-loader/apply.cmake) で二つのincludeと
単一ファイル分岐、最初の項目を返すコードがそれぞれ一回だけ存在することを検証する。
CRLF / LFの違いは許容し、上流リビジョンの完全一致は要求しない。

生成先は `target/shim/generated/windows-vst3-loader/ClassModuleInfo.cpp`。
remidyの元ファイルをコンパイル対象から外して生成コピーへ差し替える。
[build.rs](../../build.rs) は補修ディレクトリと上流の対象ソースを監視し、
CMakeも対象ソースの変更を再configureの対象とする。
同じ適用条件を満たすcheckoutでは、再clone後もconfigure時に再生成される。
`UH_SKIP_CMAKE` は既存DLLを流用するため、補修の適用・検証には使わない。

### Windows VST3の選択規則

[windows_vst3_binary.h](../../patches/uapmd/windows-vst3-loader/windows_vst3_binary.h) を
補修済みローダーと [plugin_catalog.cpp](../../shim/plugin_catalog.cpp) のカテゴリ取得処理で共有する。

1. バンドルと同名の通常ファイルで、拡張子が `.vst3` のものを優先する。
2. 同名ファイルがなければ、ABIディレクトリの通常ファイルで `.vst3` が一つだけある場合に採用する。
3. 候補なし、候補複数、列挙エラーでは空のパスを返す。拡張子の大文字小文字は区別しない。
4. 単一ファイル形式でも、通常ファイルと拡張子を確認する。

これはWindowsの形式単位の補修であり、Reason専用の製品名判定ではない。
macOS / Linuxのローダー選択には適用しない。バイナリ破損やプラグイン初期化エラーは修復しない。
製品固有処理の集約を扱う [ADR 0002](0002-plugin-specific-favorite-state-comparison.md) とは対象を分ける。

## 見送ったもの

| 案 | 理由 |
| --- | --- |
| 依存checkoutを直接編集する | 再cloneで失われ、依存のローカル変更と補修の適用状態が混ざる |
| 生成済みキャッシュだけを手作業で直す | 新しいビルド領域で再適用できず、リポジトリから修正を再現できない |
| 上流リビジョンだけで適用可否を決める | 同じ対象コードを持つ別リビジョンにも適用できるよう、コードの条件を検証する。リビジョンは調査の根拠として記録する |
| 条件不一致時に補修をスキップする | 既知の不具合を残したままビルドが成功し、適用失敗を見逃す |
| Reason専用のパスを埋め込む | 不具合はWindows VST3の本体選択にあり、リソースを含む他のバンドルにも起こり得る |

## 影響と制約

補修の処理と根拠を依存checkoutから独立して管理でき、再適用をテストできる。
一方、上流のコードやCMakeのソース一覧が変わるとビルドが停止し、補修の更新または撤去の判断が必要になる。
文字列の出現条件は上流コードの意味全体を保証しないため、条件が一致しても上流更新後の回帰検証は必要。

本ADRは、依存全体のバージョン固定や新規環境でのフルビルドの再現性を保証しない。
生成ソースだけを保存して補修処理の代わりにせず、適用スクリプトと共有ヘルパーも配布対象に含める。

## 検証した根拠と再発時の入口

補修の説明書に記録された検証では、次を確認している。
本ADRの追加に際してビルドや実機検証を再実行したものではない。

- [test_apply.py](../../patches/uapmd/windows-vst3-loader/test_apply.py):
  二つの新しい模擬checkout、再適用の同一結果、入力の非変更、上流変更・重複・include変更・対象欠落時の診断。
- [binary_test.cpp](../../patches/uapmd/windows-vst3-loader/binary_test.cpp):
  共有ヘルパーとコンパイル済みremidyの選択処理。インストール済みReasonバンドルの本体も選択。
- [native_catalog_smoke.py](../../tests/native_catalog_smoke.py):
  二回の探索で22プラグインを取得し、ReasonのInstrument / Effectを取得。Bad Imageダイアログで停止せず完了。
- fmt、Clippy、リリースビルドが成功し、補修の7ファイルがCargoの配布対象に含まれることを確認。

フルビルドは既存の依存キャッシュを使用した。模擬checkoutでの再適用確認を、
依存全体を新規cloneした環境でのフルビルド成功とは扱わない。
具体的な実行コマンドと既存ImTimelineキャッシュで発生したconfigure問題の経緯は、補修の説明書を参照する。

適用失敗時は診断の対象パス、リビジョン、SHA256とcheckoutのローカル差分を確認し、
上流修正か別の変更かを切り分ける。由来不明の依存差分を上書きして通さない。

## 撤去条件

使用するUAPMDに上流修正が入ったことを確認し、修正リビジョンを記録する。
リソースが混在するReasonバンドル、単一ファイル形式、候補の欠落・曖昧さを検証してから、
CMakeの補修include、Cargoの監視、生成ソース、カテゴリ取得の共有ヘルパー参照を整理し、補修一覧を更新する。
古いUAPMDもサポートする場合は、その範囲を先に決めてから撤去を判断する。

現在の上流報告は未報告。
[UPSTREAM_REPORT.md](../../patches/uapmd/windows-vst3-loader/UPSTREAM_REPORT.md) は送信用の草稿として保持する。
