# ADR 0021: Windows VST3ローダー補修を撤去し、カテゴリ取得の本体選択だけshimに残す

- 状態: 採用。`patches/uapmd/windows-vst3-loader/` を撤去済み。
- 関連: [ADR 0003](0003-build-local-dependency-patches.md)（補修の置き方と撤去条件）

## 決定

- UAPMD `a4a96938eb31ddcb70209d62caf0dcc6ea9d3abe`（[Issue #66](https://github.com/atsushieno/uapmd/issues/66) の修正）以降を使い、
  Windows VST3ローダーのローカル補修を撤去する。古いUAPMDはサポートしない。
- カテゴリ取得（[plugin_catalog.cpp](../../shim/plugin_catalog.cpp)）が使うVST3本体の選択関数は、
  補修から [shim/windows_vst3_binary.h](../../shim/windows_vst3_binary.h) へ移して残す。

## 理由

- 上流の `getPluginCodeFile()` は、ABIディレクトリで最初に列挙された項目（証明書やフォントを含む）を返していた。
  上流修正は、拡張子 `.vst3` の通常ファイルだけを候補にし、`GetPluginFactory` を公開するライブラリだけを採用し、
  読み込み失敗時のシステムエラーダイアログを抑止する。補修が防いでいた症状を上流が直している。
- 補修の検査は上流修正後のconfigureで「self-contained VST3 binary validation」の不一致として停止し、
  ADR 0003の想定どおり上流変更を検出した。
- インストール済みのVST3バンドル6個（Reason Rack Pluginを含む）で、上流の `loadModuleFromVst3Path()` が
  バンドル同名の本体を読み込み、shimの選択関数も同じファイルを選んだ。リソースだけのバンドルと
  壊れた本体のバンドルでは、ダイアログなしで `nullptr` を返した。
- カテゴリ取得は、UAPMDが既に読み込んだモジュールに対して `InitDll` / `ExitDll` を重ねて呼ばないよう、
  読み込み前にパスを知る必要がある。上流の候補列挙関数は `ClassModuleInfo.cpp` 内に閉じていて呼べない。

## 見送ったもの

| 案 | 理由 |
| --- | --- |
| 補修を新しい上流コードに合わせて更新する | 上流が同じ不具合を直しており、二重の選択処理になる |
| カテゴリ取得で上流の `loadModuleFromVst3Path()` を使う | 読み込み後にしかパスが分からず、既に読み込み済みかを判定できない |
| 上流の候補列挙関数をheaderで公開する補修を足す | 補修を1件撤去するために別の補修を増やすことになる |

## 結果

- configureで生成・差し替えるUAPMDソースはCLAPの2件だけになる。
- カテゴリ取得の選択は上流より厳しい。名前を変えたバンドルに `.vst3` が複数あると、
  上流は読み込める最初の1つを使うが、カテゴリ取得は選ばず、カテゴリは不明になる。
