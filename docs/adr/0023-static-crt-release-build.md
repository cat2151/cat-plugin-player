# ADR 0023: 配布物は VC++ ランタイム静的リンク版だけにし、crt-static だけで切り替えて CI で毎日公開する

- 状態: 採用。
- 決定者: user。配布物を静的リンク版の1種類にし、CI でだけビルドして毎日自動で公開することを承認。
- 関連: [ADR 0020](0020-visual-studio-2026-toolchain.md)（ツールチェーンとローカルの動的リンク版の要件）

## 決定

- 配布物は静的リンク版の zip（`cat-plugin-player-<version>-windows-x64.zip`）1つだけ。動的リンク版は配布しない。
  ローカルのビルドは動的リンク版のまま。
- 静的リンク版のスイッチは Rust の `-C target-feature=+crt-static` だけにする。専用の環境変数は足さない。
  - build.rs は `CARGO_CFG_TARGET_FEATURE` に `crt-static` があれば、CMake configure に
    `-DUH_MSVC_RUNTIME_LIBRARY=MultiThreaded` を渡す。無ければ空を渡す。
  - shim/CMakeLists.txt は値があれば、uapmd と依存を追加した後で、全 target の `MSVC_RUNTIME_LIBRARY` をその値にする。
  - cc でビルドする依存は、cc が同じ `crt-static` を見て `/MT` を選ぶ（cc 1.6.0 で確認）。
- release workflow（`.github/workflows/release.yml`）は rustflags を `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS` で渡す。
- `scripts/crt_linkage.py` が exe と `target/shim/out/uapmd_shim.dll` の import を検査し、
  VC++ ランタイムと UCRT の DLL に依存していれば止める。
- workflow は毎日 01:23 JST に走り、build・CRT 検査・test がすべて成功したときだけ、既存の `v<version>` の release と tag を
  消して作り直す。手動起動の既定はビルドと検査だけ。

## 理由

- 2種類あると利用者がどちらを取るか迷う。静的リンク版は再頒布可能パッケージが要らず、zip の差は 6.0MB と 6.4MB で小さい。
- Rust と C/C++ のランタイムが一致しないと、リンクエラーか、モジュール内で2つの CRT が混在する。
  Rust 側の flag 1つから C++ 側を決めれば、片方だけ切り替える事故が起きない。
- uapmd は `CMAKE_MSVC_RUNTIME_LIBRARY` を DLL 版で cache に FORCE し、midicci・umppi の target にも DLL 版を直接設定する。
  そのため `-DCMAKE_MSVC_RUNTIME_LIBRARY` をコマンドラインで渡しても上書きされ、再 configure では cache から DLL 版が読まれる。
  uapmd が触らない変数名で受け取り、全 target に後から設定する。
- 生成済みの全 vcxproj は `RuntimeLibrary` で CRT が決まり、`/MD` を直接足す依存は無い（動的版の `target/shim` で確認）。
- shim の C API は、呼び出し側のバッファと create/destroy の対で受け渡すので、exe と DLL が別々の静的 CRT を持っても
  メモリの所有権がまたがらない。
- 毎日の自動公開なら、user が release を管理する手間が無く、利用者は最新の上流を取り込んだ版を得られる。
  失敗した日に release を消さないので、前日の版が残る。

## 採らない案

| 案 | 理由 |
| --- | --- |
| 動的リンク版も配布する | 利用者が迷う。サイズの利点が小さい |
| `UH_STATIC_CRT` などの環境変数を足す | Rust 側の crt-static と二重のスイッチになり、食い違える |
| `-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded` を渡すだけ | uapmd の cache FORCE と target 設定で DLL 版に戻る |
| uapmd に補修を当てて FORCE を外す | cat 側の target 設定で足りる。補修の保守が増える |
| ローカルでも静的版をビルドする | user の決定によりローカルは動的版だけ |

## 結果

- CRT の選択は configure 時に固定される。build.rs は configure を build dir ごとに1回しか走らせないので、
  既存の `target/shim` で crt-static を切り替えても C++ 側は変わらない。切り替えるときは `target/shim` を削除する。
- 静的版は exe と DLL がそれぞれ CRT を持つ。CI の静的版で検査が通り、実機で音が鳴ることを確認した。
- 配布物とローカルで CRT のリンク方式が違う。静的版だけで起きる不具合は、`target/shim` を削除して
  crt-static でビルドし直さないと手元で再現できない。
- 公開される exe は、その日の上流 main（uapmd・兄弟 repo）を使う。手元で確認した uapmd と一致するかは release ノートに書く。
