# ADR 0012: 3つの依存repoを常時ローカル参照し、target/releaseから運用する

- 状態: 採用。UAPMDの既存ローカル参照を維持し、TUI・play-serverのCargo参照を実装済み。
- 決定者: user。常時ローカル依存と、cargo installを使わずtarget/releaseから運用することを明示承認。
- 関連計画: [Random patch handoff](../../random-patch-handoff.md)

## 背景

当アプリはすでに兄弟ディレクトリのUAPMD checkoutをCMakeでビルドする。
Random patchではclap-mml-render-tuiのcatalog readerと、clap-mml-play-serverで整備する
patch変換APIも使う。3repoを横断して変更・検証する際、catの連携検証にcommit/pushと
公開Gitへの反映を挟むと、動作確認のための公開操作が必要になってしまう。

TUIにはplay-serverのGit依存をローカルへ切り替える既存scriptがあるが、
catはUAPMDと同じ常時ローカル連携を採る。切替状態とCargo.lockの復元を運用へ追加しない。
userは実行元もcargo install先から既定target/releaseへ変更した。

## 決定

| ローカルrepo | 利用内容 | catからの参照方式 |
| --- | --- | --- |
| `../uapmd` | plugin hosting、音声処理、state適用 | 既存build.rs/CMake。既存UAPMD_DIR指定も維持 |
| `../clap-mml-render-tui` | catalog reader、変換APIの再export | `app` と `patches` のCargo path依存 |
| `../clap-mml-play-server` | 変換APIの実装、共用config型 | cat rootのCargo patchでTUI内のGit依存もローカルへ統一 |

- TUIの `app` はpackage `clap-mml-render-tui`、`patches` は `cmrt-patches`。
  play-serverは `core-lib` が `cmrt-core`、`server-config` が `cmrt-server-config`。
  cat rootのCargo.tomlでTUIの2packageをpath参照し、play-serverのGit URLに対する
  `[patch]` で後者2packageを兄弟checkoutへ向ける。
- catの常時参照設定は追跡可能なCargo.tomlへ記載する。個人の絶対パスを埋め込まない。
  cat用のon/off script、ignored configによる必須切替、Gitへの自動fallbackは追加しない。
  repo欠落やAPI不足はその原因を示して停止する。
- TUI側の `.cargo/config.toml` はcatからのCargo実行へ自動継承されないため、
  cat自身の設定で推移依存を揃える。TUIとplay-serverの既存依存方針・切替scriptは変更しない。
- 公開APIの整備はRustの `pub` APIとして共用できるようにする意味で継続する。
  GitHub公開、commit、push、TUIのlocal offをcat連携検証の前提にしない。
- Release版を `cargo build --release` でビルドし、当アプリの既定
  `target/release/cat-plugin-player.exe` を直接実行して運用する。
  `cargo install` は運用・受け入れ確認に使わず、別targetディレクトリも作成しない。
- この決定は既存のcargo install済みexeの削除やPATH変更を許可しない。
  古いインストール先のexeを取り違えないよう、実行には上記の明示pathを使う。

## 影響と検証

未commit変更を含むローカルAPIをcatからすぐ検証でき、公開待ちが不要になる。
一方、ビルド担当者・CIは3repoのcheckoutを用意する必要がある。
実行時にcheckoutが必要かどうかとは別に、ビルド時の依存が増える。
この決定は他のcrateやCMakeの依存取得までローカル化するものではなく、完全なネット接続不要を保証しない。

Cargo.lockはpath依存のソース内容やGit revisionを固定しない。
検証結果にはcatを含む各repoのHEAD、未commit変更の有無、実際の依存pathを記録する。
HEADの記録を未commit内容の完全な再現性保証とは扱わない。
依存checkoutを自動pull/resetして内容を揃えたり、無関係なlock差分をHEADへ戻したりしない。

実装時にはcat rootで `cargo metadata --format-version 1` のresolved graphを調べ、
到達するTUI/play-serverのpackageがすべて意図したローカルrepo内のmanifestを指し、
Git由来との二重取り込みがないことを確認する。UAPMDはCargo graph外なので、
build.rsの解決先と既存CMakeCacheのUAPMD_DIRを別に確認する。
既存手順のfmt、clippy、test、release buildを行い、target/releaseのexeで実機確認する。

CIのcheckout準備はローカル参照を満たす必要がある。
既存の共用workflowが自動で兄弟repoを用意すると仮定せず、必要な対応を調査・報告する。
このADRは別repoの共用workflow改修や検証省略を許可しない。

## 採らない案

| 案 | 理由 |
| --- | --- |
| catのTUI/play-server依存を常時Gitから取得 | 横断検証に上流公開を挟むため、既存UAPMDのローカル運用と揃わない |
| catにもTUI同様のon/off scriptを追加 | 常時ローカルなら切替は不要。解除時のlock復元と状態管理も増える |
| TUIだけローカルで、play-serverはGitのまま | 変換APIとconfig型が公開版・ローカル版で混在する |
| cargo install先のexeを運用 | userがtarget/releaseからの直接実行を選択した |

## 確認範囲

このADR作成では既存の依存定義・build.rs・CMake・運用資料・TUI切替scriptを読み取り確認した。
Cargo path/patchの追加後、cat rootのmetadataで到達するTUI・play-serverの全packageが
兄弟checkout内のmanifestを指し、Git版が重複しないことを確認した。既定targetの `cargo check` が成功した。
UAPMDは既存build.rsの兄弟path解決と `target/shim/CMakeCache.txt` のUAPMD_DIRが同じcheckoutを指す。
これらは既存キャッシュでの検証であり、新規環境での再現性やRandom patchのGUI適用を保証しない。

CI調査では、catの `call-rust-windows-cargo-check.yml` が参照する共用workflowは現在repoだけを
checkoutし、兄弟repoを配置する処理もcheckout準備用のinputもなかった。
CIを成立させるにはcatを含む4repoの兄弟配置と、必要な公開APIを持つcheckoutの準備が必要となる。
参照先は `cat2151/github-actions/.github/workflows/rust-windows-cargo-check.yml@main`。
別repoの共用workflow改修は今回実施していない。ローカル解決成功をCI成功とは扱わない。
