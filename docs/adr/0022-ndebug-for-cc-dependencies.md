# ADR 0022: cc でビルドする依存の C/C++ に NDEBUG を渡す

- 状態: 採用。
- 決定者: user。debug を含むすべてのビルドで依存の C/C++ の `assert` を無効にし、`/W4` は足さないことを承認。

## 決定

- repo の `.cargo/config.toml` の `[env]` で `CFLAGS_x86_64_pc_windows_msvc` と
  `CXXFLAGS_x86_64_pc_windows_msvc` に `/DNDEBUG` を設定する。debug を含むすべての profile で効く。
- `/W4` などの警告フラグは足さない。
- `force` は付けない。同名の環境変数を設定したビルドでは、そちらが優先される。

## 理由

- cc は NDEBUG を定義せず、tree-sitter の build.rs も定義しない。MSVC の `assert` は
  ソースの絶対パスを UTF-16LE で exe に埋め込み、`--remap-path-prefix` では消えない。
- NDEBUG の有無は個人の環境ではなく、この repo のビルド結果の性質なので、CI でも同じに効く repo 側に置く。
- ターゲット付きの名前にすると、`CFLAGS` だけを設定した別のビルドと混ざらない。
  cross compile しない限り、ビルドスクリプト用の host ビルドにも同じ名前で効く。

### release のときだけ無効にしない理由

- user の目的は、普段の `cargo build --release` でも assert を無効にして高速化すること。
  なので、リリースバイナリを作る CI だけで assert を無効にしても、目的を達成できない。
- cargo の `[env]` は profile 別に分けられない（項目は value・force・relative だけで、`[profile.*]` に env は無い）。
  cc 1.6.0 は NDEBUG を定義せず、`PROFILE` も見ない（見るのは `OPT_LEVEL` と `DEBUG` だけ）。
- release.py や CI のときだけ環境変数を付けると、普段の exe と配布物が一致せず、普段の exe には
  tree-sitter の絶対パスが残る。切り替えるたびに cc を使う crate とその下流が再ビルドされる。

### `/W4` を足さない理由

- cc は環境変数のフラグを crate が指定したフラグより後ろに置き（cc 1.6.0 の lib.rs 2480 行付近、
  「do this last」）、MSVC は後に書いたフラグが勝つ。`/W4` を足すと、`warnings(false)` を指定している
  lua-src・tree-sitter・rubberband-ffi が `/W4` で上書きされる。rubberband-ffi は path 依存なので、
  cargo の画面にその警告が出るようになる。

## 結果

- 警告レベルが変わるのは、警告を指定していない git 依存の mmlabc-to-smf と tree-sitter-chord だけ
  （cc の既定の `/W4` が付かなくなり、cl の既定の `/W1` になる）。git 依存なので cargo は警告を表示せず、
  画面上の差は無い。警告レベルで出力バイナリは変わらない（逆アセンブルで確認）。
- lua-src は release では `LUAI_ASSERT` も `LUA_USE_APICHECK` も定義せず、もともと assert が無いので、
  配布 exe は変わらない。debug では `LUA_USE_APICHECK` の検査が無効になる。
- 依存の C/C++ の `assert` が debug で要るときは、シェルで `CFLAGS_x86_64_pc_windows_msvc`
  （C++ なら `CXXFLAGS_x86_64_pc_windows_msvc`）を設定してビルドすると、`[env]` より優先される（cargo の `[env]` の仕様。この repo では未検証）。

## 採らない案

| 案 | 理由 |
| --- | --- |
| `CARGO_HOME` の config.toml に置く | 他の repo と CI に効かない |
| shell で `CFLAGS` を設定してビルドする | 設定し忘れたビルドで気づかずパスが入る |
| release.py や CI のときだけ環境変数を付ける | 「release のときだけ無効にしない理由」のとおり |
| `CL` 環境変数で `/DNDEBUG` を渡す | cc が監視しないので、変えても再ビルドが起きない。shim の MSBuild にも効いてしまう |
| `/W4` も足す | 「`/W4` を足さない理由」のとおり |
