# ADR 0028: Surge XT Effectsのpresetは、事前に作ったstateを`.srgfx`のhashで引く

- 状態: 採用
- 決定者: user。preset単位のstateをhashで引くこと、cat-music-patternsに置いて人間がcommit・pushすること、生成をcatの開発者用exampleにすること、表に無いpresetを候補から外すことを承認

## 決定

- Browse effectsでCLAP `org.surge-synth-team.surge-xt-fx` のpresetを選んだときは、一時instanceを作らない。
  `.srgfx` のSHA-256をkeyに、cat-music-patternsの `surge-xt-fx-states.json` からstateを引いて、そのまま新しいinstanceに読ませる。
- 表は起動ごとにeffect catalogのworkerの中で取得し、`%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt-fx-states.json` に保存する。
  取得や検査に失敗したら前回のファイルを使う（[ADR 0025](0025-surge-xt-patch-catalog-without-cmrt.md)と同じ `surge_patch_catalog::refresh`）。
- 表に無いpresetはeffect catalogの候補から外し、外した件数をstatusとerror.logに出す。表そのものが無ければ、Surge XT Effectsのpresetをすべて外す。
- 表は `cargo run --release --example export_surge_fx_states -- <output.json>` で作る。stateを作る関数は、他のeffectの準備と同じ `surge_fx_states::prepare_state` を使う。

## 理由

- 以前は、presetを選ぶたびにworker threadで一時instanceを作り、presetを読ませてstateを保存していた。CLAPではinstanceの生成と破棄はmain-thread限定である。
  Surge XT Effects（JUCE）の一時instanceをworkerで破棄している間に、main threadのWin32 messageがSurgeのwindowへ届くと、
  `std::bad_function_call` が投げられてprocessが落ちる（`0xc0000409`）。同じ落ち方のdump3つで、この2つのthreadの位置が一致した。
- `.srgfx` は実値を持ち、stateは0..1に正規化した値を持つ。正規化に使う範囲はpluginに聞かないと分からないので、cmrtはpresetを読むたびにpluginで範囲を測っている。
  事前に作ったstateならpluginを動かさずに済み、出来上がるstateも今のpluginの出力と同じになる。
- hashで引けば、Surge XTの更新でpresetの中身が変わったことを「表に無い」として検知できる。
- 生成は一時instanceをmain threadで1つずつ作り、毎回同じ出力になる。stateはsample rateに依らない（44.1kHzと48kHzで出力が一致した）。

## 却下案

- 一時instanceの生成と破棄をmain threadへ移す: 準備の間UIが止まる。止まる長さを測る前に、Surge XT Effectsについてはinstanceを作らない案をuserが選んだ。
- effectの種類ごとの範囲だけを配り、catで正規化する: catで組み立てたstateが、pluginの出力と一致するかを保証できない。Surge XTの更新で範囲が変わっても検知できない。
- cmrtの `build-patch-catalog-cache` で生成する: cmrt本体はこの表を使わない。catの準備と同じ関数で作れば、出力が一致することを保証しやすい。
- 表に無いpresetを候補に残し、選んだときに失敗を出す: 選べないpresetが一覧に並ぶ。

## 結果と制限

- Surge XT 1.3.4のfactory preset 291件すべてのstateを作れた。表は約695KB。
- 表はSurge XT 1.3.4から作ったもの。presetの中身が変わった版では、その分が候補から外れる。表の作り直しは作者が手で行う。
- 作者のuser preset（Documents配下）は、もともとeffect catalogの走査対象ではないので対象外。
- Dragonfly・Shu・TONE3000・Voyageの準備は、従来どおりworker threadの一時instanceで行う。
