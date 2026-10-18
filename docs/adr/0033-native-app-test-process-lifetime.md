# ADR 0033: App起動テストはcaseごとにnative deviceのプロセス寿命を持つ

- 状態: 採用
- 関連: [0029](0029-shared-host-smf-selection.md) / [0032](0032-scan-tool-data-dir-race.md)

## 故障と比較証拠

[診断run 39090697087](https://github.com/cat2151/cat-plugin-player/actions/runs/39090697087)は、
元の失敗runと同じWindows image、Rust、UAPMD・tui・serverのSHAで、静的CRTのbuildとCRT検査に合格し、
通常の並列テストと診断用の直列テストがともにアクセス違反で終了した。

どちらもsnapshotテストのDLL読込・host作成・device照会・host破棄は完了し、
そのthreadの終了後、plugin_scanテストのhost作成完了後のdevice照会中に終了した。
最後のテスト完了行を原因とせず、名前付きの呼出前後ログを比較した結果である。

使用するCPAL 0.15.3のWASAPI実装は、最初の照会threadでCOMを初期化し、
`OnceLock<Enumerator>`をプロセス全体に保持する。一方、COM初期化のguardはthread終了時に解放される。
アプリは一つのUI threadで起動するが、Rustのテストharnessはcaseごとに短命threadを使う。
直列実行でもこの寿命の違いはなくならない。

ローカルの動的CRTでは通常テストの反復、200回の短命threadでのdevice照会、
32並列hostの作成・破棄を50プロセスで実行した比較はいずれも合格した。
CIの異常終了位置はdevice照会まで絞れているが、CPAL内の命令・native stackは未取得であり、
上流の特定行の不具合を証明したとは扱わない。

## 決定

通常のApp起動テスト17件は、`native_library::process::isolated`でcaseごとに子プロセスを使う。
case本体のassertionと実際のdevice照会を残し、DLL読込・host作成より前に境界を置く。

- 子は正確なテスト名一つを実行する。親は正常終了と「1件合格・失敗0件・ignore 0件」の結果を検査する。
- 子markerはcase名と照合し、再帰起動を拒否する。標準出力はファイルに受けてpipeの詰まりを防ぐ。
- 120秒の期限で子を停止して回収する。期限超過、異常終了、0件成功は失敗にする。
- 子には固有の一時config・status・DLL cache領域を渡す。App生成後のpath差替えだけに頼らず、
  起動時から実利用の保存内容を読み書きしない。領域とログは親が今回作成したものだけを片付ける。
- device照会を含まない複数host同時作成・破棄の通常回帰テストを残す。hostを直列化しない。

本体の起動・保存形式・音声処理、UAPMD補修、依存checkoutは変更しない。
既存のinstalled-plugin向けignoredテストへこの共通fixtureを一括適用しない。
それらは実利用の音色等を読む契約を持つため、従来の明示実行手順を維持する。

## 選択理由と適用範囲

App起動全体はdeviceとCOMの寿命も含むintegration testであり、native-onlyの並列host契約とは分ける。
全テストの直列化は診断でも失敗しており、回避にならない。
device照会を削除したり、故障caseをignoreしたりする方法も採らない。

ADR 0029のプロセス寿命境界を通常のApp起動caseへ適用し、ADR 0032の並列host契約は別途検証する。
共有FFI・native hostは変えず、editorの単一UI thread起動やtui/serverのlibrary呼出には変更を広げない。
新規dependency patchを作らないため、再cloneや上流checkoutへローカル編集を要求しない。

## 検証と限界

故障前の証拠は上記runの並列・直列両方の同一境界ログ。
修正後は同じ通常caseのassertion、子の実行件数検査、native-only並列host回帰、
fmt・clippy・release build・全テストで確認する。
CIでは修正差分に対する別のcommit・push許可を得たうえで、`publish=false`の静的CRT条件を確認する。
診断差分だけへの送信許可を修正差分へ拡張しない。
ローカル合格だけでCI復旧や原因となる上流命令の特定を主張しない。
