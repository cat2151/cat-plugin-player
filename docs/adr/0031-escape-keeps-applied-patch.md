# ADR 0031: patch browser の Esc は反映済みの音を残し、まだ鳴っていない選択だけを捨てる

- 状態: 採用
- 関連: [0015](0015-random-patch-transaction.md) / [0029](0029-shared-host-smf-selection.md)

## 決定

Esc で browser を閉じる・隠すときは `PatchBrowserRequests::invalidate` を今の形のまま使う。

- `selected` / `desired` / `confirmation` は捨てる。準備中の音色は準備が終わっても反映しない。
- `applying`（反映を始めた transaction）は捨てず、最後まで終わらせて `applied` にする。
- Esc で開く前の音へ戻す操作は持たない。

↓ の直後に Esc を押し、カーソルの音がまだ準備中なら、その音は鳴らないまま前の音で終わる。これは仕様とし、直さない。

## 理由

- 守るのは「user が聞いた音は捨てない」こと。反映を始めた音は editor で鳴るので、Esc の後も残す。
- 準備中の音はまだ鳴っていない。Esc の後でそれを採用すると、user が聞いていない音が曲に残る。
- 常駐では、↓ から約 130 ms より後の Esc でカーソルの音が残り、それより前ではまだ鳴っていない。
  境界の前後どちらでも、残る音は user が最後に聞いた音と一致する。

## 却下した案

- **準備中の音色も最後まで反映して残す。** カーソルの位置の音は必ず残るが、Esc の後に、user が聞いていない音が採用される。
- **`applying` も Esc で取り消す。** editor で既に鳴った音が捨てられ、その前の音に戻る。

## 結果

↓ の直後に Esc を押すと、カーソルの音が残らないことがある。残したいときは、音が鳴ってから Esc を押す。

## 壊れたら気づく場所

| テスト | 落ちたら |
|---|---|
| `resident_esc_while_the_editor_applies_keeps_that_patch` | 反映中の Esc で、鳴った音が捨てられた |
| `resident_esc_while_preparing_drops_that_patch` | 準備中の Esc の後に、鳴っていない音が editor へ送られた |
| `resident_esc_after_an_applied_patch_hides_and_keeps_it` | 反映済みの音が Esc で失われた |
| editor `escape_soon_after_down_keeps_a_patch_that_played`（`#[ignore]`、自動操縦） | 本物の editor と組んだとき、Esc 前に鳴った音が保存曲に残らなかった |
