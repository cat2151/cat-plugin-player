# ADR 0023: VC++ ランタイム静的リンク版は crt-static だけで切り替え、CI でだけビルドする

- 状態: 採用。
- 決定者: user。配布物を動的リンク版と静的リンク版の2つにし、静的リンク版は CI でだけビルドすることを承認。
- 関連: [ADR 0020](0020-visual-studio-2026-toolchain.md)（ツールチェーンと動的リンク版の要件）

## 決定

- 静的リンク版のスイッチは Rust の `-C target-feature=+crt-static` だけにする。専用の環境変数は足さない。
  - build.rs は `CARGO_CFG_TARGET_FEATURE` に `crt-static` があれば、CMake configure に
    `-DUH_MSVC_RUNTIME_LIBRARY=MultiThreaded` を渡す。無ければ空を渡す。
  - shim/CMakeLists.txt は値があれば、uapmd と依存を追加した後で、全 target の `MSVC_RUNTIME_LIBRARY` をその値にする。
  - cc でビルドする依存は、cc が同じ `crt-static` を見て `/MT` を選ぶ（cc 1.6.0 で確認）。
- release workflow（`.github/workflows/release.yml`）は動的・静的を matrix の別 job でビルドし、
  `target/shim` を作り分けない。CI の静的版では rustflags を `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS` で渡す。
- 各 job で `scripts/crt_linkage.py` が exe と `target/shim/out/uapmd_shim.dll` の import を検査する。
  静的版は VC++ ランタイムと UCRT の DLL に依存しないこと、動的版は VCRUNTIME140 に依存すること。

## 理由

- Rust と C/C++ のランタイムが一致しないと、リンクエラーか、モジュール内で2つの CRT が混在する。
  Rust 側の flag 1つから C++ 側を決めれば、片方だけ切り替える事故が起きない。
- uapmd は `CMAKE_MSVC_RUNTIME_LIBRARY` を DLL 版で cache に FORCE し、midicci・umppi の target にも DLL 版を直接設定する。
  そのため `-DCMAKE_MSVC_RUNTIME_LIBRARY` をコマンドラインで渡しても上書きされ、再 configure では cache から DLL 版が読まれる。
  uapmd が触らない変数名で受け取り、全 target に後から設定する。
- 生成済みの全 vcxproj は `RuntimeLibrary` で CRT が決まり、`/MD` を直接足す依存は無い（動的版の `target/shim` で確認）。
- shim の C API は、呼び出し側のバッファと create/destroy の対で受け渡すので、exe と DLL が別々の静的 CRT を持っても
  メモリの所有権がまたがらない。

## 採らない案

| 案 | 理由 |
| --- | --- |
| `UH_STATIC_CRT` などの環境変数を足す | Rust 側の crt-static と二重のスイッチになり、食い違える |
| `-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded` を渡すだけ | uapmd の cache FORCE と target 設定で DLL 版に戻る |
| uapmd に補修を当てて FORCE を外す | cat 側の target 設定で足りる。補修の保守が増える |
| 1つの job で両方ビルドする | `target/shim` を CRT ごとに作り分ける必要がある |
| ローカルでも静的版をビルドする | user の決定によりローカルは動的版だけ |

## 結果

- CRT の選択は configure 時に固定される。build.rs は configure を build dir ごとに1回しか走らせないので、
  既存の `target/shim` で crt-static を切り替えても C++ 側は変わらない。切り替えるときは `target/shim` を削除する。
- 静的版は VC++ 再頒布可能パッケージ無しで起動する見込み。exe と DLL がそれぞれ CRT を持つので、
  動的版よりファイルは大きくなる。
- 確認範囲: 動的版の `target/shim` を使うローカルの release ビルドと検査 script。
  uapmd と同じ cache FORCE・target 設定を持つ最小の CMake project で、全 target が `MultiThreaded` になり、
  再 configure 後も保たれること。静的版のビルドは CI の初回実行で確かめる。
