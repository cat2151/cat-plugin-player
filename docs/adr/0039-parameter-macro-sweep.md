# ADR 0039: Vaporizer2・Six Sines・FloeのMacroをCLAP parameterで自動変化させる

- 状態: 採用
- 決定者: user（3音源のMacroをSurge XTと同じMacro欄に載せること、欄を出す条件はpatchがそのMacroを使っているかだけにすること、CCではなくCLAP parameterを直接送ること、使用判定をeditorと共用すること、FloeのCC1で動くMacroを欄に出さないこと、3音源のFavorite重複判定でMacroの値を無視すること）
- 関連: [ADR 0036](0036-surge-xt-macro-sweep.md)（Macro欄）、[ADR 0037](0037-direct-parameter-automation.md)（parameterの送り方）、[ADR 0009](0009-sweep-cc1-favorite-comparison.md)（本ADRで置き換えた比較）、[editor ADR 0015](../../../cat-smf-editor/docs/adr/0015-vaporizer2-macro-edit.md)・[editor ADR 0016](../../../cat-smf-editor/docs/adr/0016-six-sines-floe-macro-edit.md)

## 決定

Macro欄（[ADR 0036](0036-surge-xt-macro-sweep.md)）に、次の3音源（CLAPのみ）の使用中のMacroを載せる。欄を出す条件は、patchがそのMacroを使っているかだけとする。

| 音源 | CLAP id | 欄 | 動かすparameter | 値域 |
| --- | --- | --- | --- | --- |
| Vaporizer2 | `com.vastdynamics.VAST2` | Custom modulator 1〜4 | `Custom modulator N` | 0〜1 |
| Six Sines | `org.baconpaul.six-sines` | Macro 1〜6 | `Macro N Level` | -1〜1 |
| Floe | `com.floe-audio.floe` | Macro 1〜4 | `Macro N` | 0〜1 |

選択肢（`-`・0・127・sweep・ドラッグした値）・Macro番号ごとのsweep周期・`M`キー・sessionへの保存・patch変更時に固定値を`-`へ戻す動作は、Surge XTのMacroと共通の経路をそのまま使う。
欄の0〜127は、parameterの値域へ線形に写して送る。CCと同じ128段階なので、表示・ドラッグ・周期の仕組みを変えずに使える。
見出しは、patchがMacroに付けた名前、無ければ「Custom modulator N」（Vaporizer2）・「Macro N」（Six Sines・Floe）。CCで送らないのでCC番号は付けない。

Surge XTとTyrellN6は今のCC送信のまま変えない。
[ADR 0036](0036-surge-xt-macro-sweep.md)の「プラグインパラメータを直接動かす方式は採らない」はSurge XTについての決定で、Surge XTでは既定のCC 41〜48でMacroが動く。
この3音源は、patchのMacroに既定で割り当てられたCCが無い（Floeは工場出荷時のCC1→Macro 1だけ）。CCで動かすには音源ごとのCC learnを読む必要があり、learnしていないMacroは動かない。

## 使用判定

判定は[shared/instrument_macros.rs](../../shared/instrument_macros.rs)にあり、editorも`#[path]`で同じファイルを読む（解析を二重にしない）。

- Vaporizer2: mod matrixの行の音源が`CustomModulatorK`ならMacro Kを使用中とする。名前・値の位置・polarityは[editor ADR 0015](../../../cat-smf-editor/docs/adr/0015-vaporizer2-macro-edit.md)。
- Six Sines: stateのXMLで、各nodeの3つのmodulation slotのMod Sourceが400〜405（Macro 1〜6）または410〜415（そのMacroの変調出力）で、対になるMod Targetが0でなければ使用中とする（上流`src/synth/macro_usage.cpp`の`computeMacroUsage`と同じ）。名前は`macroName`で、既定の`Macro N`は無名。値は-1〜1なので、`-`の表示値は-1→0、1→127へ写す（SurgeのbipolarなMacroと同じ）。
- Floe: state v30を先頭から読み、Macro定義の割り当て先が1つ以上なら使用中とする（上流`MacroDestinationList`）。名前はMacro定義の名前で、既定の`Macro N`は無名。

FloeのMacroのうち、stateのCC割り当てにCC1があるもの（工場出荷時はMacro 1）は欄に出さない。CC1欄とparameter送信が同じMacroを取り合い、値が飛ぶため。

## 送信

parameterの準備（`prepare_parameters`）とbindingの解決は、音声rendererが止まっている間に、voiceを作るときに行う。3音源とも、hostからのstate読み込みでは`params.rescan`を出さない。アプリのpatch変更は音声を止めてstateを読み込み、voiceを作り直すので、bindingはその時に1回取れば足りる。

欄が選ばれている間は、block（最大1024 frame）ごとに今の値を、そのblockのprocessの直前に送る。sweepの1段（約23 ms以上）はblock長（48 kHzで約21 ms）より長いので、block単位で足りる。
Floeは、stateを読み込んだ直後しばらく実時間が経つまでparameterのeventを捨てる。変化したときだけ送ると固定値が届かないことがあるため、送り続ける。

音声callbackで送信を拒否されたら、そのvoiceはMacroのparameter送信だけを止めて鳴り続け、statusに「Sending macro parameters failed (code); macros stopped」を出す（error.logにも残す）。[ADR 0037](0037-direct-parameter-automation.md)の「失敗を黙って捨てない」を守りつつ、Macroのせいで音を止めないため。voice作成時のbinding失敗は、voice作成の失敗として扱う。

patchの読み直しの時期はTyrellN6と同じく、アプリでpatchを変えたときと選択を`-`へ変えたとき（[ADR 0038](0038-tyrell-n6-control-ab-sweep.md)）。

## Favoriteの重複判定

3音源とも、Macroの値だけが違うstateは同じFavoriteにする（Surge XTの[ADR 0007](0007-surge-xt-favorite-state-comparison.md)と同じ）。それ以外の値の違いは別Favoriteのまま。

- Vaporizer2: root直下の`PARAM`で、idが`m_fCustomModulator1`〜`4`の`text`（有限の0〜100）を無視する。MSEGの微小差の扱い（[ADR 0010](0010-vaporizer2-mseg-recalculation-drift.md)）はそのまま。
- Six Sines: `<patch><params><p>`で、idが`Macro N Level`（40000+250·(N-1)）の`v`（-1〜1）だけを無視する。`macroName`などほかは完全一致。
- Floe: state v30のparameter表からid 101〜104のf32（有限の0〜1）と、stateの末尾の「presetから変更済み」フラグ（0/1）を無視する。Macroを動かすとこのフラグも1になり、比べるとMacroの値だけが違う音色が別Favoriteになるため。フラグは音に影響しない。CC割り当て・Macroの割り当て先と量・長さは完全一致が必要。

この比較はpatch変更の検出（[ADR 0036](0036-surge-xt-macro-sweep.md)の「patch変更時」）にも使う。parameterで動かしたMacroの値が保存stateに入っても、patch変更と誤判定して固定値を`-`へ戻すことはない。

[ADR 0009](0009-sweep-cc1-favorite-comparison.md)の「sweep中だけMacro 1を無視する」規則と、それに使っていたsweep由来のCC1の追跡は消した。Macroの値を常に無視すれば、sweep中かどうかを区別する必要が無いため。

## 却下した案

- 3音源もCCで送る: 音源ごとのCC learnの設定を読む必要があり、learnしていないMacroは動かない。Vaporizer2のCC learnを欄の条件にしないことはuserが決めた。
- 値が変わったときだけparameterを送る: Floeは読み込み直後のeventを捨てるので、固定値が届かないことがある。
- 128段階より細かくparameterを送る: 表示・ドラッグ・周期の仕組みを作り直す必要があり、細かくする理由がまだ無い。
- FloeのMacro 1もparameterで送る: CC1欄のsweepと取り合い、Macro 1の値が飛ぶ。
- sweep中だけMacroの値を無視する（ADR 0009の方式）: 固定値やドラッグで動かしたMacroが別Favoriteになり、patch変更の検出も誤判定する。

## 制限

- 3音源とも、音源の画面でpatchを変えても欄の表示・非表示と`-`の表示値は変わらない見込み（hostからのstate読み込みでは`params.rescan`を出さないことを確認した。画面での変更は未確認）。アプリでpatchを変えるか、選択を`-`へ変えたときに更新する。
- `prepare_parameters`は音源のautomation modeを有効にする（[parameter automation補修](../../patches/uapmd/parameter-automation/README.md)）。3音源はvoiceを作るたびにこれを通る。音源の画面でつまみを動かしたときの影響は確かめていない。
- FloeのMacro 1（CC1）は欄に出ないので、CC1欄で動かす。
- Macroの位置だけを変えた音色は別Favoriteにならない。
- 適用範囲は、Floe 2.0.2 / state v30、Vaporizer2の`VC2!`・`VASTVaporizerParamsV2.20000`形式、Six Sinesの`<patch id="org.baconpaul.six-sines">`のXML。未知の形式では欄を出さず、Favoriteの比較はほかの規則に戻る。

## 検証

- 使用判定・欄の表示・見出し・`-`の表示値・FloeのCC1除外・Macroの値だけが違うstateの比較とpatch変更の判定はunit testで確認。手元のfavoriteでは、Six SinesのMacro 1・2、FloeのMacro 2（`Brightness`）だけが欄に出た（`instrument_macros_saved_tests`、ignored）。
- 実機で、欄の固定値0・127・sweepに合わせて`current`が値域の端・送った値と一致し、音が変わることを`macro_parameter_lane_native_tests`で確認（ignored、1件ずつ`--exact`）。0→127での音の変化の指標は、Vaporizer2 0.284、Six Sines 0.214、Floe 0.373（同じ音符の対照は0〜0.043）。`live`版は既定の出力デバイスで本物の音声callbackを回す。
- parameterで動かした後のstateが元のstateと同じFavoriteになり、Macro以外のparameterを動かすと別Favoriteになることを`macro_value_native_tests`で確認（ignored、1件ずつ`--exact`）。
- Surge XTの`native_organ_one_macros_follow_their_ccs`とTyrellN6の`native_controls_follow_their_saved_ccs`が通ることで、CC経路が変わっていないことを確認。
