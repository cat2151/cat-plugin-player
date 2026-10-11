# ADR 0026: 初回導入の紹介録画に使う recording-mode

- 状態: 採用。
- 決定者: user。

## 決定

- Cargo feature `recording-mode` は既定OFF。通常のビルドには初回状態の切り替えを適用しない。
- feature付きexeは起動ごとに `%LOCALAPPDATA%\cat-plugin-player-recordings\<時刻>-<PID>-<連番>\` を新規作成する。設定、status、state、Favorites、History、画像、Surge catalog、shim DLLの展開先をそこへ分離する。過去の録画データは削除せず、新しい起動では読まない。作成できない場合は通常の保存先へ戻さずエラーにする。
- shimの走査モード2はCLAPの候補パスを列挙し、`Surge XT.clap` と `Surge XT Effects.clap` だけを読み、Surgeの2つのCLAP IDだけを採用する。通常のplugin list cacheの読込・更新と、他のプラグインのロードを行わない。VST3は録画対象外。
- Surge XTの不在やインストール成功を偽装しない。撮影前に人間またはagentがアンインストールし、実ファイルの不在を確認する。UACは人間が操作する。
- 録画用featureでのwinget installにだけ `--custom /NOCLOSEAPPLICATIONS` を加える。Inno Setupが録画中のOBSを閉じないようにする。wingetの標準引数・ハッシュ検証・昇格・終了コードの判定は維持する。通常ビルドには追加しない。
- cmrt catalogを参照せず、既存のSurge XT catalog取得・検査・保存処理を使う。保存先が新規なので前回ファイルへのfallbackはない。
- Effects catalogはSurge XT Effectsだけを既存APIで収集する。実インストール成功後は、起動時の未インストール状態で収集した結果とworkerを破棄し、再スキャン後に再収集する。この再収集は通常モードの初回インストールにも適用する。
- 自動操作をアプリに埋め込まない。撮影中のアプリは人間が操作する。

## 理由と制限

- スキャン結果を表示だけで隠すと、共有キャッシュがアンインストール済みSurgeを残したり、通常環境のキャッシュを更新したりする。限定走査で実際の導入状態を示す。
- 毎回新規の保存先にすることで、撮り直し時に通常の履歴・設定を消す操作が不要になる。録画データは蓄積するので、不要になったセッションは撮影後に整理する。
- Surgeのプラグイン自身が管理するユーザー設定や追加音色、Windowsのインストール登録、UAPMDのblocklistを隔離するものではない。個人の追加音色を削除しない。blocklistでSurgeが除外されている場合、録画モードでも解除しない。
- Surge以外のCLAPとSurge VST3を紹介する録画には使えない。限定するのは紹介の再現性のためで、一般利用者向けのプラグイン制限ではない。
- キャッシュ無しのSurge catalog取得にはネットワークが必要。通常の導入失敗・catalog失敗をそのまま表示し、録画成功のためにエラーを隠さない。
- アンインストール・録画・OBSの起動はfeatureの副作用にしない。
- Surge XT 1.3.4のインストーラはRestartManagerで録画中のOBSを使用中アプリと判定し、終了確認を出す。OBSが終了しないとインストーラは中断し、wingetは `0x8A15010C` を返す。上流の[1.3.4のインストーラ定義](https://github.com/surge-synthesizer/surge/blob/release_xt_1.3.4/scripts/installer_win/surge64.iss)は `*.exe,*.vst3,*.ttf` を検査し、フォントをインストールする。OBSがどのファイルを使用しているかは未特定。
- [wingetのcustom引数](https://learn.microsoft.com/en-us/windows/package-manager/winget/install)で既定引数に追加し、[Inno SetupのNOCLOSEAPPLICATIONS](https://jrsoftware.org/ishelp/topic_setupcmdline.htm)でアプリの終了だけを防ぐ。使用中ファイルの更新まで保証するものではなく、必要な再起動・導入失敗は引き続き報告する。
