# ADR 0036: Surge XTのMacro 1〜8をCCで自動変化させる

- 状態: 採用
- 確認: userは、Organ 1でMacro 1にもCC1と別の大きな音色変化が割り当てられているため、Macro 1もCC1とは違う周期でsweepしたいと依頼。小さく始めるため、Surge XTのMacro 1だけを対象にCC1の2倍の周期で動かすことを指定。CLAP Note形式補修で全CCをMIDI対応portへ転送することを選択。のちに、全Macroが同じ周期で音が画一的になったため、CC1の周期を軸にMacroごとに周期を変えることを依頼。

## 決定

左上欄のCC1 modulationの下に、Macro 1〜8ごとに「CC*n* Macro *m*」の行を出す。
行は、現在の音源stateがそのMacroをどこかへmodulation routingしていて、そのMacroのCCが判明した場合だけ表示する。
選択肢はCC1と同じ0 / 127 / sweepで、sweepは0→127→0を繰り返す。片道の時間はCC1が2.0秒、Macro 1〜8が2.9 / 3.7 / 4.3 / 5.3 / 6.1 / 7.1 / 7.9 / 8.9秒。
Macroごとに周期を変えるのは、全Macroを同じ周期にすると同時に同じ向きへ動き、音色の変化が画一的になったため。値は素数×100msで、CC1（20×100ms）とも互いとも短い時間では揃わない。
patchがMacroに名前（`customcontroller/entry/@label`）を付けていれば、行の見出しを「CC*n* Macro *m*」の代わりにその名前にする。Surgeは未命名を`-`で保存するので、`-`と空は未命名として扱う。

CC1とMacroの値表示バーを横にドラッグすると、0〜127の任意の固定値を送る。ドラッグ開始時の表示値からの相対移動で、バーの幅を0〜127に対応させる。
固定値は0 / 127と同じ扱いで、patch変更時に「-」へ戻す。

メインウィンドウの`M`キーで、Macro 1〜8をまとめてsweepと「-」の間で切り替える。表示中のMacroが全部sweepなら全部を「-」に、それ以外なら全部をsweepにする。欄ごとに選ぶ操作だけでは、全Macroを止める・動かすのに8回の操作が要ったため。非表示のMacroも同じ値にし、次のpatchで現れたMacroも直前の一括操作に揃える。Macro欄が1つも無いときは何もしない。
ドラッグ中は毎フレーム値が変わるため、sessionの保存はドラッグを離したときにまとめて行う。

Surge XT以外のプラグインにも類似の仕組み（Macroや割り当て可能なCC）がある。TyrellN6のControl A / Bは[ADR 0038](0038-tyrell-n6-control-ab-sweep.md)で同じ欄に載せた。

Surge XTのMacro 1〜Macro 8は、1つの操作で複数のパラメータに同時に影響を与える仕組みと認識している。
そのため、分解能が128段階しかないデメリットをあえて許容し、Control Changeを使っての制御でUXを検証すると決めた。
プラグインパラメータを直接動かす方式は採らない。

## MacroとCCの判定

[surge_xt_macro.rs](../../src/plugin_specific/surge_xt_macro.rs)が、[ADR 0007](0007-surge-xt-favorite-state-comparison.md)と同じsub3形式のstateから判定する。

- Macroの使用: `modrouting`の`source`が7〜14。上流[ModulationSource.h](https://github.com/surge-synthesizer/surge/blob/main/src/common/ModulationSource.h)の`ms_ctrl1`〜`ms_ctrl8`（`ms_modwheel`の次から連番）。
- CC番号: `patch/dawExtraState/customcontrol_map/c[@p="Macro番号-1"]/@v`。Surgeは実行中のMacro用CC（既定41〜48、MIDI learnで変更可能）をここへ保存し、`channelController`がこの番号のCCを対応する`ms_ctrl*`のtargetへ渡す。
- 送信channel: `customcontrol_chan_map`の同じ`p`が-1（全channel）または0のときだけ表示する。アプリはchannel 1（0）で送る。
- 割り当てが無い、CCが0〜127外、未知の形式は行を出さない。既定値41を推測で使わない。

factoryの637 patch中121件がいずれかのMacroをroutingしている（Macro 1は120件、Macro 8は10件）。

## 判定の時期と送信

Surge XTは、GUI・stateのどちらでpatchを変えても、数ブロック後にCLAPの`params.rescan`を出す（上流`SurgeSynthProcessor::processBlockPostFunction`の`patchChanged`確認）。
UAPMDはこれをUIスレッドで`parameterMetadataChangeEvent`として通知するので、shimがその回数を数え、増えたフレームでだけstateを読み直す。
ほかに、起動後の最初の1回、アプリでpatchを変えたとき（[ADR 0038](0038-tyrell-n6-control-ab-sweep.md)）、CC1・Macroの選択を「-」へ変えたときにも読む（ドラッグ中に毎フレーム読むとUIが止まるため、固定値やsweepへの変更では読まない）。
読み直しはSurge XTの場合だけ行い、読み込み・準備中やSMF選択中は行わない。
実機のOrgan 1でstate取得は約6〜12msかかり、UIスレッドを止める。そのため一定間隔では読まない。
実機でGUIからpatchを変えるたびに通知が届き、Macroを動かすCCでは届かないことを確認した。

選択はCC1と同じくMacro番号ごとにsessionへ保存し、次回起動時に復元する（音源やpatchごとには持たない）。
CC1とMacroの選択肢に「-」（未送信）を置き、初期値とする。「-」はpatchに保存された値を上書きしない。
0 / 127 / sweepを選ぶと送信を始め、CC番号の変更や再生開始では初期値から送り直す。

## patch変更時

patchを変えたら、CC1とMacroの固定値（0 / 127 / ドラッグした値）を「-」へ戻す。
固定値は一度送るとpatchの値を失わせ、patchによっては無音や想定外の音色になるため。
sweepは全域を通るので続ける。

- アプリのBrowse patches・Favorites・Historyでは、新しいpatchを鳴らす前に戻す。
- GUIでの変更は、読み直したstateが[ADR 0007](0007-surge-xt-favorite-state-comparison.md)の比較で前回と違えば戻す。この比較はMacro値とホイール値を無視するので、送ったCCやGUIでのMacro操作では戻らない。
- 「-」の欄は、読み直したstateの`modwheel/@s0`と`customcontroller/entry/@v`を0〜127へ換算して表示する。bipolarのMacroは-1〜1を0〜127へ対応させる（実機でCC0が-1、CC127が1）。

## 依存補修

UAPMDはCLAPへCCを渡さず、[CLAP Note形式補修](../../patches/uapmd/clap-note-dialect/README.md)はCC1とCC120だけをMIDI対応portへ渡していた。
この補修を、MIDI対応portへ全CCを渡すように広げた（[ADR 0005](0005-clap-input-note-dialect.md)）。native CLAP専用portにはCCを送らない。

## 却下した案

- CLAPパラメータとしてMacro 1を直接動かす: 分解能は上がるが、音源ごとのパラメータ解決とsample単位の配送が必要で、CCで足りるかのUX検証より先に作る理由がない。
- Surge XTの既定CC 41〜48だけを補修で通す: 汎用の補修にSurge固有の番号が入り、MIDI learnで変えたMacroは動かない。
- patch変更後に選んでいる0 / 127を送り直す: 新しいpatchに保存された値を失い、無音になるpatchがある。
- 1秒ごとにstateを読み直す: 読むたびにUIが6〜12ms止まり、GUIでのpatch変更への追従も最大1秒遅れる。
- patch読み込み直後のstateだけで判定する: Surgeはstateをaudio threadで遅れて適用するため、直後の取得は前のpatchを返す。直後の読み直し（ADR 0038）は行うが、正しい判定は続く通知で読むstateに頼る。

## 制限

- VST3版Surge XTのCC経路は確認していない。
- Favoriteの重複判定は、CC1と同じくMacro値（`patch/customcontroller/entry/@v`）の違いを無視する（[ADR 0007](0007-surge-xt-favorite-state-comparison.md)）。Macro位置だけを変えた音色は別Favoriteにならない。
- 「-」の欄の値は、patch変更時と選択を「-」へ変えたときにだけ更新する。GUIでMacroのツマミやホイールを動かしても表示は変わらない。
- Surge XT以外の音源では、patchを変えてもCC1の0 / 127は戻らない。

## 検証

- 実機Surge XT CLAPにfactoryのOrgan 1を読み込み、保存stateからCC41を判定。CC41=127でMacro 1が1.0、CC41=64で約0.50になり、CC1ではMacro 1が変わらないこと、CC42〜48でMacro 2〜8が動くこと、Macro値だけ違うstateが同じFavoriteと判定されることを`native_organ_one_macros_follow_their_ccs`で確認。
- 補修前は同じ手順でCC41を送ってもMacro 1が0のままだった。
- sweepの周期・送信先CC・未選択時の無送信・CC番号変更時の送り直し・patch変更時に0 / 127だけ戻ることをunit testで確認。
- 同じ実機テストで、CC1=64とCC41=64を送ったstateの表示値が64になること、bipolarにしたMacro 1でCC41=0 / 127が-1 / 1になること、stateの読み込み後に通知回数が増え、CC41では増えないことを確認。
