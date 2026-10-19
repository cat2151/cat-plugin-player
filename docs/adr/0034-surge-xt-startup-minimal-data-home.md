# ADR 0034: 起動時に復元する Surge XT だけを最小 data home で作る

- 状態: 採用
- 関連: [cat-smf-editor ADR 0012](../../../cat-smf-editor/docs/adr/0012-surge-minimal-data-home.md) / [0008](0008-surge-xt-automatic-note-start-delay.md) / [起動時間の計測](../startup-timing.md)

## 決定

前回の音源が Surge XT（CLAP・VST3）のとき、起動時の復元で作る1件だけを、`SURGE_DATA_HOME` を最小構成の
data home にした状態で生成する。最小構成の作り方と無効化（`CMRT_SURGE_MINIMAL_DATA_HOME=0`）は
cat-smf-editor と共有する `shared/surge_data_home.rs` に従い、cache は
`%LOCALAPPDATA%\cat-plugin-player\cache\surge-data-min-v2\` に置く。

- 環境変数は main thread で生成要求の直前に変え、完了または失敗の時点で元の値に戻す（元が未設定なら削除）。
- 最小構成で作った instance には印を付け、その editor を開く操作は、state を持ったまま通常構成で
  同じ plugin を作り直してから開く。作り直しは手動の `Load` と同じ差し替え経路を使う。
- 準備に失敗したら環境変数を変えずに通常構成で作り、`error.log` に記録する。
- 手動ロード・patch browser・Favorite / History・エフェクト・走査後の復元・`--select-smf`・resident は通常構成のまま。

## 理由

- Surge は instance を作るたびに patch・FX preset・wavetable を走査し、発音までの大半を占める。
  最小構成で生成は約690ms から約70ms になり、起動から最初の信号は約820ms から約215ms になった。
- Surge は instance の constructor ごとに `getenv("SURGE_DATA_HOME")` を読むので、同じ process の中で
  生成ごとに切り替えられる。切り替えている間は走査と patch catalog が保留中のロードを待つため、
  他の生成が最小構成を拾わない。
- 最小構成では Surge の画面の patch・wavetable・FX preset のメニューがほぼ空になる。
  画面を開くのは起動直後の一部の場合だけなので、そのときだけ作り直しの待ち（音が一度止まる）を払う。

## 却下した案

- **cat-smf-editor と同じく process 全体を最小構成にする。** player は Surge の画面を開けるので、
  メニューが常に空になる。
- **editor を開くときだけ別 process で作る。** 画面と演奏の instance が分かれ、音色の往復が要る。
- **手動ロード・patch 適用も最小構成にする。** 作り直しの対象が増える。起動時の1件だけで目的は満たす。

## 結果

- 起動直後に Surge の画面を開くと、音が一度止まり、0.3〜0.4秒ほどで画面が開いて音が戻る。
- 罠: Surge は editor を一度開くと state に editor 専用の項目（`instanceZoomFactor` など）を足す。
  作り直しの前後で state を比べるときは、旧 instance でも editor を開閉してから基準を取る。
- 前回の構成にある Surge XT Effects は通常構成のままで、その生成待ちは残る。
