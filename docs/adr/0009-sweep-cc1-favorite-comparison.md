# ADR 0009: 自動sweepによるCC1の変更をFavorite重複判定から除外する

- 状態: 採用
- 確認: userは「重複判定から除外」「Vaporizer2のCC1との対応」「TyrellN6のUI_op確認」を承認。演奏パターンが違う音色は別Favoriteとして保存する。

## 問題と決定

保存済み35件のうちFloeが11件、Vaporizer2が7件。FloeはRealistic Celtic Harp、Vaporizer2はAccent Arpで、主な差は自動演奏のCC1が動かすMacro 1だった。configのsequence_modulationはsweep。

復元用stateを加工せず、sweep由来の値だけを重複判定から除外する。音色パラメータの変更、pluginの形式・ID・役割、楽器の演奏パターンの違いは引き続き別Favorite。既存項目を再利用する場合は名前・ID・元stateを保つ。既存ライブラリの一括整理・削除は行わない。

## 自動CC1の由来

audio callbackは、成功したprocessへ実際に渡したCC1イベントの由来を記録する。sweep、固定値、未送信を区別する。設定でsweepを選ぶだけでは除外を許可しない。

pause時はstreamを停止してcallbackの完了を待ち、その由来を楽器instanceに引き継ぐ。音声再接続でまだイベントが送られていない場合は前の由来を保持する。固定値のCC1送信でsweep由来を解除し、Favoriteのstate読み込みでも解除する。復元に失敗して旧stateを戻す場合は旧由来も戻す。

手動FavoriteとAuto Favoは同じ判定を使う。Effectのstateへこの除外は適用しない。

## Floeの適用条件

- CLAP ID `com.floe-audio.floe`、Floe 2.0.2、state schema 30のみ。
- 上流タグ[v2.0.2のstate_coding.cpp](https://github.com/floe-audio/Floe/blob/80066c2f1f9a3d716c93b6e246be87568c3b0fa0/src/common_infrastructure/state/state_coding.cpp)を参照し、可変長メタデータ、parameter表、Macro定義、IR、FX、CC割り当て、extrasを順に読み取る。ユーザーのstateの固定offsetを本番処理に使わない。
- CC1の割り当てがMacro 1（parameter ID 101）だけであることを左右両stateで確認する。割り当て変更、Macroの割り当て先・強度、その他の設定は比較に残す。
- Macro 1の有限値0〜1と、extrasのmodified_from_preset（0/1）だけを除外する。それ以外のbytesと長さは完全一致が必要。
- 未知のversion、切り詰め、不正な長さ、重複parameter ID、未確認のCC1割り当て、余分な末尾データは除外を適用しない。
- 保存済み11件ではMacro値4bytesとmodified_from_preset 1byteだけが異なった。実機へCC1 0/32/64/127を送信するとMacro値が0、32/127、64/127、1に変わった。初回ロードとCC1を同時に処理すると復元の適用待ちと競合するため、測定ではロード後のsettlingを必要とする。

## Vaporizer2の適用条件

- CLAP ID `com.vastdynamics.VAST2`、`VC2!`ラッパー、`VASTVaporizerParamsV2.20000`のみ。既存のMSEG微小誤差の扱いは維持する。
- sweep由来のCC1がある場合、root直下のPARAMでidが`m_fCustomModulator1`のtext属性だけを除外。有限値0〜100に限定する。Macro 2〜4、Master Tune、他の属性・配置は比較に残す。
- 実機のCC1 0/32/64/127で保存値0/25.1968/50.3937/100を確認。既存7件にはMacro差と既知のMSEG微小誤差があり、このルールで1件にまとまる。
- MIDI割り当ての全設定を標準APIから取得する仕組みは現時点ではない。確認したMacro 1との対応に限定した例外であり、別パラメータへ再割り当てしたCC1の値は除外しない。

## 制限と撤去条件

送信元はhostのCC1まで追跡し、plugin内部の各parameter変更の送信元は追跡しない。sweepがMacro 1を制御している間のMacro 1手動編集も比較から除外される。sweep由来がない保存ではMacro 1の差を残す。

復元用ファイルには最初に保存した瞬間のMacro値を保持する。音色用stateから自動演奏の値を分離する標準APIが利用可能になったら、本例外の置き換えを検討する。Floeの未知の版や別製品に適用範囲を広げる前に再測定する。

## TyrellN6 UI_opの調査

確認済み9/10の比較は[ADR 0011](0011-tyrell-n6-ui-operation-favorite-comparison.md)で扱う。以下はその前提となる調査結果。

- CLAP build 16976で、CC1 0/32/64/127によってUI_opは変わらなかった。
- UI_op 9/10のstate読み込み直後は指定値を保存することがあり、処理・main-thread pump後には9が10へ変わる場合を確認。editorの表示・非表示だけでは値の変化を再現しなかった。
- ローカルのskin scripts、preferences、CLAPの94 parameterを確認したがUI_opの定義は得られなかった。plugin内の文字列ではViewHost/UI管理コード付近にあるが、それだけで意味を確定しない。
- UI_opと空行の差は今回除外しない。従来の圧縮領域の例外は維持する。9/10が同じ音色であるという前回の推定は確定事項ではない。

## 検証

- unit tests: sweep未送信・固定値・失敗processでは除外を有効化しない、Macro以外の変更保持、未知形式・不正データ拒否、名前・ID・元bytes保持、演奏パターンの別登録。
- 保存済みFloe 11件を一時ライブラリで2件（演奏パターンごと）にまとめる。
- Vaporizer2実機: 保存済み7件を一時ライブラリで1件にまとめ、CC1各値でも増殖しない。Master Tune 440→432は別Favoriteになる。
- ユーザーのconfig、Favorite indexとsnapshotは読み取りのみ。実機probeは`tests/native_auto_favo_probe.py`。`--ui`は新規のテストinstanceのeditor表示・非表示を測定する任意のオプション。
- 初回の実機回帰で、最新snapshotの再保存が完全一致となり、以前の「必ずdriftが出る」というテスト前提が失敗した。安定済みsnapshotも正常として扱い、MSEG差のunit regressionと保存済みsnapshotの比較は維持した。
- 通常テスト86件成功（実機依存12件は通常実行で除外）。Floeの保存済みsnapshot回帰とVaporizer2実機回帰を個別実行して成功。native probeで両pluginのCC1各値もassertして成功。`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo build --release`成功。
- 既定の既存target/shimキャッシュを使用し、新規環境でのビルド再現性確認は含まない。試行用ビルド設定の変更はなく、失敗した実機テストの一時ライブラリは除去済み。
