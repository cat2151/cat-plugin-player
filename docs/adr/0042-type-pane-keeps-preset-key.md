# ADR 0042: patch browser の Type 欄は表示名だけを変え、保存キーとコードの名前は `preset` のままにする

- 状態: 採用
- 決定者: user（AI の提案を採用）。表示名を変える要件は user、名前 `Type` と保存キーの据え置きは AI の提案
- 関連: [cmrt ADR 0030](../../../clap-mml-render-tui/docs/adr/0030-patch-selector-type-pane-label.md)（依存元の cmrt も画面で `Type` と表示する）/ [cmrt ADR 0028](../../../clap-mml-render-tui/docs/adr/0028-selector-list-follows-preset-order.md)（同じ欄を `Preset 欄` と呼ぶ）

## 決定

- patch browser の2つ目の欄（strings / pad / kick / ★ Favorite / 保存した検索条件など）を、画面では `Type` と呼ぶ。
- status.json の `patch_browser.preset`、`--patch-filter-file` の JSON、コードの識別子（`preset` / `presets` / `restore_preset` など）は変えない。データ移行もしない。

## 理由

- 欄の中身は依存ライブラリ `cmrt_patch_select` の `FilterPreset` で、コードもこの呼び名で揃っている。キーだけを `type` にすると、保存形式とコードで呼び名が2つになる。コード全体を名前替えしても、ライブラリとはずれたまま残る。
- キーは status.json を手で開いたときにしか目に入らず、変えても画面上の得は無い。
- 古いビルドと新しいビルドが同時に動くと（cat-smf-editor が起動した resident など）、相手が書いたキーを読めず、選択が ALL に戻る。
- cat-smf-editor は欄の選択を `[Role, Type]` の並びで受け渡していて、キー名には依存していない。どちらを選んでも editor 側は変わらない。

## 却下した案

- **キーを `type` に変え、実データを一度だけ書き換える。** 利用者が1人なので移行は Python 1回で済むが、上の呼び名の食い違いが残る。
- **表示名を `Category` にする。** Surge XT の分類（Category）と衝突する。
- **表示名を `Preset` のままにする。** plugin のプリセット（音色そのもの）や、お気に入り一覧の「Preset: …」と紛らわしい。
- **表示名を `Instrument` / `Filter` にする。** それぞれ楽器 UI・音響フィルターと衝突する。

## 結果

画面は Type、保存形式とコードは preset と、呼び名が2つ並ぶ。`cmrt_patch_select` 側でこの概念の名前が変わったら、コード・キー・実データの移行をまとめて揃える。
