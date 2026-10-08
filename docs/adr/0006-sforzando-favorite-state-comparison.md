# ADR 0006: sforzandoの保存カウンターと演奏値をFavorite重複判定から除外する

- 状態: 採用
- 対象: sforzando CLAP 2.1.2.4、ID `com.Plogue Art et Technologie, Inc.sforzando`

## 原因と測定

同じ音色を繰り返しfavorite保存すると増殖するというユーザー報告を、独立hostで再現した。
製品のconfig・favorite・stateは読み取りのみとし、同じTableWarp2 stateをメモリ復元した。
何も操作しない連続保存でも、CEGPラッパー内のzlib圧縮XMLにあるSettings・Slot・EffectSlotの`sc`が18→19→20と増えた。
再読込後の保存も21となった。ノート60/velocity100だけでSlot直下に`Param id="131"`が入り、その値は100/127だった。
CC1=100を送ると`Param id="1"`にも100/127が入り、CC120では両値が残った。
このためバイト完全一致では、保存・演奏だけで別favoriteを作ってしまう。

## 決定

既存の[固有処理の入口](../../src/plugin_specific/mod.rs)から[sforzando.rs](../../src/plugin_specific/sforzando.rs)を呼ぶ。
標準state APIが返す不透明な保存state自体は変更しない。CLAP全体への推測による比較は導入しない。
確認済みの`CEGP`、展開サイズ、zlibストリーム、`AriaSave version="1982" productID="1014"`が成立する場合だけXML設定を比較する。

- root直下のSettings・Slot・EffectSlotの数値`sc`を比較から除外する。
- Slot直下の空要素Paramで、属性がid/valueのみ、idが1または131、値が有限の0〜1の場合だけ演奏値として除外する。
- 他の設定、音色名、bank、音量、チューニング、effect設定、GUI設定は保持して比較する。
- 形式不明、破損、未知の版、展開長不一致、余分な圧縮末尾は完全一致へ戻す。展開は1MiBまで。

手動・自動favoriteとも既存snapshotのIDと名前を再利用し、snapshotのバイトは保持する。
同じpluginでも演奏パターンが違えば、既存仕様どおり別favoriteとして保存する。
既存の重複favoriteを自動削除・統合する移行は含めない。

## 検証と限界

単体試験でカウンター・演奏値の差だけを同一と判定し、音色・音量・チューニング・他Param・未知版・破損の差を残す。
実機試験では同一stateの6回の手動／自動保存・再読込・ノート/CC1を通して1件を維持し、改名と元snapshotを保持した。
チューニングを変更してhostへ読込み、再保存した場合は2件になった。

CC1の違い自体を別favoriteとして残す用途と、最後のノートvelocityの違いは区別しない。
保存される元snapshotにはそれらの値も残るため、既存snapshotの内容を正規化して書き換える処理ではない。
VST3版や別ARIA schemaには適用しない。未知の音色・版で再発した場合は実際のstate差を確認してから更新する。
標準APIで安定した音色stateが得られるようになれば、本例外の撤去を検討する。
