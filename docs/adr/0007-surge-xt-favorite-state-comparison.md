# ADR 0007: Surge XTのFavorite重複比較

状態: 採用

## 根拠

Auto Favoの3〜5付近で同じ音色が増える報告を受け、保存済みstateを読み取り専用で比較した。

- auto Surge XT 3 / 4: VST3、Cute 1、同じfmaj7_g6_arpeggio。差は`patch/dawExtraState/instanceZoomFactor/@v`の125 / -1だけ。
- Surge XT 2 / auto Surge XT 5: CLAP、Guitar、同じguitar_arpeggio。差は`patch/modwheel/@s0`・`@s1`の0 / 0.33858266だけ。
- auto Surge XT 1と手動Surge XT 2はstateが完全一致するが演奏パターンが異なる。従来仕様どおり別項目として保持する。

上流1.3.4の[SurgePatch.cpp](https://github.com/surge-synthesizer/surge/blob/release_xt_1.3.4/src/common/SurgePatch.cpp)はmodwheelにControllerModulationSourceのtarget値、instanceZoomFactorにeditor拡大率を保存する。
stateは32バイトsub3ヘッダ、指定長XML、6枠のwavetableデータからなる。
ローカルVST3の2件には末尾に16バイトのゼロとJUCEPrivateDataがある。

## 決定

`src/plugin_specific/surge_xt.rs`に製品固有比較を置く。
対象IDはCLAP `org.surge-synth-team.surge-xt`とVST3 `ABCDEF019182FAEB566D624153675854`。
sub3ヘッダ、長さ、UTF-8、単一rootのpatch revision 24を確認する。
VST3のみ測定済みの固定JUCE末尾データを許可し、比較にも完全一致を要求する。
XMLの構造、属性名・順序、テキスト、その他の値、wavetable長と全バイトは一致を要求する。
上記の正確なパスの属性だけ比較から除外する。
CC1は有限の0〜1、拡大率は整数-1または1〜1000のみ許可する。
未知製品・形式・revision、不正長・XML、未知末尾データは共通処理のバイト完全一致に戻す。

プラグイン形式・ID・役割・演奏パターンの一致条件を維持する。
同一なら既存のID、名前、番号、保存済みstateを再利用する。保存・復元は元データ全体を使う。
既存Favoritesの削除や書き換えは行わない。
CC1位置による演奏上の音の差を別音色としない制限がある。
モジュレーションのルーティング・深さ、ピッチ、チューニング、他のdawExtraStateは除外しない。
上流で安定した音色用stateを取得できるようになれば撤去を検討する。
別revisionへ広げる前は保存差の調査と実機検証が必要。

## 検証

- 合成stateで限定パス、音色・tuning・wavetable変更、不正値・長さ・XML・未知revision、VST3末尾データを確認。
- 保存済みの2組を読み取り、一時ライブラリで各組が1件にまとまることを確認。
- Surge XT 1.3.4 CLAPで6回の手動/自動保存、再読み込み、ノート・CC1後も1件、既存名・ID・元state保持を確認。
- oscillator 1のピッチを0から-6へ変更し、音声処理でqueued stateを適用後、別Favoriteとして保存されることを確認。
- VST3は保存済みstate比較を検証。今回editor操作による拡大率変更の再現は未実施。
- 通常テスト79件、上記実機テスト1件、Clippy、fmt、`cargo build --release`成功。既定の既存キャッシュを使用。新規環境での再現性は未確認。
- ユーザーのFavoritesは変更せず、一時ライブラリのみ書き込み。
