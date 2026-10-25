# ADR 0040: 音高が鍵盤に追従するpercussionを実測表でpitched percussionへ移す

- 状態: 採用
- 決定者: user（clap等と別にpitched percussionを設けること、対象をdrum/percussion行の全pluginにすること、移す条件を`tracks_keyboard`だけにすること、計測結果をcat-music-patternsで共有すること、欄名をpitched percussionにすること、名前でmallet・bellと分かる音色はその欄を優先すること）、agent（照合キー、表の取得と保存の流れ）

## 目的と判断

共通selectorは音色名・categoryで用途を分類するため、マレット系やDexedの`PERCSYN`のように旋律を弾ける音色が、名前の`perc`等でclapと同じdrum/percussion行に入る。音色名では用途を分けられないので、drum/percussion行の音色を全pluginで実際に鳴らし、note 48/49/55/60で音高がnoteに追従するかを測った表を使う。表で`tracks_keyboard`がtrueの音色だけを、leadの`pitched percussion`へ移す。ただし名前でmallet・bellと分かる音色は、より狭いその欄へ入れる。欄名をchromatic percussionにしないのは、移す音色がtimpaniやPERCSYNまで含み、狭義のchromatic percussion（鍵盤打楽器）より広いためである（user判断）。

却下した案:

- 用途をuserが確認した音色だけを移す手動フラグ方式。userの指定した音色しか移らず、目的を果たさなかった（user判断で不合格）
- Dexedだけを対象にし、DX7パラメータの内容hashで照合する方式。他pluginの同種の音色が残り、hashはplugin共通の識別子にならない
- 減衰・音高の明瞭さ等の軸を足すこと。userが判定軸を`tracks_keyboard`だけに決めた

## 採用範囲と制限

対象は、元の分類でdrumのpercussion行に入る音色だけ。kick・snare・hi-hat行の音色は、表でtrueでも移さない。照合キーはcatalogの`plugin_id`と`display`で、全pluginに共通に使える識別子がこれだけだからである。改名・移動した音色は表に一致せず、元の分類のまま残る。表に無い音色（計測失敗・未計測）も元の分類のまま残す。

共有表は`cat-music-patterns/patch-pitch-tracking.json`に置き、音声開始後の既存catalog workerで取得する。形式を検査してからcacheを更新し、取得に失敗した場合は前回保存した表、それも無ければ元の分類を保つ。catalog読込には最大3秒の待ちが加わる。分類はcategoryとして共通selectorへ渡すだけで、音色state・Favorites・History・catalog保存形式は変えない。表の生成はclap-mml-render-tuiの`build-pitch-tracking`が行い、本アプリは計測コードに依存しない。表の型と照合は、clap-mml-render-tuiの`cmrt-patches`（`pitch_tracking`）をcmrt・cat-smf-editorと共有する。アプリごとに照合を別に持つと、同じ表から違う欄ができるためである。置き場所を`cmrt-patches`にしたのは、cat-smf-editorがCLAPホスト一式を持たず、分類のcrateだけに依存しているためである。取得とcacheは各アプリが持つ。

`tracks_keyboard`の閾値と推定方式は校正していない。外れは耳で見つけ、名前を個別登録せず判定方式の見直しへ戻す（user判断）。
