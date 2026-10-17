# ADR 0032: PluginScanTool の data directory 競合は uapmd 側を補修して直す

- 状態: 採用
- 関連: [0003](0003-build-local-dependency-patches.md) / [補修](../../patches/uapmd/scan-tool-data-dir-race/README.md)

## 決定

uapmd の `PluginScanToolImpl` のコンストラクタが、プロセス共通の data directory を無 lock で書く箇所を、
`patches/uapmd/scan-tool-data-dir-race/` の補修で static mutex の下に置く。shim とテストは変えない。

## 理由

- 別々の host を別々のスレッドで作っても安全であることは、ライブラリの側が持つ責務と判断した。
  uapmd の文書に「最初の作成は1スレッドで」という約束は無く、使う側からはこの共有が見えない。
- 壊れている箇所そのものを直すので、uapmd を shim 以外の経路から呼んでも同じ競合が残らない。
- 並列の `resident::tests::escape` は補修前に約1割の回で落ち、補修後は 160 回中 0 回だった。

## 却下した案

- **shim の `uh_create` で `AudioPluginHostingAPI::create()` を lock する。** 使う側でライブラリの不具合をかばう形で、他の経路に競合が残る。
- **テストを `--test-threads=1` にする。** 症状を隠すだけで、並列に host を作る将来の使い方で再発する。

## 結果

上流が直れば、補修の README の撤去条件に従って外す。上流への報告文は補修のディレクトリに下書きとして置き、送信は user が決める。
