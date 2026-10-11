# ADR 0020: C++ のビルドを Visual Studio 2026 に統一する

- 状態: 採用。build.rs の既定 generator を `Visual Studio 18 2026` に変更済み。
- 決定者: user。ローカルと release 用 CI の両方で VS 2026 を使うことを承認。

## 決定

- build.rs が UAPMD と shim の CMake configure に渡す既定 generator を
  `Visual Studio 18 2026`（ツールセット v145）にする。ローカルも CI もこの既定でビルドする。
- `UH_GENERATOR` による上書きは残すが、VS 2022 など他の generator でのビルドは確認の対象にしない。

## 理由

- UAPMD 上流の CI は `windows-2025-vs2026` イメージと `Visual Studio 18 2026` でビルドしている。
  上流 main を取り込むとき、新しいコンパイラで通るかどうかを上流側が先に確かめている状態になる。
- GitHub Actions の `windows-latest` には VS 2026 しか無い。VS 2026 に揃えれば
  `windows-latest` をそのまま使え、Windows Server 2022 イメージの廃止で release workflow が止まることがない。

## ADR 0012 との関係

[ADR 0012](0012-local-dependencies-and-release-execution.md) の兄弟 checkout 構成と、
`target/release` の exe を直接実行する運用は変えない。この ADR は、そのうち
UAPMD と shim を CMake でビルドするときのツールチェーンだけを決める。
CI に4repo の兄弟配置が必要なことも ADR 0012 のとおり。

## 結果

- 配布する exe は MSVCP140.dll・VCRUNTIME140.dll・VCRUNTIME140_1.dll と UCRT（`api-ms-win-crt-*`）に依存し、
  同梱の uapmd_shim.dll はさらに MSVCP140_ATOMIC_WAIT.dll に依存する。
- 利用者の PC には、ビルドに使った MSVC ツール（確認時点で 14.51）と同じかそれより新しい
  Visual C++ v14 再頒布可能パッケージ（x64）が要る。VS 2026 付属の再頒布可能パッケージが
  対応する OS は Windows 10 / 11（Server は 2016 以降）。出典:
  [Latest Supported Visual C++ Redistributable Downloads](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)。
  VS の更新で MSVC ツールの版が上がると、利用者に要る再頒布可能パッケージの版も上がる。
- ランタイムの同梱や静的リンクは、この ADR では決めない。
- 罠: `target/shim` が別の generator で configure 済みだと、build.rs は configure を飛ばし、
  CMakeCache に記録された generator のままビルドし続ける。generator を切り替えるときは `target/shim` を削除して作り直す。

## 採らない案

| 案 | 理由 |
| --- | --- |
| VS 2022 のまま、CI を `windows-2022` に固定 | Server 2022 イメージが廃止されると release workflow が止まる。UAPMD 上流 CI とコンパイラの版がずれる |
| ローカルは VS 2022、CI だけ `UH_GENERATOR` で VS 2026 | 配る exe と、普段実機で確かめている exe のコンパイラが別になる |

## 確認範囲

ローカルの VS Build Tools 2026（MSVC 19.51、Windows SDK 10.0.26100.0）で `target/shim` を作り直し、
release ビルドの exe で音が鳴ることを人間が確認した。CI 上での VS 2026 ビルドは未実施。
