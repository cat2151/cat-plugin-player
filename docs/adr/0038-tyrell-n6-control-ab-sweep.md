# ADR 0038: TyrellN6のControl A / BをCCで自動変化させる

- 状態: 採用
- 確認: userは、TyrellN6にはSurge XTのMacro 1〜8に似たControl A / Bがあり、preset patchによってはmodulation sourceがModWheelだけでなくControl A / Bになっている可能性があるため、まずfactory presetを調査し、使っているpatchがあればSurge XTのMacroの表示やsweepを参考に同様にすることを依頼。

## 決定

[ADR 0036](0036-surge-xt-macro-sweep.md)のMacro欄の仕組みをそのまま使い、TyrellN6のControl AをMacro欄の1番目、Control Bを2番目に載せる。
見出しは「CC*n* Control A」「CC*n* Control B」。選択肢・ドラッグ・sweepの周期（Control Aが2.9秒、Control Bが3.7秒）・`M`キー・session保存・patch変更時に固定値を「-」へ戻す動作は、Surge XTのMacroと共通。
patchのUsage欄に`Ctrl A = bass dominator`のような説明があれば、見出しをその説明にする。

factoryの669 patch中、Control Aを使うのは33件、Control Bは7件（重複を除き38件）。
Usage欄にCtrl A / Bの説明があるpatchは、すべてこの判定でも使用と出る。

## Control A / BとCCの判定

[tyrell_n6_control.rs](../../src/plugin_specific/tyrell_n6_control.rs)が、TyrellN6 CLAPのstateのテキスト部（圧縮領域より前）から判定する。

- 使用: 下の項目の値が、`#ms=`の並びで`CtrlA` / `CtrlB`を指す。どれもCLAPパラメータとして公開され、表示名で値3が`CtrlA`、4が`CtrlB`と出ることを実機で確認した。
  - PCore: `MMS1` `MMS2`（Matrix Source）、`MMVS1` `MMVS2`（Matrix ViaSrc）
  - Tyrell: `TMSrc1` `TMSrc2` `PWSrc` `FMSrc1` `FMSrc2` `ARMod`（AudioRateModSrc）
  - LFO1 / LFO2: `FMS1`、および非公開の`DMS1`
- `DMS1`は値の分布がmodulation sourceの範囲と一致し、`DMS1`だけでControl Aを使うpatch（Chromium）のUsage欄が`Ctrl A = vibrato`なので含める。
- depth（`MMD1`など）が0でも使用とみなす。対応するdepth項目が確認できない項目（`ARMod`）があるため。
- CC番号: PCoreの`CtrlA=` / `CtrlB=`。実機で`CtrlA=20`に書き換えると、CC20だけでControl Aが動いた。factory presetの多くはこの行を持たず、読み込み後のstateに値が入る。
- 対象はTyrellN6 CLAP（`com.u-he.TyrellN6`）で、`#AM=TyrellN6`があるstateだけ。CCが0〜127外、行が無いときは欄を出さない。

## 判定の時期

TyrellN6はpatchを変えても`params.rescan`を出さない（実機でstate読み込み後も通知回数が増えない）。
そのため、アプリのBrowse patches・Favorites・Historyでpatchを変えたときは、通知を待たずに次のフレームでstateを読み直す。
TyrellN6はstateを読み込みと同時に適用するので、直後の取得で新しいpatchの値が返る。
Surge XTでもこの読み直しは行われ、前のpatchのstateを返すが、続く通知で正しいstateを読み直すため表示は変わらない。

## 却下した案

- CLAPパラメータとしてControl A / Bを直接動かす: Control A / BはCLAPパラメータとして公開されていない。
- 一定間隔でstateを読み直してTyrellN6のGUIでのpatch変更に追従する: [ADR 0036](0036-surge-xt-macro-sweep.md)と同じく、読むたびにUIスレッドを止める。

## 制限

- TyrellN6のGUIでpatchを変えても欄の表示・非表示とCC番号は変わらない。アプリでpatchを変えるか、選択を「-」へ変えたときに更新する。
- 「-」のときの値表示は送った値になる。Control A / Bと、ModWheelの現在値はstateのテキスト部に無いため。
- Favoriteの重複判定は、Control A / Bの値が圧縮領域にしか入らないため、[ADR 0011](0011-tyrell-n6-ui-operation-favorite-comparison.md)の比較のまま値の違いを無視する。
- VST3版TyrellN6は対象外。

## 検証

- 実機TyrellN6 CLAPで、Control A / BをCutoffへ割り当てたstateを読み込み、`CtrlA=20` / `CtrlB=21`から欄のCCを判定。CC20=127でControl Aだけがフィルタを開き（RMS 0.004→0.112）、CC21では変わらないこと、Control Bはその逆になること、演奏後もstateのテキスト部が変わらないことを`native_controls_follow_their_saved_ccs`で確認。
- 使用判定の項目・未使用・CC範囲外・他プラグイン・Usage欄の名前をunit testで確認。
