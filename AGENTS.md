# その他
- README.mdは更新禁止。README.ja.mdから生成されるので。
- ビルドロック回避を含め、`--target-dir`等で既定以外のtargetディレクトリを作成・使用することを禁止

# rustソースを変更して作業完了報告時
- 450行をoverした*.rsは、単一責任の原則に従いファイル分割
- cargoのclippyとfmtを使うこと
- リリースビルド（ cargo build --release ）をすること
