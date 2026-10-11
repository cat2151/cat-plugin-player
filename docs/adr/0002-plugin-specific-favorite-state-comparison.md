# ADR 0002: プラグイン固有処理を集約し、Favorite重複判定を限定対応する

- 状態: 採用（実装済み。圧縮部分の設定差を区別できない制限あり）
- 確認: userは「同じ音色なら重複をまとめ、違う音色は履歴として残す」を選択。固有処理をできるだけ少なくし、その有無を可視化するよう指示。

## 何の話か

Auto FavoでTyrellN6の同じ音色が繰り返し追加される。
従来の重複判定はstateのバイト完全一致だったが、実機では演奏だけで圧縮部分が変わった。
製品固有の例外を共通処理に散在させず、導入理由と制限を追跡できるようにする。

## 決定

製品名・plugin IDに依存する本番処理は `src/plugin_specific/` に集約する。
ネイティブのウィンドウ検査が必要な固有処理は `shim/plugin_specific/` に集約する
（[ADR 0004: Shu GUI](0004-shu-clap-editor-show-compatibility.md)）。
共通処理は [mod.rs](../../src/plugin_specific/mod.rs) の入口だけを呼ぶ。
固有処理の追加・撤去はADRに記録し、本ADRを置き換える場合は後続ADRへの参照を残す。
新しい例外を増やす前に、プラグイン形式の標準APIで解決できるか確認する。

## 現在の本番処理：7件

| 対象 | 処理・場所 | 必要な理由 | 適用範囲・撤去条件 |
| --- | --- | --- | --- |
| Shu CLAP (`audio.mikey.Shu`) | GUI表示：[shu_ui.h](../../shim/plugin_specific/shu_ui.h) | 埋め込み成功後も `gui.show()` が `false` を返し、ホストが親を隠す | Windows CLAPで可視かつ非ゼロサイズの子GUIがある場合だけ成功扱い。標準APIで正常表示できれば撤去。詳細はADR 0004 |
| TyrellN6 CLAP (`com.u-he.TyrellN6`) | Favoritesの重複判定：[tyrell_n6.rs](../../src/plugin_specific/tyrell_n6.rs) | ノート・CC1の受信だけでstate末尾の圧縮データが変わり、バイト完全一致では同じ設定が増殖する | 実機確認済みのbuild 16976 / `#Vers=10010` / little endian / `$$$$1924`形式のみ。標準APIで安定した音色stateを取得できるようになれば撤去を検討 |
| Vaporizer2 CLAP (`com.vastdynamics.VAST2`) | Favoritesの重複判定：[vaporizer2.rs](../../src/plugin_specific/vaporizer2.rs) | stateの読み込み・保存でMSEGの時間・座標に微小な数値差が生じ、同じ設定が増殖する | `VC2!`ラッパーと`VASTVaporizerParamsV2.20000`の確認済み形式のみ。安定したstateが取得できるようになれば撤去を検討 |
| Floe CLAP (`com.floe-audio.floe`) | sweep中のFavorite重複判定：[floe.rs](../../src/plugin_specific/floe.rs) | 自動CC1がMacro 1とpreset変更フラグに保存される | Floe 2.0.2 / state schema 30、CC1→Macro 1だけの割り当てを確認。詳細は[ADR 0009](0009-sweep-cc1-favorite-comparison.md)。Vaporizer2のsweep対応も同ADR参照 |
| sforzando CLAP (`com.Plogue Art et Technologie, Inc.sforzando`) | Favoritesの重複判定：[sforzando.rs](../../src/plugin_specific/sforzando.rs) | 保存だけで`sc`が増え、ノート・CC1の演奏値もstateに残る | `CEGP`とARIA schema 1982/1014の確認済み形式のみ。CC1と最後のvelocityは別音色としない。詳細と撤去条件は[ADR 0006](0006-sforzando-favorite-state-comparison.md) |
| Surge XT CLAP / VST3 | Favoritesの重複判定：[surge_xt.rs](../../src/plugin_specific/surge_xt.rs) | editor拡大率と演奏中のCC1値がstateに残る | `sub3` / patch revision 24、実機確認済み1.3.4。詳細と撤去条件は[ADR 0007](0007-surge-xt-favorite-state-comparison.md) |
| Six Sines / TyrellN6 CLAP | state復元後のnote開始待ち：[patch_settling.rs](../../src/plugin_specific/patch_settling.rs) | Six Sinesは反映blockのnote-onを捨て、TyrellN6は反映に複数blockが必要 | Six Sinesは最大1block分、TyrellN6は4096frames・最低4blocks。sample rateに応じて既存sequencerで待つ。空回しなしで冒頭音・選択音色が安定すれば撤去を検討。詳細は[ADR 0019](0019-random-patch-all-catalog.md) |

### Vaporizer2の判定と制限

現在のMSEG許容幅は時間`0.00011` ms、座標`0.000001`。以下の`1e-9`と検証数は初回実装時のもの。現在の根拠・制限は[ADR 0010](0010-vaporizer2-mseg-recalculation-drift.md)参照。

- 保存済み6件はCLAP・`steps`で、XML構造と他の設定が一致した。差はMSEGの15〜22項目、最大絶対差は約`4.0011e-10`だった。
- ローカル実機では保存stateの読み込み直後に14項目の差を再現した。連続保存、続く3回の再読み込み、ノート・CC1後の保存では差を確認しなかった。TyrellN6の圧縮部分を除外する方式とは原因・対応を分ける。
- `clap.state-context/2`拡張はローカル実機で提供されていなかった。標準の音色用contextによる代替取得は使えない。
- XMLを解析し、root/version、要素構造、属性名・順序、他の属性値とテキストの一致を要求する。JUCEの長さヘッダは実際のXML長と一致し、末尾は1バイトのNULであることを確認する。
- `VASTvaporizer2/chunkData/msegData0`〜`msegData4`の`m_fAttackTimeExternalSet`、`m_fDecayTimeExternalSet`、`m_fReleaseTimeExternalSet`と、その直下の`msegPoint1`・`msegPoint2`の`xVal`だけ、有限数値の絶対差`1e-9`以下を同一と扱う。未知の版・形式・対象位置、不正データはバイト完全一致に戻す。
- 許容幅以内の意図的な編集も同一と扱う制限がある。全浮動小数値への許容差、相対許容差、MSEG全体の除外は使わない。
- 元のstateは変えずに保存・復元する。手動・自動Favorite保存時に一致する既存Favoriteがあれば、名前・ID・保存済みstateを維持して再利用する。起動時の一括削除は行わない。調査・テストではユーザーの保存済みFavoritesを変更していない。
- 解析には既存の依存グラフにある`quick-xml 0.41`を直接依存として使用する。
- 回帰テストは微小差の重複整理、対象外属性・MSEG座標・時間の変更保持、未知形式・不正ヘッダを確認する。実機テストでは一時ライブラリで保存済み6件を1件にまとめ、load/saveと演奏後の比較、元バイトの保存、Master Tuneを440から432に変えた別音色の保持を確認した。
- 追加後の検証：通常テスト63件成功（実機依存5件は通常実行では除外）、Vaporizer2の実機テスト1件成功、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`cargo build --release`成功。既定の既存ビルドキャッシュを使用し、新規環境でのビルド再現性は未確認。

### TyrellN6の判定と制限

PCoreの確認済み`UI_op=9/10`とテキスト末尾の空行は比較から除外する。
以下の全テキスト完全一致は初回実装時の記録。現在の根拠・制限は[ADR 0011](0011-tyrell-n6-ui-operation-favorite-comparison.md)参照。

- プラグイン形式・ID・役割・演奏パターンの一致は従来どおり必要。
- メタデータ、プリセット名、テキストに記録された全設定を完全一致で比較する。
- 確認した形式の圧縮部分は比較から除外する。保存・復元するstateは圧縮部分を含む元データのまま。
- 未確認の版、別製品・形式、不正なデータは、従来のバイト完全一致に戻す。
- 圧縮部分の独自フォーマット全体を解読したわけではない。圧縮部分だけに保持される設定の差は、この例外では区別できない。
- 手動・自動Favorite保存時に一致する既存Favoriteがあれば再利用する。既存の重複を起動時に一括削除することはない。
- 同じ設定の保存で名前と自動番号が変動しないよう、既存Favoriteの名前・ID・保存済みstateを維持する。手動で付けた名前も維持する。

## 見送ったもの

| 案 | 理由 |
| --- | --- |
| バイト完全一致を継続 | 音色の設定を変えずに演奏した場合も重複を見逃す |
| Auto Favoをプラグインごとに最新1件にする | 異なる音色の履歴を残すというuserの選択に合わない |
| 全プラグインの圧縮・バイナリ部分を除外 | 必要な設定を区別できなくなる恐れがあり、形式も共通ではない |
| CLAPの音色用state contextを使う | 確認したTyrellN6版は `clap.state-context/2` を提供していない |

## 検証した根拠

ローカルのTyrellN6 CLAP build 16976で確認：

1. 既存7件（手動1件、自動6件）はテキスト設定が完全一致し、圧縮部分だけ異なっていた。
2. 同じstateをロードし直して保存するだけならバイト完全一致した。
3. ノートとCC1を処理すると、テキスト設定は一致したまま圧縮部分が変化した。
4. `clap.state-context/2`拡張は取得できなかった。音色用stateを標準のcontext指定で取り出す代替策は、この版では使えない。

## テストだけの製品依存処理

本番の例外数には含めない。汎用経路を実機検証する対象指定やアサーション。

| 対象 | 場所 | 用途 |
| --- | --- | --- |
| TyrellN6 | [tyrell_n6_tests.rs](../../src/plugin_specific/tyrell_n6_tests.rs) | ノート・CC1後の重複整理と、設定変更の保存を実機検証 |
| Vaporizer2 | [vaporizer2_tests.rs](../../src/plugin_specific/vaporizer2_tests.rs) | MSEG微小差の限定比較、既存Favoritesと読み込み・保存後の重複整理、Master Tune変更の保存を一時ライブラリで実機検証 |
| TyrellN6 | [native_startup_smoke.py](../../tests/native_startup_smoke.py) | editor子windowの再描画確認 |
| Vaporizer2 | [session_tests.rs](../../src/session_tests.rs) | JUCE XML stateを編集し、保存・復元を確認 |
| Dexed / Dragonfly Hall Reverb / Surge XT | [favorites_tests.rs](../../src/favorites_tests.rs)、[routing_tests.rs](../../src/routing_tests.rs) | instrument/effect切り替え、Favorites、ルーティングの実機検証 |

## プラグイン形式への対応

CLAP / VST3 / LV2等の形式別処理は、製品固有処理とは区別する。
例えば `shim/uapmd_shim.cpp` のLV2文字列終端対応や
`shim/windows_vst3_binary.h` のWindows VST3本体選択は形式単位の処理であり、上の本番例外数には含めない。

## 検証と再発時の入口

実機回帰テストではノート・CC1後のstateをFavorite保存しても1件にまとまり、
Cutoffの編集後は別項目として残った。復元用ファイルのバイトは取得したstateと一致した。
TyrellN6対応時には通常テスト61件、fmt、Clippy、リリースビルドも成功した。

再発時はplugin ID・形式・build・stateヘッダが適用範囲内か確認し、
テキスト設定と圧縮部分のどちらに差があるかを調べる。
圧縮部分だけに必要な設定差が見つかった場合は、その内容を解明して判定を修正するか、
本例外を撤去する。未確認の版へ適用範囲を広げる前に実機検証とADRの更新を行う。
