# ADR 0043: stb revision固定の補修を撤去する

- 状態: 採用
- 関連: [ADR 0003](0003-build-local-dependency-patches.md)（補修の置き方と撤去条件）、[ADR 0035](0035-stb-revision-pin.md)（撤去した補修）

## 決定

- UAPMD `4ec5c3b1c30130fdd3b7d4b375082273247e561f` 以降を使い、共有hostでstbのrevisionを先に宣言する補修
  （`patches/uapmd/stb-revision-pin/`）を撤去する。古いUAPMDはサポートしない。

## 理由

- 上流の `source/remidy/CMakeLists.txt` が、rtlog-cppの `CPMAddPackage` より前に
  `FetchContent_Declare(stb … GIT_TAG 2c980bb59875b0d32144a71867fbdebb2f77cd20)` を置くようになった。
  補修が固定していたのと同じSHAで、FetchContentは最初の宣言を使うので、補修が防いでいた症状（`stb_sprintf.h` の移動でrtlogのincludeが壊れる）は上流で起きない。

## 見送ったもの

| 案 | 理由 |
| --- | --- |
| 補修を残して上流の宣言と二重に固定する | 上流と同じSHAの宣言が2つになり、上流が版を上げたときに補修側が古い版で勝ってしまう |
| 既存cacheの検査だけを残す | 検査の根拠だった未固定取得が無くなり、守る対象が無い |

## 結果

- 共有hostはUAPMDより前に何も宣言しない。stbの版は上流の宣言に従う。
