# ADR 0010: Vaporizer2 の MSEG 再計算差の許容幅を修正する

- 状態: 採用
- 契機: userからVaporizer2のAuto Favoが同じものを追加するとの報告。

## 原因と決定

保存済み3件を読み取りで比較した。`Vaporizer2 1` と `auto Vaporizer2 2` はCLAP、同じ`fmaj7_g6_arpeggio`で、XML要素・属性配置と他の設定が一致する。差はsweepのMacro 1と、MSEGの時間・座標15属性だけだった。`auto Vaporizer2 1` は音色と演奏パターン`steps`が異なるため別Favoriteである。

旧判定の絶対差`1e-9`は、今回のMSEG差を扱えなかった。時間の最大差は`0.000105000030000135` ms（Decay 0との比較）、座標の最大差は約`9.61795e-7`。Attackでは約`1.03812e-6` ms、別MSEGのAttack/Decayでは約`7.65090e-7` msの差もある。

既存の対象属性だけ、時間の有限値の絶対差を`0.00011` ms以下、正規化座標`xVal`を`0.000001`以下へ変更する。相対誤差やMSEG全体の除外は導入しない。PARAM、`yVal`、カーブ、ループ、対象外の点・属性は比較に残す。Macro 1の差はADR 0009のsweep由来が確認できる場合だけ除外する。保存・復元する元stateは加工しない。

対象は引き続きCLAP ID `com.vastdynamics.VAST2`、`VC2!`ラッパー、`VASTVaporizerParamsV2.20000`、`chunkData/msegData0`〜`msegData4`のAttack/Decay/Release時間と直下の`msegPoint1`・`msegPoint2`の`xVal`に限定する。未知形式・対象位置・不正データはバイト完全一致へ戻す。

## 根拠と限界

[上流VASTMSEGData.cpp](https://github.com/VASTDynamics/Vaporizer2/blob/1c56c4be304255f1c397dad725ea784f15a55ef5/VASTvaporizer/Source/Engine/VASTMSEGData.cpp)の`calcADSR()`は座標から時間を再計算し、`doADSR()`はfloatへの座標変換、安全用の下限、座標の再正規化を行う。保存に使われる時間・座標は再計算されるため、独立した安定値ではない。

今回の保存済み差はその再計算に整合する。ただし単純なload/save、editor表示、ノート・CC1処理では今回の大きな差そのものは再現せず、従来の約`1e-10`の差だけを再現した。差が発生した具体的操作は未特定であり、上流ソースの参照リビジョンをインストール済みバイナリのビルドリビジョンと同一とは扱わない。

許容幅以内の意図的なMSEG時間・座標編集は区別できない。時間の幅は0.11マイクロ秒、座標は正規化範囲の100万分の1である。他のPARAMの編集は別Favoriteとして残る。さらに大きな差が発生した場合は再調査し、根拠なく許容幅を拡大しない。安定した音色用stateを標準APIで取得できる場合は本例外を撤去する。

既存Favoriteの一括削除は行わない。同一と判定した保存では既存ID・名前・元snapshotを再利用するため、既存の2件は一覧に残る。

## 検証

- 今回のMSEG差を値そのままのfixtureで再現し、左右両方向で同一判定する。
- 許容幅を超える時間・座標、Master Tune、`yVal`の変更は別判定する。
- 実機回帰は最初のsnapshotと同じ演奏パターンを対象にし、異なるパターンを重複と扱う旧テスト前提を修正する。
- 通常テスト91件成功（実機依存13件は通常実行で除外）。Vaporizer2実機回帰1件成功：今回の2件を一時ライブラリで1件にまとめ、CC1 0/32/64/127でID・名前・元snapshotを再利用し、Master Tune 440→432は別Favoriteになる。
- `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo build --release`成功。既定の既存target/shimキャッシュを使用し、新規環境での再現性確認は含まない。試行用ビルド設定変更・残った一時ライブラリはない。
- ユーザーのFavorite index・snapshotと設定は読み取りのみ。実機の保存は一時ライブラリで行う。
