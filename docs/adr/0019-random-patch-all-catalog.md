# 0019: Random patch を既存 cmrt カタログの7音源へ拡張する

日付: 2026-10-10。状態: 実装済み。7音源の機械検証は通過。release版の人間による聴取・操作確認は未確認。

## 背景と決定

ADR 0014 の Surge 限定候補・準備を置き換える。Surge XT、Dexed、Floe、sforzando、
Six Sines、TyrellN6、Vaporizer2 の CLAP instrument について、shared reader の
PatchRef と PatchBase で候補を解決し、installed instrument の format と厳密 ID を照合する。
同名 patch の音源を混同せず、Dexed の cartridge program suffix を保持する。
全候補 patch から均等に選ぶ既存 choose は維持する。Dexed が候補の大半を占めることも
この配分の結果であり、音源均等モードや自動再抽選は追加しない。

Surge・Vaporizer2・Six Sines・TyrellN6 は検査付き純粋変換、Dexed・Floe・sforzando は
共有 core の既存 loader を一時 renderer で使い、反映後の owned state を返す。
一時 instance の生成から破棄まで worker 内で完結する。GUI・音声デバイス・server は
起動せず、cat の Host・live instance・processor・pointer を worker に渡さない。
既存 busy 制御により scan、復元、manual/random 生成と競合させない。cmrt の生成ロックを
UAPMD と共有することは仮定しない。rescan 後の bundle と適用前の instrument identity も照合する。

## 適用、反映、保存

ADR 0015 の既存 transaction を維持する。停止中は停止を維持し、再生中は選択フレーズの
先頭から再開する。確定 MML と先頭休符、Velocity・CC1 選択、接続 effect の順序・state・
Bypass は変更しない。失敗時は旧 state を復旧し、復旧失敗は unsafe_state と保存抑止へ進む。
成功時は準備に使った state を保存し、音声反映前の live save で上書きしない。
Favorites／History の既存契約は変更しない。

Six Sines は state 反映 block の note-on が消えるため、note 開始まで MAX_BLOCK_FRAMES の
空の演奏区間を置く。TyrellN6 は cmrt の実測した4096 frames・最低4 blocks を使う。
cat は process block を1024 frames以下に制限するので4096 framesで両条件を満たす。
sample rate に応じて duration を計算し、既存 sequencer の開始遅延を使う。
effect chain を高速で空回しせず、通常の音声 callback が実時間で進む。Surge／Floe の
既存100ms遅延は維持し、他音源へ一律に広げない。

## 検証と制約

Stage 1 は実 Dexed の同一 cartridge program 0/1、Floe の二つの Celtic Harp preset、
sforzando TableWarp2.sfz / Airy Bells.ariax を一時 instance で準備して別 UAPMD instance
へ復元し、発音と音声差を確認した。sforzando の保存 state に program 座標と ARIA params
が残ることも確認した。Floe の DLL lifecycle は別 host の所有管理と同一ではないため、
成立を一般化せず、この環境での実測として扱う。

opt-in `native_preparation_cross_host_gate` は各音源の二候補を指定し、worker 準備、
既存 instance への適用、停止／再生維持、適用 state の保存、別 instance 復元と音声差を確認する。
実行結果・patch・環境・待ち時間は handoff ledger に記録する。state bytes の完全一致だけを
音色成功の証拠にしない。人間による音色・GUI・待ち時間の確認は機械検証と区別する。
