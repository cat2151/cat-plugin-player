# コマンドパレットの参照仕様と検証

参照元は cat-smf-editor `d000ee5cffef46c0af143ccaeeaa0488efa8156e` の
`src/view/palette.rs`、`palette/history.rs`、`selection_history.rs`、
`command_row.rs`、`commands.rs` と関連テスト。
参照元を更新するときは、以下の対応と差異も確認する。

| 期待する挙動 | 参照元の根拠 | playerの担当 | 検証条件 |
|---|---|---|---|
| 実行不能な操作を候補から除外 | commands::candidates / 各collect | catalog / routing | 非表示・件数に含まない・キー移動の対象外 |
| 同一検索の履歴、全体履歴、定義順の優先順位 | selection_history::rank / palette::filter | history / filter | 検索vでA、別検索でBを実行後、vはA優先、空検索はB優先 |
| 検索文字列は履歴キーとして原文を使う | selection_history::record | history | 大文字小文字・空白の異なる検索を混同しない |
| 成立した実行だけ履歴に記録、再起動後に復元 | palette::end / history::load/save | history / begin・end | 再生成したContextでも同順位、取消・実行不能は更新なし、読めない履歴を上書きしない |
| テーマ配色・等幅キー文字・ラベル隣のキー表示 | command_row::action/width | row | 同じStyle・候補・選択で描画を比較 |
| クリックで選択しEnterで実行 | palette::end | view / end | クリックだけでは実行せず、次のEnterで選択した操作だけを実行 |
| 入力欄・件数・候補を表示 | palette::end / list_navigation::position | view | 独自見出し・常設操作説明・検索ヒントを追加しない |
| 上寄せ中央、絞り込みで幅・入力欄位置を変えず高さを伸縮 | palette::end / feedback tests | view / layout_tests | 全件→1件→0件→全件、小画面、再表示 |
| 現画面と遷移可能な他画面の機能を重複なく表示 | commands::candidates / ADR 0019 | routing | 現画面の同機能を優先、他画面の閉じるを除外、実行先にfocusを移して留まる |
| 入力・確認中の内容と実行対象を保護 | modal / commands::execute | begin / routing | IME確定Enter、背景入力、二重実行、状態変更、SMF選択中の画面移動 |

外観・クリック・画面横断も参照元に合わせることはuser承認済み。
画面名を含む表示ラベルも検索対象にする。
操作名・検索語・キー割り当て・音色やeffectの処理はplayerの既存機能を使う。
参照元のSMF編集機能自体をplayerに追加するものではない。

履歴はplayerのconfig.tomlと同じdirectoryのcommand-palette-history.jsonへ保存し、
音色state、config.toml、status.jsonの形式は変えない。
常駐起動ではmain windowは編集対象を持たないため遷移先にしない。
常駐browserはeditorから渡された対象に属するため、別browserへの切り替えは
editor側の新しいOpen要求を必要とする。対象を持たない別browserの操作も候補にしない。
入力・確認で移動できない画面は遷移先から除外する。
mainのPlay / stopは通常音色とDrum kitで共通の候補・Space処理を使い、
通常音色ではSequence、Drum kitではメイン画面専用previewを再生・停止する。
Drum kit適用中はSequence選択・強弱・変調・MML編集の候補を除外する。
Drum kits画面の試聴操作は別画面の操作として維持し、通常音色へ戻ると通常の候補を復元する。
SMF選択中の再生契約は変更しない。

画面移動はplayerの既存のopen/close処理を使い、SMF選択のbrowser_onlyを
移動先に切り替えてから閉じる。移動を「保存して戻る」操作として扱わない。
実行直前に対象・利用可否を再検証し、既存の入力handlerが受理した操作だけを
履歴に記録する。非同期の音色ロードの完了を履歴の保存条件にはしない。

一致の対象は上表の共通仕様。playerへの適用に必要な操作名・起動モードの条件は
上記のとおり区別する。画面resize時にも表示範囲を守る既存の挙動は維持し、
開いている間でも画面サイズが変わった場合だけ幅・中央位置を測り直す。
