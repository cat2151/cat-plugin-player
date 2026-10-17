# 0029: shared hostとSMF選択モード

## 背景と決定

cat-smf-editorの音色・effect編集をこのアプリのbrowser／お気に入りへ委譲する。
editor自身の試聴・範囲再生・全track再生は残し、両アプリで同じhost実装を使う。
独立crate、常駐サービス、editorへのApp/UI/MML依存全体の移植は追加しない。

Rustの正本は`shared/`のFFI、identity、state、chain、settling、SMF、timelineモジュール。
両既存crateへpath付きmoduleとして組み込む。native hostの正本はこのrepoの`shim/`、
configureとtarget定義は`shared/host.cmake`、build driverは`shared/build.rs`。
各アプリの既定`target/shim`でUAPMDを一度だけconfigureし、hostとumppiのSMF bridgeをbuildする。
各exeは自分のDLLを埋め込み、DLLを生存させたままUI thread上でhostを破棄する。
editorの旧Surge専用hostとfactory-only browserは削除する。

CLAP note形式とstate.load失敗伝搬の既存補修を、note→stateの順で各buildのgeneratedへ適用する。
正本のscripts/tests/説明書を共用し、UAPMD checkoutを書き換えない。未知の上流版は既存検査で停止する。
CRTは各アプリのRust flagsと合わせ、cacheを別targetへ逃がさない。

## 選択モード

`--select-smf <path> --browser patch|effect --preserve-position`で同じSMFを入出力に使う。
browserは省略時patch。位置保持は明示指定を必須とする。入力は一つのtrackと必要なtempoを含む。
`--browser-only`は選択モード専用で、main viewportを非表示にし、初期browserを一度だけ前面へ出す。
非表示のroot updateでhost処理を維持する。Favorites / routingは明示操作でrootを表示する。
browserを閉じるEnterも保存・全体終了に接続し、
変更処理や保存が終わるまでbrowserを残す。保存失敗のエラーはbrowser内に出し、再試行できる。
通常statusのMML・音色・effectは復元せず、選択モードから通常statusを書き戻さない。
お気に入りのsound snapshotは利用するが、そのMML／演奏パターンで入力SMFを置き換えない。

sample位置をprocessorの寿命から分離して保持する。新processorは同じ位置からevent cursorと
直前のcontroller・持続音を復元する。交換中の短い無音で音楽時刻を進めず、自然なloop境界だけは先頭に戻す。
初回位置は`--start-seconds`からsampleに換算し、同じ保持cursorへ設定する。
`--loop-end-seconds`は入力以上のloop長を指定する。開始位置・終端を検査してから音源を作り、
元のSMFは加工しない。UAPMDのSequencerEngineにもjumpPlayback／playbackPositionがあるが、
選択モードの既存Timeline/LoopPlayerを使い、別engineや多重hostを追加しない。
旧streamをjoinして全channelの発音を解放してからstateを扱う。state capture前のDSP settlingも共通hostに集約する。

本体とbrowserのEsc、文字入力として消費されないq、×は同じ保存要求になる。
処理中の変更が確定するまで待ち、unsafe state／復元失敗／処理失敗を成功として保存しない。
同じdirectoryの新しいファイルを完全に書き、入力の外部変更を確認して置換する。
保存失敗時は画面と選択を保ち、成功してから正常終了する。

## SMF契約

tick 0の`FF 7F`に`7D`＋`cat-plugin-player\0chain\0`＋JSONを一つ保存する。
schema version 1はinstrumentのformat/id/bundle pathとstate bytes、表示情報、
順序付きeffectsの同じ情報、chain全体のbypassを持つ。preset名やfavorite IDで復元しない。
Device/Program Nameは既存の表示情報として保持し、復元根拠にしない。
version、重複、位置、identity、stateとpayloadの容量を検査する。旧editor形式の移行は追加しない。
未設定の普通のSMFは従来の既定音色相当のSurge XTを使う。
stateのOS／plugin版互換や外部sampleの複製は保証しない。

editorはformat 0/1のconductor tempoをcurrent trackへ反映し、format 2は対象track自身の時間軸を使う。
division、channel、note tick/duration/velocity、CC、未知meta等を保持し、別trackの音符を混ぜない。
非同期の子processが正常終了し、返却performanceと設定を検査した後、UI threadでnative復元を確認する。
file/track/generationが合う呼出し元へ設定だけを取り込み、元のノート・tempo・他trackを丸ごと置き換えない。
dirty/savedの既存経路を使い、原本は上書きしない。一時ファイルは呼出しごとに区別し、正常完了で今回のものだけ片付ける。
失敗した入力は調査用に残す。

## 検証と制限

host契約のnative testsはこのrepoの通常test buildへ集約する。
editorには抽出・process・対象guard・saved往復など接続の検査を残す。
Rustのnative testsはprocess-lifetime main-thread登録のため、各caseを別プロセスで実行する。
CLAPのinactive params.flushの既存診断は補修範囲外で、追加patchをこの変更に混ぜない。
聴感、実画面の操作、複数trackのfollow/mixerはhandoffの人間確認を別に行う。
