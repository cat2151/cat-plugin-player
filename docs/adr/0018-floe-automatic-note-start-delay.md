# ADR 0018: Floeの自動演奏開始を暫定100ms遅らせる

状態: 採用（暫定対策）

## 背景と決定

Floeで最初の音が消えるという報告への観察用対策として、主音源が
CLAP `com.floe-audio.floe` の場合だけ最初のフレーズを100ms遅らせる。
名前・vendor・pathでは識別しない。既存Surge XTの待機は維持する。

`session::prepare_voice`は主音源のidentityから待機を決めるため、effectとしての
Floeには適用しない。新規ロード、state復元、Favorite／History／Random patch後の
音声再構築も同じ入口を通る。Sequencerのサンプル時間で待ち、待機中もDSPは進む。
UIやcallbackでsleepせず、先頭音を捨てずにフレーズ全体を遅らせる。
MML先頭休符は100msに加算される。通常のPlayや同一ストリーム内のrestartで
100msを毎回追加しない。

## 検証と限界

exact CLAP IDの識別と他identityの除外を回帰テストする。
既存Sequencerテストは44.1/48kHz、64/256/1024フレームで100ms境界をまたいで
先頭note onと後続時刻が維持されることを確認する。
実機のFloe試聴は未確認。100msは必要最小値や症状の完全解消を実証した値ではない。

## 見直し条件

同じstate／サンプルlibrary／デバイスで冒頭音を繰り返し観察する。
Floe側の対応または待機不要という再現結果が得られたら、待機値の短縮・撤去を検討する。
依存checkoutの変更や新規patchは行わない。外部への報告は未送信。
