# ADR 0044: JC303のsequencer同期をHostからInternalへ切り替える

状態: 採用

- 決定者: user（JC303だけを切り替える案2を、全pluginに効く再生中フラグの変更より選んだ）
- 症状の出どころ: user報告（Seq Sync ModeがInternalでないと、送ったnoteが鳴らない）

## 背景

shimは全pluginへ常に再生中（`isPlaying(true)`）を渡す。JC303はplugin版の初期値が
Host同期で、hostの再生開始を合図に内蔵sequencerを走らせる。走っている間は外から来た
MIDI noteを無視する。そのため、新しいinstanceや同期設定がHostのstateでは、送った
noteでなくJC303自身のpatternが鳴る。Midi Clock同期はMIDI Startを待つだけで、この
appはMIDI Startを送らないため、noteはそのまま鳴る。

## 決定

主音源がCLAP `co.midilab.JC303`のとき、`session::prepare_voice`がchainを作る前に
stateを読み、`seqSyncMode`がHostならInternalへ書き換えて読み込み直す。新規ロード、
state復元、Favorite／History／Browse patchesの後の音声再構築は、どれもこの入口を通る。
値は同じ長さ（`1.0`→`0.0`）で置き換え、JUCEのbinary wrapperの長さ欄を保つ。
Internal・Midi Clock・値の無いstateは変えない。

## 却下案

- shimが再生中を渡すのをやめる: 原因そのものは消えるが、全pluginの挙動が変わる。
  tempoに同期するLFOやdelayへの影響を確かめる必要があり、見返りが小さい。
- 画面での注意表示: 気づいても、毎回自分で切り替える手間が残る。

## 結果

罠: 一度でもHost同期のまま処理したinstanceは、stateでInternalに戻しても内蔵sequencerが
止まらない。このため、切り替えは音声処理の前に行う必要がある。JC303の画面で
演奏中にHostへ切り替えた場合は、次にchainを作り直すまで送ったnoteは鳴らない。
切り替えたstateが保存されるため、Hostのまま保存されたFavoriteと現在の音は、
同じ音でもbyte比較では別物として扱われる。

回帰テストは2つある。1つは、識別と属性順と値の範囲を見る単体テスト。もう1つは、
実物のJC303で新規instanceがHostで始まることと、切り替え後の音が新規ロード時と
一致することを見る`real_jc303`（`#[ignore]`）。
