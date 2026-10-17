# ADR 0030: master選択ではtrack chainを固定し、effectだけを返す

- 状態: 採用
- 関連: [0029](0029-shared-host-smf-selection.md) / [editor ADR 0004](../../../cat-smf-editor/docs/adr/0004-master-effects-in-conductor-track.md)

## 決定

`--select-smf` に `--master-effects` を付けた場合はmaster effectの選択とする。
入力はformat 1の1track一時SMF。保存契約はeditor ADR 0004に集約する。

- 既存track chainを読み込んで固定した試聴sourceとし、effect browserとroutingにはmasterのeffectを載せる。
- patch browserはmaster選択中に開かない。master選択でinstrumentのstateを返却SMFへ書き戻さない。
- 試聴sourceは独立した共有 `Chain` が所有する。streamを止めてからprocessor・source chainを解放する。
- shared shimの `instrument_id == -1` はeffectだけのprocessorを作る。
  `process_input` が合成済みのinterleaved audioを受け、順序付きeffectを通す。
  mono/stereoを変換し、空chain・bypassは入力を保持する。処理失敗時は無音にしてエラーを記録する。
- sourceのtrack effectはmaster bypassの対象にしない。同じpluginをtrackとmasterで使っても別instanceで処理する。
- 通常のSMF選択は既存instrument・track effectの保存契約を保つ。

## 理由

masterを現在trackのeffectとして編集すると、保存先もbypassの対象も混ざる。
固定sourceと編集中のmasterを分けることで、既存effect browserの追加・削除・順序変更を使いつつ、
元のtrack設定とmaster設定を独立して保持できる。
音声入力を共有hostへ追加することでeditorの全track合成にも同じeffect-only処理を使える。

## 却下した案

- **track effectとmaster effectを1つの編集chainに連結する。** track側を誤って変更・削除でき、bypassも共有される。
- **masterに仮のinstrumentを保存する。** 合成音の処理に不要な音源を保存契約へ持ち込む。
- **playerに全trackミキサーを新設する。** 今回は既存の現在track試聴契約を維持する。

## 結果

master選択中は現在trackのみ試聴する。全trackへ適用した結果はeditorへ戻った後の再生で確認する。
format 0/2のmaster対応は検証用の運用データが無いため実装しない（YAGNI）。
既存format対応を削除したり、一時入力を理由に原曲のformatを変換したりしない。

## 壊れたら気づく場所

| テスト | 落ちたら |
|---|---|
| `master_selection_saves_only_master_and_keeps_preview_track_chain` | master選択でtrack設定・演奏を上書きした |
| `native_master_effect_processes_input_bypass_and_empty_chain_pass_through` | effect-only DSPの入力、bypass、空chain、失敗時の無音が壊れた |
