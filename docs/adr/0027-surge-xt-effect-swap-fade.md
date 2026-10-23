# ADR 0027: Surge XT Effectsの切り替えをフェードで挟む

- 状態: 採用（50msは暫定値）
- 決定者: user（fadeoutとfadeinの両方を入れ、Surge XT Effectsが関わるeffect切り替えだけに限定すること）

## 決定

Browse effectsでeffectを置き換えるか追加するとき、置き換え前か後のeffectがCLAP `org.surge-synth-team.surge-xt-fx` なら次のようにする。

- 旧音声ストリームを止める前に、出力を50msで0まで下げる。0のまま2 callback分を書き終えるのを待ってから止める。callbackが止まっている場合は300msで待つのをやめる。
- 切り替えが終わるまでに作る音声ストリームは、出力を50msで0から上げる。

判定は `src/plugin_specific/surge_xt.rs`、gainのrampは `src/audio_fade.rs` にある。音色の変更、音源の変更、Favorites、MML、終了、他のeffect同士の切り替えではフェードしない。

## 理由

- 音声ストリームを止めると、鳴っている途中の波形が0へ不連続に落ちる。
- 音源のinstanceは、stream停止時のrelease tailを保ったまま次のchainで処理を再開する。新しいSurge XT Effectsのdistortionを通ると、このtailが1サンプル目からfull scale近くで出る。実測（Surge XT初期音色、48kHz、Guitar arpeggioの途中で切り替え）では、`Distortion/Crunchy` で冒頭0〜5msのpeakが1.387、`Distortion/Deep 1` で5〜10msのpeakが1.610だった。fadeinを入れるとそれぞれ0.344、0.409まで下がった（0〜30msの最大）。
- 切り替え中の無音は、新しいinstanceの生成とstate読み込みを待つ間に元から生じている。fadeoutの待ちはそれに最大約70ms（rampの50msと2 callback分）を足すだけで済む。

## 却下案

- 全停止にフェードをかける案: 音色の変更などすべての操作に最大約70msの遅れが加わる。userの判断で、症状の出るSurge XT Effectsの切り替えだけに絞った。
- fadeoutだけにする案: 上の実測どおり、distortion系では切り替え後の冒頭にも不連続が残る。
- 音源のtailを消す案（切り替え時に音源をreset・再生成する）: 音源の状態と開始遅延に影響し、変更範囲がshimまで広がる。

## 結果と制限

- fadeinの間も音源のtailは小さく残り、約30msで消える。
- VST3版のSurge XT Effectsは対象外とする（Browse effectsが扱うのはCLAPだけ）。
- 他の歪み系effectで同じ症状が出たら、判定を広げるか全停止へ広げるかを再検討する。
