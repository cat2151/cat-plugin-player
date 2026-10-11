# ADR 0025: cmrt の catalog が無い環境では Surge XT 用 catalog を取得して使う

- 状態: 採用。
- 決定者: user。切り替え条件（cmrt の catalog ファイルが存在しないとき）、毎起動の取得、取得失敗時に前回のファイルを使うこと、
  catalog を cat-music-patterns に置くこと、切り出し script を `scripts/` に置くことを承認。

## 決定

- patch browser の候補の読み元を、cmrt の `patch_catalog_cache::cache_file_path()` が**存在するか**で選ぶ。
  存在すれば従来どおり cmrt の `load()`。存在しなければ Surge XT 用 catalog を `load_from()` で読む。
  存在するが読めない場合は従来どおりエラーとし、Surge XT 用へは切り替えない。候補を組む処理は共通（[ADR 0019](0019-random-patch-all-catalog.md)）。
- Surge XT 用 catalog は `https://raw.githubusercontent.com/cat2151/cat-music-patterns/main/surge-xt-patch-catalog.json`。
  既存の catalog worker（[ADR 0014](0014-random-surge-patch-catalog.md)）の中で、起動ごとに1回取得する。タイムアウトは全体で10秒。
- 取得した bytes は保存先と同じディレクトリの一時ファイルに書き、cmrt の `load_from()` が通ったときだけ
  `%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json` を atomic に置き換える。
  取得または検査に失敗したら前回のファイルを使い、それも無ければ既存の `Patch catalog unavailable` として出す。
- catalog は `scripts/extract_surge_xt_patch_catalog.py` で cmrt の catalog から Surge XT の CLAP（`id:org.surge-synth-team.surge-xt`）分だけを切り出して作る。
  ディスクに無い patch は落とし、catalog に無い `*.fxp` がディスクにあれば停止する。書き出す前に、ドライブ付きパスが
  `C:\Program Files\` か `C:\ProgramData\` で始まることを確かめる。

## 理由

- [ADR 0024](0024-offer-surge-xt-via-winget.md) で Surge XT を自動インストールする環境には cmrt の catalog が無く、音色を Surge XT の GUI でしか選べなかった。
- catalog JSON を cat 側で解釈しない（ADR 0014）。cmrt の `load_from()` を公開して同じ検査を通せば、形式の追随は cmrt 側だけで済む。
- 保存先を cmrt のディレクトリにすると、次の起動で「cmrt の catalog がある」と判定してしまう。
- 検査してから置き換えるので、壊れたものや形式の古いものが届いても前回のファイルで動き続ける。
- 候補を実ファイルの有無で絞らない。catalog は Surge XT のインストール前に読まれることがあり、絞るとインストール後も候補が0件になる。
- effect catalog は cmrt を読まずにインストール済みの preset フォルダを走査するので、cmrt が無くても Surge XT Effects が候補に出る。

## 却下案

- cmrt.exe の有無で判定する: 要るのは catalog ファイルなので、ファイルの有無で足りる。
- ETag による条件付き取得と metadata ファイル: catalog は約2MB（gzip で約100KB）と小さく、分岐とファイルが増えるだけになる。
- effects 用・Surge XT 以外の音源用の catalog: effects は上記のとおり不要。自動インストールするのは Surge XT だけ。
- 取得失敗時の自動再試行、URL などの設定項目: 無くても成立する。

## 結果

- Surge XT 1.3.4 から生成した catalog は 3008 patch・1 plugin・`format_version` 5 で、そこから候補 3008 件が組めた。
- 作者の cmrt catalog には、Surge XT 1.3.4 に無い 163 件が残っていた。script のディスク突き合わせでこれを落とす。
- Surge XT が版上げされると、増えた音色は一覧に出ず、消えた音色を選ぶと準備失敗になる。script を流し直して cat-music-patterns を更新する。
- cmrt の catalog が無く Surge XT 以外の音源だけがある環境では、patch browser は空のまま。
- 取得が終わるまで（最大10秒）`Loading catalog...` のまま。
