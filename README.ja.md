# cat-plugin-player

オーディオプラグインを手軽に演奏できる軽量アプリ。[UAPMD](https://github.com/atsushieno/uapmd)を利用しています。Rustで書かれています。

# これまでの課題と、このアプリが解決すること
- オーディオプラグインの音色は素晴らしいですね！
- 演奏するには、DAWを起動し、演奏のための前準備が必要なことが多いです。
- 手軽に演奏するための軽量アプリは、入手のためのアカウント登録が必要なことがあったり、操作感覚がnot for meだったりしました。
- 現代は、ほしいUXがあれば、バイブコーディングで実現できます。
- そうすることにしました。

# ダウンロード

[Windows版をダウンロード](https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip)

# 説明動画

もしオーディオプラグインをインストールしていないなら、このようにアプリが自動で[Surge XT](https://surge-synthesizer.github.io/)をインストールします。かんたん！

https://github.com/user-attachments/assets/c4a7cec0-ebfd-4cea-bc21-bbfc4dd8e92d

# 機能
- 自動インストール
  - もしオーディオプラグインをインストールしていないなら、前述のようにアプリが自動で[Surge XT](https://surge-synthesizer.github.io/)をインストールします
- 検出
  - 起動すると、installされているプラグインを自動で検出します
- 演奏
  - プラグインをクリックすると、プラグインload後、自動で演奏を開始します
  - `F-G-Em-Am`のようなコード進行、`cde`のようなMML(Music Macro Language)も演奏できます
- GUI
  - `Show UI` ボタンを押すと、プラグインのGUIを表示して、音色やエフェクトのエディットができます
- 音色
  - プラグインそれぞれが多彩なプリセット音色を持っています。それらを、独自の音色ブラウザでわかりやすく選べます（一部プラグインのみ対応）
  - 音色ブラウザに未対応のプラグインは、普通にプラグインのGUIで音色を選べます
- エフェクト
  - プラグインそれぞれが多彩なプリセットエフェクトを持っています。それらを、独自のエフェクトブラウザでわかりやすく選べます（一部プラグインのみ対応）
  - エフェクトブラウザに未対応のプラグインは、普通にプラグインのGUIでエフェクトを選べます  

※↑このセクションは人間が書きます。AIがここに書くことを禁止します

# AIが編集するセクション

読みづらいのであとでなおすつもり。折りたたんでおきます

<details><summary>機能の詳細、ほか</summary>

## 機能（AIによる詳細な追記）

オシロスコープの表示方式と制約は[ADR 0001](docs/adr/0001-scope-follows-waveform-position.md)に記録しています。

- 検出
  - 通常起動とeditor連携では、`%LOCALAPPDATA%\cat-plugin-player\plugin-list-cache.json`から全プラグイン一覧と音源／effectの分類を先に復元します。キャッシュが無い・読み込めない初回は検出して作成します。起動後、読み込みや音色変更がなく操作が2秒間止まると、バックグラウンドで再検出します。一覧に変化がある場合だけキャッシュとpatch一覧を更新し、変化が無ければ表示を維持します。再検出に失敗した場合は既存キャッシュを保持します
  - FluidSynthPlugin VST3は、非標準のカテゴリ表記への互換対応によりinstrumentとして分類します。以前「不明」になっていた一覧は、再検出後に更新されます。対応範囲と理由は[ADR 0041](docs/adr/0041-fluidsynth-vst3-category-separator.md)を参照してください
  - プラグインの検出が成功して音源（instrument）が1つも無いときは、無料の Surge XT を winget でインストールする案内を表示します。effectだけがある場合も表示します。Windows の表示言語が日本語なら日本語、それ以外は英語で表示します。`あとで` を選んでも、音源が無い間は起動のたびに表示します
  - `利用規約を見る` で Surge XT の利用規約（GPLv3）をブラウザで開けます。`利用規約に同意してインストールする` を押すと winget を実行し、「1/4 準備」「2/4 ダウンロード」「3/4 インストール」「4/4 読み込み」の段階と経過秒数を表示します。3/4 で Windows の確認画面（ユーザー アカウント制御）が出たら「はい」を選んでください。成功するとプラグイン一覧を自動で再スキャンし、Surge XT（CLAP）を読み込んで `Load` と同じく演奏を始めます。このときは前回の音源・effectの復元は行いません。案内を閉じてもインストールは続き、終われば再スキャンと読み込みを行います
  - winget が無い・インストールに失敗した場合は、winget のメッセージと手動ダウンロード先を表示し、`error.log` に記録します。入るのは Surge XT だけで、ダウンロードの割合は表示しません。選んだ理由は[ADR 0024](docs/adr/0024-offer-surge-xt-via-winget.md)にあります
- GUI
  - GUIを初めて開くと、描画を2秒待って自動でスクリーンショットを撮り、縦横比を保って一覧の表示枠と画面の拡大率に合わせたサイズのPNGに縮小します。`%LOCALAPPDATA%\cat-plugin-player\plugin-icons\` に保存し、名前＋メーカーが一致するプラグインはVST3 / CLAPでも同じ画像を共有します（前後の空白・大文字小文字は無視）。名前・メーカーが不足する場合は形式＋IDで区別します
  - 以前保存した形式別の画像も、一覧・お気に入りの識別情報から共有画像へ自動移行して使います。音色stateとお気に入りのロード先は従来どおり形式＋IDで区別します
  - Plugins / Favorites / Routing にプラグインの画像を表示します。行内ではボタンの高さに収め、縦横比を保って小さく表示します。画像にマウスを重ねると保存サイズで確認できます。GUI未表示のプラグインは同じ高さの仮アイコンを表示します
  - GUIは本体の右隣・上端合わせで画面の作業領域に収まる位置を優先します。収まらない場合は本体がある画面の作業領域の右下に配置します。表示時とGUIのサイズ変更時に配置し、本体の移動には常時追従しません。GUIは縮小せず、画面より横幅が大きい場合は左端、縦幅が大きい場合は上端を作業領域に合わせます
  - 白紙画像や撮影失敗時は1秒間隔で、GUI表示から最大20秒まで再試行します。保存済みの白紙画像も未撮影として扱い、GUIを開くと有効な画像で置き換えます。成功後は再撮影しません。
  - 再試行しても撮影に失敗した場合はエラーを表示します。GUIを開き直すと再試行します。撮影非対応のGUIでは仮アイコンのままです。撮り直したい場合や保存PNGが破損した場合は、該当プラグインの共有PNGと残っている旧形式別PNGを削除してアプリを再起動し、GUIを開いてください（移行時には旧PNGを残します）
  - `?` または `F1` で、そのウィンドウ（本体・Browse patches・Browse effects）のキー操作一覧をoverlay表示します。機能ごとに色分けしています。本体のhelpには `d`／`a` のメニュー入口を載せ、各項目キーは対応するメニューを開いて `?`／`F1` または `Help (? / F1)` ボタンで表示します。もう一度 `?`／`F1` または `Esc` でhelpだけを閉じ、元のメニューへ戻れます。表示中は他のキー操作が効きません。文字入力中の `?` は文字として入力されます
  - 本体ウィンドウの英字1文字のキー操作（各機能の説明に記載）は、MML入力画面・Browse patches／Browse effects表示中・文字入力中は動作しません。`u` キーでは、Routingの音源の `Show UI`／`Hide UI` と同じく音源GUIの表示・非表示を切り替えます（effectのGUIはボタンで操作します）
- 演奏
  - Surge XTは通常、音声処理を再構築したときに最初のフレーズ開始を100ms待ちます。CLAPのcatalog音色のうち配布判定表で許可された音色は0msとし、そのstateに紐付けて次回起動にも引き継ぎます。表はGitHubから背景で取得し、`%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt-start-delay.json`に保持します。同梱はせず、初回・未掲載・照合不一致は100ms、通信失敗時は保存済みの表を使います。プラグインGUIを開くと編集を追跡できないため許可を解除します。GUIを閉じてブラウザーから既知の音色を選び直すと再判定します。0msはUXを優先した採用判断であり、実測条件と制限は[ADR 0008](docs/adr/0008-surge-xt-automatic-note-start-delay.md)を参照してください
  - 最後に演奏した instrument と接続中の effect・Bypass 状態・演奏パターンを `%LOCALAPPDATA%\cat-plugin-player\status.json` に保存し、次回起動時に復元します。停止中の演奏パターン選択、確定したMML／コード本文も復元します。起動時にPlugins一覧で音源候補が1つだけなら、未ロードの場合は自動ロードし、保存された停止状態でも選択済みのパターンで演奏を始めます。複数候補の場合は停止状態を復元します。起動後に手動で停止した場合は停止を維持します。未確定の編集は保存しません
  - CLAPのNote On/Offはプラグインが対応する入力形式に合わせて送信します。MIDIのみを受け付けるsforzando CLAPもこの対象です。補修の適用範囲と検証記録は[CLAP Note形式の説明](patches/uapmd/clap-note-dialect/README.md)に記録しています
  - 左上の `Sequence` pane で、左右矢印またはドロップダウンから演奏の種類を選べます。矢印は種類の両端で循環し、停止は種類に含めません。本体ウィンドウの `s` キーでも次の種類へ循環します。演奏中の切替は鳴っているフレーズの全音を解放し、新しいフレーズの先頭から開始します。停止中は種類を選んでも演奏せず、`Play` で開始、`Stop` で停止します。本体ウィンドウでは `Space` キーでも開始・停止を切り替えられます（文字入力中を除く）。MML／コードを含む演奏パターンの選択と停止状態は次回起動時にも復元します。ただし音源候補が1つだけの起動時は停止状態から自動再生します
  - MML／コード進行の試奏は、`i` キーまたは `Sequence` の `Chord` ボタン（ホバーでショートカットと入力方法を確認できます）で入力画面を開きます。単音の例は `t120 c8r8d4e4r4`、コード進行の例は `C F G C` です。同じ入力欄で両方を受け付け、コード表記の変換を先に試します。大文字 `C` はコード、小文字 `ceg` はMMLになります
  - 入力は `Enter`／`Confirm` で確定し、`Esc`／`Cancel` で取り消します。取消しや変換エラーは直前の有効なフレーズを維持します。入力画面内の `Space` や `i` は文字入力に使い、`Space` による再生・停止は動作しません
  - 確定したフレーズは `Play`／`Stop` でループ再生します。休符・音長・テンポ変更・和音・末尾休符を保ちます。入力画面を開くと全体の通常演奏を一時停止します。開く直前に演奏中だった場合、確定後は新しいフレーズ、取消後は元の演奏パターンを先頭から再生します。停止中に開いた場合は確定・取消後も停止を維持し、`Play`で全体再生を開始します。通常の演奏パターンにも切り替えられます。確定直後に本文と選択・再生状態を保存し、取消時も戻した再生状態を保存します。次回起動時や音源のお気に入り・Historyから復元します。未確定の本文は保存せず、取消しや変換失敗では直前の確定本文を保ちます
  - 入力変更やカーソル移動で、その位置の単音／和音を一度試聴します。途中編集や貼り付けも対象です。同じ発音単位の内側での移動や再描画だけでは鳴らし直しません。前のテンポ・オクターブ・音長を反映し、休符やコマンドの位置では発音しません。編集を開いた時点で通常演奏は停止し、試聴音の終了後も全体再生は始まりません。次の発音単位、取消し、確定、音源交換でも試聴音を解放します。試聴は確定本文・通常の選択を変えません
  - 確定したMMLのループ再生の発音Velocityは、入力内の指定より `Velocity` 欄の設定を優先します。CC1も `CC1 modulation` 欄から操作できます。非常に過密なフレーズで演奏イベントの容量に達した場合は、鳴っている音を解放して発音を停止します。`Stop` → `Play` で再開始できます
  - 通常起動でのFavorites／Historyからの呼び出しやBrowse patches／Browse effectsによる音色変更では、停止・再生状態を維持します。再生中はフレーズの先頭から演奏し、変更前のフレーズ途中の待ちを引き継ぎません。MMLで指定した先頭休符は保ちます。音色ブラウザー・effectブラウザーの表示中は背後のSpaceによる再生・停止とMML入力開始を抑止します。プラグイン独自GUIの音色変更は検出しません
  - SMF選択モード（`--select-smf`）では、入力trackをtempo・duration・velocity付きでloop再生します。一括音色指定では全trackを選択中の音色と代表trackのeffectで試聴します。通常sessionの音色・effect・MMLは復元せず、お気に入りでもsound snapshotだけを使います。patch／effect／お気に入り・接続変更で先頭へ戻さず、交換中の短い無音の後に停止位置から続けます。MML／通常パターンの操作欄はこのmodeでは使いません
  - SMF選択モードの`--master-effects`では、現在trackの音色・effectを保持して試聴し、master effectだけを編集・返却します。patch browserは使いません。masterはSMF format 1のconductor専用metaに保存し、全trackへの適用結果はeditorへ戻って確認します。format 0/2のmaster対応は追加していません。[ADR 0030](docs/adr/0030-master-effects-selection.md)を参照してください。
  - cat-smf-editorは`--resident <名前>`でplayerを1つ常駐させ、`t`／`x`のたびにpatch／effect browserを前面に出させます。常駐中のplayerは選んだ音色・effectを自分では読み込まず、stateをeditorへ送り、editorが自分の音源で鳴らします。Esc・`q`・×・effect browserを閉じるEnterはbrowserを隠すだけで、playerはeditorが終了すると一緒に終了します。前回の音色は復元せず、playerの通常のstatus.jsonも更新しません。Role／Preset・Category／Kindと検索文字列（Solo／Mute・除外条件を含む）は、editorのstatus.jsonへ曲・track・patch／effect別に保存し、master effectは曲ごとに独立して保存・復元します。音色を選ばず閉じても保存します。patch一覧の選択もpatch名と音源・元patchの識別情報でeditorのstatus.jsonへ曲・trackごとに保存し、再起動後に復元します。選択の復元では音色を再適用しません。
  - Six Sines／TyrellN6 CLAPでは、ロード・state復元・接続変更などで音声処理を再構築した際、冒頭音を保つため、通常の音声処理を続けながら最初のフレーズ開始を待ちます。48kHz時の待ちはSix Sinesで約21ms、TyrellN6で約85msです。MMLの先頭休符はさらに加算されます。適用条件と実機検証は[ADR 0019](docs/adr/0019-random-patch-all-catalog.md)を参照してください
  - 演奏パターンの下の `Velocity` 欄で、同じ左右矢印・ドロップダウンから `100`・`127`・`40-100`・`80-127` を選べます。初期値は100です。`v` キーで次の選択肢へ循環します。範囲指定はフレーズ内のNote On順に値を上げ、次のループでは下げる動作を繰り返します。変更はフレーズを再開せず次の発音から反映し、次回起動時にも復元します
  - `CC1 modulation` 欄で、左右矢印・ドロップダウンから `-`・`0`・`127`・`sweep` を選べます。`w` キーで次の選択肢へ循環します。初期値の `-` は何も送らず、音源のmodulation wheelの値を変えません。`sweep` は2秒で0から127へ、次の2秒で127から0へ変化する動作を繰り返します。欄の右の値表示バーを横にドラッグすると、ドラッグ開始時の値から増減した0〜127の値を送ります（バーの幅で0〜127を動きます。`sweep` 中なら止まり、欄はその値になります。クリックだけでは変わりません）。選択とドラッグした値は次回起動時にも復元します
  - Surge XTで、現在のpatchがmodulationに使っていてCC番号が分かるMacro（Macro 1〜8）ごとに、`CC1 modulation` 欄の下に `CC41 Macro 1` のような欄が出ます（番号はSurge側の割り当て。既定はMacro 1〜8が41〜48）。patchがMacroに `LFO Speed` のような名前を付けているときは、`CC41 Macro 1` の代わりにその名前を出します（名前にマウスを乗せると `CC41 Macro 1` を表示）。選択肢と値表示バーのドラッグはCC1と同じで、`sweep` はMacroごとにCC1とも互いとも違う周期で繰り返します（0から127までの片道がMacro 1〜8でそれぞれ2.9 / 3.7 / 4.3 / 5.3 / 6.1 / 7.1 / 7.9 / 8.9秒。CC1は2秒）。初期値の `-` は何も送らず、patchに保存されたMacro値を変えません。Surge XTでは、`-` のときの値の表示に、patchを変えたときと選択を `-` に変えたときに読み直したpatchの値を出します（Surge XTのGUIでツマミを動かしても表示は変わりません）。Browse patches・Favorites・History、またはSurge XTのGUIでpatchを変えると、CC1とMacroの `0`・`127`・ドラッグした値は `-` に戻り、新しいpatchに保存された値で鳴ります（`sweep` は続けます）。`M` キーでMacro 1〜8をまとめて `sweep` にし、表示中のMacroが全部 `sweep` のときにもう一度押すとまとめて `-` に戻します。選択はCC1と同じく保存され、次回起動時も引き継ぎます。Favoriteの重複判定はCC1と同じくMacro値の違いを無視します。Surge XTのGUIでpatchを変えた場合も、すぐに欄の表示・非表示が切り替わります。詳細は[ADR 0036](docs/adr/0036-surge-xt-macro-sweep.md)を参照してください
  - TyrellN6（CLAP）でも、現在のpatchがControl A・Control Bをmodulationに使っているとき、同じ場所に `CC2 Control A` のような欄が出ます（番号はpatchに保存されたControl A・Bの割り当て）。patchのUsage欄に `Ctrl A = bass dominator` のような説明があるときは、見出しをその説明にします。選択肢・ドラッグ・`M` キー・保存・patch変更時の動作はSurge XTのMacroと同じで、`sweep` の片道はControl Aが2.9秒、Control Bが3.7秒です。`-` のときの値の表示は送った値で、patchの値は出せません。TyrellN6のGUIでpatchを変えても欄は切り替わらず、Browse patches・Favorites・Historyでpatchを変えたときに切り替わります。詳細は[ADR 0038](docs/adr/0038-tyrell-n6-control-ab-sweep.md)を参照してください
  - Vaporizer2・Six Sines・Floe（CLAP）でも、現在のpatchがmodulationに使っているMacroごとに同じ場所に欄が出ます（Vaporizer2はCustom modulator 1〜4、Six SinesはMacro 1〜6、FloeはMacro 1〜4）。見出しはpatchがMacroに付けた名前で、名前が無ければ `Custom modulator 1`・`Macro 1` のように出します（CCで送らないのでCC番号は付きません）。この3音源はCCではなく音源のparameterを直接動かし、欄の0〜127を音源の値域へ写して送ります（Six Sinesは-1〜1、ほかは0〜1）。選択肢・ドラッグ・`sweep` の周期・`M` キー・保存・patch変更時の動作はSurge XTのMacroと同じです。`-` のときの値の表示は、patchを変えたときと選択を `-` に変えたときに読み直したpatchの値です。Floeで、patchがCC1を割り当てているMacro（工場出荷時はMacro 1）は欄に出ず、`CC1 modulation` 欄で動きます（2つの欄が同じMacroを取り合わないため）。音源のGUIでpatchを変えたときは欄が切り替わらないことがあり、Browse patches・Favorites・Historyでpatchを変えたときに切り替わります。Favoriteの重複判定はMacroの値の違いを無視します。上記以外のプラグインのMacroは未対応です。詳細は[ADR 0039](docs/adr/0039-parameter-macro-sweep.md)を参照してください
  - ギター開放弦は MIDI note number 40・45・50・55・59・64（ミ・ラ・レ・ソ・シ・ミ）を125ms間隔で順にNote Onします。最後のNote Onから2秒後に6音を同時にNote Offし、0.5秒後に繰り返します
  - `Load` 成功時、`Sequence` の種類選択・開始・停止時に記憶します。停止や `Remove` 後も記憶は残ります。古い履歴・お気に入りは従来の4音パターンで復元します
  - 通常起動の終了時と `Remove` 時には、UAPMDのstate APIでプラグインが公開する音色・パラメータ等のバイナリstateを `%LOCALAPPDATA%\cat-plugin-player\states\` にプラグインごとに保存します。次回起動時や同じプラグインの再load時には、演奏開始前に復元します
  - instrument の `Load` は音源だけを置き換え、接続済み effect と設定は維持します。置き換えるプラグインの state を保存し、新しいプラグインの生成・復元・接続に成功してから切り替えます。失敗した場合は元の構成を維持してエラーを表示します。起動時の最小構成の Surge XT を同じ音源へ読み直す場合だけは、下記の `Show UI` と同じ保全・再生成・回復経路を使います
  - stateの保存範囲はプラグインとUAPMDの形式別実装によります。UI stateも要求しますが、現在のUAPMDのVST3実装はcomponent stateのみを取得します。外部サンプルの複製や、強制終了時の保存は含みません
  - 起動時は保存済みの全プラグイン一覧を読み、前回の instrument と effect を直接ロードしてstateを復元し、演奏を開始します。その後にGUIを初期化し、操作が落ち着いてから一覧を背景で再検出します。全一覧のキャッシュがない初回はGUI表示後に検出します
  - 前回の instrument が Surge XT（CLAP / VST3）のときは、起動時のこの1件だけを、patch・wavetable・FX preset の一覧を省いた構成で作って早く鳴らします。この構成のままでは Surge のGUIのメニューがほぼ空になるため、その Surge の `Show UI` を押すと、最新の音色・編集内容・Favorite 選択・演奏設定・接続中のeffectを保って通常構成で作り直し、GUIを開きます。その間は音が一度止まります。保存できなければ作り直さず元の音源を維持します。生成・復元・接続に失敗した場合は保存した内容から1回だけ回復し、エラーを表示します。回復も失敗した場合は保存済み内容を残して停止するので、`Load` で再試行してください。連打しても作り直しは増えません。`Load`・音色ブラウザー・お気に入り・History・effect で作る Surge XT は通常の構成です。環境変数 `CMRT_SURGE_MINIMAL_DATA_HOME=0` で無効にできます。理由・待ち時間の実測と検証範囲は[ADR 0034](docs/adr/0034-surge-xt-startup-minimal-data-home.md)を参照してください
  - 古い履歴やファイル移動などで直接ロードできない場合は、GUI表示後の走査結果から復元します。古い履歴は一度演奏すると更新されます
  - 主音源がCLAP Floeの場合は、ロードや音色復元などで音声処理を再構築した際、最初のフレーズの開始を100ms遅らせます。MMLの先頭休符はさらに加算されます。冒頭音が消える症状の観察用の暫定対策で、実機での解消は未確認です。適用条件と見直し方針は[ADR 0018](docs/adr/0018-floe-automatic-note-start-delay.md)を参照してください
  - 起動高速化の採用経緯、Show UIで発生した不具合と破棄順序の変更は [ADR 0034](docs/adr/0034-surge-xt-startup-minimal-data-home.md)、計測方法は[開発者向けの説明](#起動時間の計測)を参照してください
- エフェクトルーティング
  - `Browse effects`／`E`／`X` を開くと、対応するCLAP effect候補が1つだけなら自動ロードして、その枠をプリセット変更の対象にします。既に接続されている場合は再ロードせず、その枠を使います。音源が未ロードで音源候補も1つだけなら、先に音源をロードして演奏を始めます。音源候補が複数ある場合は先に音源を選んでください。候補数の判定はPlugins一覧のCLAP優先設定に従い、検索文字列では候補数を減らしません
  - instrument 1つ → 複数effect → 出力の直列接続です。instrument を読み込んでから最初の effect の `Load` を押し、以降は `Add` で末尾に追加します。接続済みの effect は `Remove` でその1つだけを外します。同じ形式＋IDの effect を重複して接続することはできません
  - Browse effectsでのeffect変更には音源が必要です。候補はplay-server共有coreが探索する既定のeffect・プリセットの場所から取得し、VST3への代替はしません。準備中は現在の演奏を続け、ロード・走査・音色操作を無効にします。通常起動では停止中は停止、再生中は現在フレーズを先頭から再開し、MML・Velocity・CC1の全体選択と他のeffectの順序・設定・Bypassを維持します。生成・適用・接続の失敗時は元の構成へ復帰します。成功後は適用したstateと接続を次回起動用に保存し、Favorites／Historyは追加しません。保存失敗は適用済みとして表示します
  - `Sequence` 横の `Browse effects` または `E`／`X` キーで、独立した最大化ウィンドウのeffectブラウザーを開き、プリセットを一覧から選べます。MML編集中は開けません。左側のCategory→Kindと検索で絞り込みます。分類と選択対象はcmrtのeffect selectorと同じです。検索は大小無視の正規表現、空白AND、`-条件`による除外で、プリセット名・Category・Kind・effect名のいずれかに当てます（例: `hall -plate`、`dragonfly`）。入力途中の無効な正規表現はエラーを表示し、直前の一覧を保ちます。上部1行にSearch・対象の枠・`Random`が並び、ブラウザー内の `/` で検索欄へ入力を移し、`R` で絞り込み結果からランダムに選び、`W` で列幅を全プリセット名が収まる幅と既定の幅とで切り替えます。一覧の複数列表示、上下左右キー／`h`／`j`／`k`／`l`・数字による回数指定・`PageUp`／`PageDown` での移動、`Enter`／`Esc` で閉じる操作はBrowse patchesと同じです。複数のeffectが混在する結果ではeffect名を付けてeffectごとに、Kindが異なればプリセット名をKindごとに色分けします。セルに収まらない名前は末尾のプリセット名（フォルダを除いた部分）を必ず残し、その左のフォルダ名・effect名を頭1文字＋`…`まで縮め、プリセット名に近い部分から長く表示します。完全な名前はマウスを重ねると確認できます
  - 対象の枠は `Add new effect` か、接続中のeffectの `Replace n` から選びます。開いたときは、前回このブラウザーで書き込んだ枠、無ければ末尾のeffect、effectが無ければ `Add new effect` になります。`Add new effect` で適用すると追加したeffectがそのまま対象になり、以降の選択はその枠を置き換えるので、選び続けてもeffectの個数は増えません。対象の枠を外した場合は末尾のeffectが対象になります。対象以外の枠に接続済みのeffectのプリセットはグレーで表示し、選ぶと理由を表示して変更しません。対象の枠にこのブラウザーで入れたプリセットには `▶` を付け、開くとそこを選択状態にします（再適用はしません）
  - クリック・キー移動、または絞り込みで別の候補が選ばれると、すぐに適用を要求します。開くだけではeffectを変えません。準備中にも次を選べ、未開始の選択は最新1件に置き換わり、閉じると取り消します。Category・Kind・検索文字列・一覧の選択位置と対象の枠は終了時に保存し、次回起動時に復元します。キー移動は復元した選択から始まります。Category・Kindは名前で復元し、無くなっていればその欄の `ALL` に戻ります。選択はプリセットで復元し、カタログから無くなっていれば未選択になり、キー移動は先頭から始まります。対象の枠はchain内の位置で復元し、その位置にeffectが無ければ開いたときの既定の枠になります
  - Browse effectsで置き換え前か置き換え後のeffectがCLAP Surge XT Effectsの場合は、切り替え直前の音を約50msでフェードアウトし、切り替え後の音を約50msでフェードインします。音源の鳴り残りが新しいeffect（特にDistortion系）を通って大きな「ぶちっ」になるのを抑える暫定対策です。切り替えはその分（最大約70ms）遅れ、フェードイン中は鳴り残りが小さく聞こえることがあります。他のeffect同士の切り替えや音色・音源の変更ではフェードしません。理由と適用範囲は[ADR 0027](docs/adr/0027-surge-xt-effect-swap-fade.md)を参照してください
  - CLAP Surge XT Effectsのプリセットは、プラグインを一時的に起動して準備する代わりに、[cat-music-patterns](https://github.com/cat2151/cat-music-patterns)から起動ごとに取得する準備済みstateの表を使います（`%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt-fx-states.json` に保存）。プリセットを選び続けるとアプリごと落ちる不具合を避けるためです。取得できない場合は前回保存した表を使い、それも無ければSurge XT Effectsのプリセットはeffectブラウザーの候補に出ず、理由を表示します。表はSurge XT 1.3.4のプリセットから作ったもので、中身が異なるプリセット（版の違うSurge XTなど）は候補から外し、外した件数を表示します。理由は[ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md)を参照してください
  - `Routing` に接続順を表示します。effect の `↕` ボタンを上下にドラッグして順序を変更できます。ドラッグ中は移動するeffect名と挿入位置の線を表示します。instrument は並べ替えの対象外です。各プラグインの `Show UI` で音色や effect を編集できます。`Instrument`／`Effect` は左側の一覧と同じ種別色で表示します
  - 全effectを外すと音源から出力への直結に戻ります。`Bypass effect` はchain全体を対象とし、instance と設定を保持したまま素通しにできます。`b` キーでも切り替えられます（MML入力画面・Browse patches／Browse effects表示中と文字入力中は動作しません）。Bypass中はRouting内の接続effectをグレーで表示し、`Bypassed` を併記します
  - `Sequence` は instrument 用です。Sequence を止めても音声処理は続け、effect の余韻を出します
  - 次回起動時は instrument・全effectの順序・それぞれの state・Bypass 状態を復元してから演奏します。旧形式のeffect 1件の履歴も読み込めます。effect の復元に失敗した場合はエラーを表示して音源の直結に戻し、保存済みの構成は保持します
  - 追加・除去・順序変更は新しい接続の準備に成功した場合だけ保存します。失敗した場合は元の構成と再生を復帰し、エラーを表示します
  - 主入力・主出力の mono／stereo に対応します。mono → stereo は複製、stereo → mono は左右の平均です。補助入力は無音です
  - 接続や Bypass の変更時には短い音切れがあり、発音中の note と余韻は途切れることがあります。サイドチェイン、並列接続、ホスト側の Dry/Wet は未対応です
- 音色
  - 音源が未ロードで音源候補が1つだけの場合、`Browse patches`／`T` で自動ロードして演奏を始めます。ロード中・走査中の要求は処理が終わってから行い、一覧を閉じた場合は未開始の自動ロードを取り消します。候補が複数ある場合は自動で選びません。候補数の判定はPlugins一覧のCLAP優先設定に従い、検索文字列では候補数を減らしません。ロード失敗は表示し、自動再試行はしません。音色一覧自体の対応形式・カタログの制限は従来どおりです
  - `Sequence` 横の `Browse patches` または `T` キーで、本体のある画面に独立した最大化ウィンドウの音色ブラウザーを開きます。本体の位置・サイズは変えません。MML編集中は開けません。枠付きのRole→Preset→検索で絞り込みます。音色が1つも無いRole・Presetは表示しません。Solo／Muteで絞った後に0件になるもの（例: Dexed以外をSoloしたときの `FM Bass`）も隠します。検索欄の文字列では隠さず、自作条件は削除できるよう0件でも表示します。左側の条件領域と右側のPatches一覧はそれぞれスクロールできます。一覧に残りの幅と高さを使い、最大化解除や狭いウィンドウでも操作できます。上部1行にSearch・`Solo/Mute`・`Random`・`☰`が並びます。検索欄には残りの幅を使い、ブラウザー内の `/` キーで検索欄へ入力を移せます。音源条件の詳細は`Solo/Mute`のhoverで確認できます。このボタン全体のクリックまたはブラウザー内の `M` キーで縦並びのSolo／Mute overlayが開き、`OK` または `Enter` でまとめて確定します。overlay内では、音源名の左に出る英字キーを押すと、小文字はSolo、`Shift`＋英字はMuteを切り替えてその場で確定します（cmrtのpatch selectorと同じ割り当てで、音源名のうちまだ使われていない最初の英字。例: Dexedは `d`、Surge XTは `u`）。Solo／Muteは音源名のすぐ右に並びます。選択中のモードを再クリックすると通常へ戻り、`Esc`／overlay外クリックは変更を破棄します。複数SoloはOR、Muteは除外として扱います。検索は大小無視の正規表現、空白AND、`-条件`による除外、`plugin:`条件を使います（例: `pad -bright`、`plugin:floe`、`plugin:surgext -bass`）。統合済み音色の別名も検索できます。無効な正規表現はエラーになり、適用・抽選・条件保存を止めます。0件なら検索欄の編集やRole／Presetの変更で条件を見直せます。カタログ読込待ち・失敗は0件とは別に表示します。Role・Preset・検索文字列は終了時に保存し、次回起動時に復元します。Presetは名前で復元し、削除済みの条件ならそのRoleの `ALL` に戻ります。復元時やSolo／Mute変更時に選択中のRoleが0件になれば全体の `ALL` に、Presetが0件ならそのRoleの `ALL` に戻ります
  - 一覧は折り返さない1行の音色名セルを、幅に応じた複数列で左→右→次の行へ並べます。狭いウィンドウでは1列へ戻ります。resizeしても選択した音色は維持し、列数の変更だけでは再ロードしません。音色をクリックしても一覧の表示位置は動かしません。上下左右キーまたは `h`／`j`／`k`／`l` は見た目の並びどおりに移動します。左右は前後の音色（行端では隣の行の端）へ、上下は上下の行へ移動し、真下に音色がない短い最終行へはその行の最後へ移動します。vim風に `2j`・`10k` のように数字を先に入力すると、その回数だけ移動します（端で止まります。移動以外のキーで数字は取り消されます）。`PageUp`／`PageDown` は上下10回分移動し、数字との組み合わせではその倍数だけ移動します。キー移動・Random・絞り込み・列数変更では、選択行が画面外の場合だけ見える位置までスクロールします。全音色が収まる場合はスクロールしません。単一音源の結果では音源名を省き、混在時は音源名を残します。文字色は結果内の違いを1つだけ示します。音源が混在すれば音源名部分を音源ごとに色分けします。単一音源では、Presetが `ALL` のとき該当Presetごとに、それで区別できなければcategoryごとに音色名を色分けします。色はMonokai系です。どれも同じなら通常の文字色です。色は5色を順に繰り返すため、離れた別グループが同色になることがあります。pink音色の音色名はpinkを優先します。分類フォルダー等は省略し、同名音色を区別する場合は必要なパスを残します。Dexedのprogram指定は維持します。マウスを重ねると完全な表示パスと適用元のパス、有効なcategory、2件以上の統合件数を確認できます。Load／Sizeは一覧に常設せず、`no category`／`1 merged` も表示しません。通常音色はマウス選択／キー移動で選択を変えると即時に適用を要求し、条件変更で別の候補が選ばれたときも同じ動作です。開くだけでは音色を変えません。絞り込みで一覧の選択位置が失われても、現在鳴っている音色が結果に残っていればその音色へ選択を合わせ、再適用しません。開くと、保存した一覧選択をpatch名と音源・元patchの識別情報で復元し、見える位置までスクロールします（再適用はしません）。行番号は保存せず、カタログの並び順が変わっても同じpatchを選びます。一覧選択の記録がない場合は、Browse patchesで適用して鳴っている音色が現在の結果にあれば、その音色を選択します。鳴っている音色は終了時に保存し、次回起動時に復元した音色として引き継ぎます。effectの追加・削除・Browse effectsでは引き継ぎを保ちます。保存したpatchが削除済み・絞り込み対象外なら未選択にし、音色は変えません。一覧選択の記録も鳴っている音色の情報も無ければ未選択です。Favorites／History・instrumentの読込や削除で音色を変えた後は鳴っている音色が分からず、選択も解除されます。キー移動は開いたときの選択から始まり、未選択なら先頭へ移動します。準備中にも次の選択ができ、未開始の要求は最新1件に置き換わります。通常の適用成功時には下部の重複する音色名や固定余白を表示しません。選択と適用済みの音色が異なる場合だけ短い案内を表示し、未確認pinkなどでは前の音色が鳴り続け得ることを示します。pinkのSpace案内と適用失敗・保存エラーは表示します
  - sample容量が64,000,000 bytes以上のpink音色は自動適用しません（容量不明は通常扱い）。選択後に `Space` で音源・音色・容量の確認overlayを開き、その中で `Enter` を押すと準備・適用を開始します。確認中の `Esc` は変更せず取消します。一度適用した音色も再確認が必要です。読込中は進行overlayを表示し、途中cancelはできません。検索欄や保存メニュー・Solo／Mute overlayのSpace／Enterは音色適用に使いません（Solo／Mute overlayの `Enter` は `OK` と同じです）
  - ブラウザー内の `Random` または `R` キーは表示結果から均等に選びます。文字入力中は `T`／`R`／`M`／`/`／`h`／`j`／`k`／`l`／数字／`Enter` のショートカットは動作せず、ブラウザー内のoverlay・保存メニュー表示中も `R`／`M` は動作しません。pinkなら選択だけで、同じSpace→Enter確認が必要です。通常起動では`Esc`・`Enter` またはブラウザーウィンドウのOS closeでブラウザーだけを閉じ、本体と最後に適用成功した音色を保ち、未開始要求と古い準備結果を破棄します。既に適用を始めた処理は完了させます。確認overlay・Solo／Mute・保存メニューを開いているときの `Esc`／overlay外クリックは子だけを閉じ、ブラウザーや背後の操作へ同じ入力を通しません。OS closeはブラウザー全体を閉じます。再openでき、元の音色へ戻す取消操作はありません
  - 本体ウィンドウの `r` キーは、Browse patchesを開かずに、保存されているRole・Preset・検索文字列（Solo／Muteを含む）の絞り込み結果から、ブラウザーの `R` と同じく選んで適用します。押した直後に本体画面中央へ `Changing patch...` のoverlayを表示し、音色の適用が完了すると消します。カタログの読込前に押した場合はoverlayを表示したまま読込後に選びます。pink音色を引いた場合は選択だけで、Browse patchesを開いて `Space` を押す案内を状態欄に出します。0件・無効な検索のときも状態欄に表示します。適用しない場合や読込失敗時もoverlayを消します。SMF選択モードでは動作しません
  - 検索条件はハンバーガー `☰` 内の `Save condition to Role` でRoleへ追加し、Presetの `Delete` で削除できます。ALLからの追加先はEtcです。共通selectorの正規化・分類規則で扱い、catのconfig directory内 `patch-filter-presets.json` に保存します。読込破損時は元ファイルを保護して書込みを止め、保存失敗も画面に表示します。同じアプリ起動中はRole／Preset／検索・Solo／Muteを保持し、再起動後に復元するのは保存した自作条件だけです。TUIの条件やFavoritesは共有・移行しません
  - Browse patchesの準備中は現在の演奏を続け、競合するロード・走査・音色操作を無効にします。Floeなどのsample読込で待つ場合があります。MML・Velocity・CC1の `sweep`／`-` の選択、effectの順序・state・Bypassを維持し（CC1とMacroの `0`・`127` は `-` に戻します）、成功後は次回起動用stateを保存します。Favorites／Historyは自動追加しません。適用成功と保存失敗は区別して表示します。子ウィンドウのfocus中にも準備・適用の進行を更新します
  - 音色ブラウザーの候補は、既存cmrtカタログにあるSurge XT・Dexed・Floe・sforzando・Six Sines・TyrellN6・Vaporizer2のCLAP音色のうち、インストール済みのinstrumentと形式＋IDが一致するpatchです
  - drum/percussion音色のうち、共有表の実測で音高がnoteに追従する（`tracks_keyboard`がtrue）ものは、pluginを問わずleadの`pitched percussion`へ分類します（名前でmallet・bellと分かる音色はその欄へ）。kick・snare・hi-hatは対象外です。起動後に表を取得・保存し、取得失敗時は前回の有効な表、それも無ければ従来分類を使います。反映には再起動が必要です。音色を改名・移動して表に一致しなくなった場合も従来分類を使い、音色state・Favorites・Historyは変更しません（[分類の理由と制限：ADR 0040](docs/adr/0040-measured-pitched-percussion-categories.md)）
  - 音色ブラウザーのカタログは既存の `%LOCALAPPDATA%\clap-mml-render-tui\patch-catalog\catalog.json` を読み取り専用で利用します。VST3への代替と未知の音源は対象外です。effectのプリセット選択はエフェクトルーティング欄のBrowse effectsを使います。Dexedはcartridge内のprogram指定を保持します。Floe・sforzandoの外部sampleは元の場所に必要で、sampleを複製・保存する機能はありません
  - cmrtが無く上記の `catalog.json` が存在しない環境では、Surge XTの音色一覧を[cat-music-patterns](https://github.com/cat2151/cat-music-patterns)から起動ごとに取得し、`%LOCALAPPDATA%\cat-plugin-player\patch-catalog\surge-xt.json` に保存して、Browse patchesの候補に使います。取得できない場合は前回保存した一覧を使い、それも無ければ音色ブラウザーに読込失敗を表示します。取得が終わるまで（最大10秒）は読込待ちの表示になります。一覧はSurge XT 1.3.4から作ったもので、版の違うSurge XTでは増えた音色が出ず、無い音色を選ぶと準備失敗を表示します。`catalog.json` が存在するのに読めない場合は切り替えません。理由は[ADR 0025](docs/adr/0025-surge-xt-patch-catalog-without-cmrt.md)にあります
  - 準備・適用に失敗した場合は理由を表示します。元の構成・state・再生状態を復旧し、stateの復旧にも失敗したinstanceは保存を抑止します。音色適用後の保存失敗は適用済みとして表示します。音源別の準備・反映条件と検証範囲は[ADR 0019](docs/adr/0019-random-patch-all-catalog.md)に記録しています
  - 各プラグインの `★ Add favorite` で、その瞬間の音色・設定をお気に入りとして複数保存できます。メイン画面とBrowse patchesでは `F` キーで現在ロード中の音源を、Browse effectsでは対象の枠に接続中のeffectを保存します（未適用の候補は保存しません）。文字入力中・ダイアログ表示中は動作せず、ロード・準備中も保存できません。音源では演奏パターン・停止中の選択・確定したMML／コード本文も保存します。effectでは演奏設定を保存しません。新規項目の名前は、音源・effectともに記録済みのpatch／プリセット名からパスと対応するプリセットファイルの拡張子を除いたstemを優先し、`stem 連番` とします。effectのプラグイン名接頭辞も除きます。音色名中のドットやprogram番号は残します（例: `…/TX7-34.SYX/22 <Flute.22>` → `22 <Flute.22> 1`）。完全名は検索・ホバー表示用に保持します。名前が未取得・空欄の場合は `プラグイン名 連番` にします。後から変更できます。既存項目は改名せず、同じstateの再保存でも既存名を維持します
  - 別のプラグインを `Load` すると、置き換わるプラグインの音色・設定・演奏パターンを切り替え前に History へ自動記録します。新規項目の名前はお気に入りと同じ規則で、patch／プリセット名を優先します。同じプラグインの再読み込みや起動時の復元では追加しません。音源の確定MML／コード本文と停止中の選択も、お気に入りと同じ範囲で記録します。effectでは演奏設定を記録しません。保存に失敗した場合は切り替えを中止します
  - 手動・自動保存時に、同じプラグイン・役割・演奏パターン・確定MML本文・state の重複を確認し、保存済みの項目と名前を再利用します。Historyの自動記録では停止だけの違いを同一と扱いますが、本文が異なるMMLは別の項目として保持します。effectは演奏設定を比較しません
  - [プラグイン固有処理の方針・一覧（ADR 0002）](docs/adr/0002-plugin-specific-favorite-state-comparison.md)で、製品ごとの例外・理由・適用範囲・制限を確認できます。TyrellN6 CLAPの確認済みstate形式では、演奏だけで変わる圧縮部分を重複判定から除外し、テキストの音色設定が同じ音色をまとめます。確認済みのPCore `UI_op=9/10`とテキスト末尾の空行も比較から除外します（[ADR 0011](docs/adr/0011-tyrell-n6-ui-operation-favorite-comparison.md)）。Vaporizer2 CLAPの確認済み形式では、読み込み・保存で生じるMSEGの時間・座標の微小差（指定項目の絶対差：時間`0.00011` ms以下、正規化座標`0.000001`以下。詳細は[ADR 0010](docs/adr/0010-vaporizer2-mseg-recalculation-drift.md)）を同一と扱います。この幅以内の編集は区別できません。保存データは元のまま保持します。Surge XT・Vaporizer2・Six Sines・Floeでは、Macro欄で動かすMacroの値の違いを無視し、Macroの値だけが違う音色は同じ項目にまとめます。FloeではMacroを動かすと立つ「presetから変更済み」の印も無視します（[ADR 0039](docs/adr/0039-parameter-macro-sweep.md)。Surge XTは[ADR 0007](docs/adr/0007-surge-xt-favorite-state-comparison.md)）
  - 中央の `Favorites` / `History` / `Plugins` タブで一覧を切り替えます。`l` キーでFavorites→History→Pluginsの順に切り替えます。Favorites は手動順を維持し、新規追加は先頭に置きます。呼び出し・再保存・Historyへの記録では順番を変更しません。お気に入り名・プラグイン名・形式に加え、保存されたpatch／effectのプリセット名で絞り込めます（大小文字を区別しない部分一致）。Browse patches／Browse effectsで適用に成功した名前を別途記録し、Favorites／Historyの補足情報に表示します。再起動・お気に入りの呼び出し後も引き継ぎます。プラグインGUI内での名前変更・プリセット変更は追跡せず、最後にブラウザーで適用した名前を保持します。名前のない既存項目は従来の情報で検索でき、同じ音色をブラウザーから適用して再保存すると名前を補完します。メイン画面の `/` キーで絞り込み欄へフォーカスを移します（各タブ共通。文字入力中・MML入力画面・ブラウザー・ダイアログ表示中は動作しません）
  - 一覧側のハンバーガーメニュー（Settings）は `d` キーで開きます。メニュー内の項目キーは開いている間だけ有効で、キーで項目を実行すると閉じます。`Esc` またはメニュー外のクリックで取り消せます。文字入力中・MML入力画面・ブラウザー・ダイアログ表示中は開くキーが動作しません。
  - Favoritesのハンバーガーメニューで `Reorder favorites`（Favoritesタブ表示中の `d` → `o` キー）をONにすると、行末の `↑` / `↓` で並べ替え、即保存できます。行クリックの読み込み・Effect取り外しは従来どおりです。`Sort by plugin name`（`d` → `n` キー）は一度だけプラグイン名の昇順に並べ替えて保存します（大文字・小文字を区別せず、同名項目は前後関係を維持）。その後も手動調整できます。絞り込み中は並べ替え不可、並べ替えモードは起動時OFFです。[採用経緯と仕様（ADR 0017）](docs/adr/0017-manual-favorites-order.md)を参照してください
  - History は最新の登録順です。同じ音色と判定した場合は保存済みの音色を再利用し、登録時刻を更新して先頭へ移します。経過時間は `1s`, `1m`, `1h`, `1d`, `1w`, `1mon`, `1y` の単位で表示します（月は30日、年は365日）。手動のFavoritesは維持します。旧形式の全項目をHistoryへ移行し、`auto ` で始まる項目以外はFavoritesにも保持します（旧形式は自動保存の出自を記録していないため、名前変更済みの自動保存をすべてHistoryに含めるためです）。方針と理由は[ADR 0016](docs/adr/0016-history-instead-of-automatic-favorites.md)を参照してください。
  - HistoryにはRename・Deleteの操作メニューを表示しません。名前変更・削除はFavoritesで行います。
  - 全体設定の `☰` にある `Show only CLAP for duplicate plugins` は初期値ONで、`d` → `d` キーでも切り替えられます。同じメニューの `Rescan plugins...` は `d` → `y` キーでも確認画面を開けます。同じ名前・メーカー・種別のCLAPがある場合、Plugins一覧のVST3を非表示にします（名前・メーカーの前後の空白と大文字小文字の違いは無視し、名前またはメーカーが空の場合は両方表示します）。OFFにすると両形式を表示します。設定は `config.toml` の `[plugins] prefer_clap` に保存し、次回起動時も維持します。Favoritesとロード・復元する形式は変更しません。
  - 両タブとも名前・画像をクリックして読み込み、接続中のエフェクトはもう一度クリックして解除できます。現在ロード・接続中の行を色づけし、エフェクトのバイパス中も色を維持します。Plugins の `Load` / `Add` / `Remove` ボタンでも同じ操作ができます
  - Favorites は1件1行で表示します。長い名前・補足情報は省略し、マウスを重ねると全文を確認できます。波形・スペクトラムは右側の Routing の下に表示し、右側の幅は境界のドラッグで調整できます。`Audio analysis` の `On right` で、分析表示を右下／画面下部に切り替えられます（起動時は右下）
  - sforzando CLAPの確認済みstate形式では、保存カウンターとノート・CC1の演奏値の差だけで同じ音色を増やさず、既存のお気に入りを再利用します。音色設定の差は保持します。適用範囲と制限は[ADR 0006](docs/adr/0006-sforzando-favorite-state-comparison.md)を参照してください。
  - 各行の左に色付きの `Instrument` / `Effect` を表示します。お気に入り名を1クリックすると音色・設定を復元します。通常起動では音源のお気に入りから保存された確定本文と演奏パターン選択を復元します。SMF選択モードでは音色stateだけを使い、入力SMFを保持します。停止中は停止のまま、`Play`で復元したフレーズを演奏します。演奏中は保存されたパターン／MMLへ切り替えます。旧形式の本文を持たないお気に入りは現在の本文を保持し、旧形式の停止項目は現在の選択を維持します。変換やstate読込・接続に失敗した場合は元の本文・選択・演奏状態を復旧し、失敗した内容をセッションへ保存しません。音源のお気に入りは接続中の effect を、effect のお気に入りは音源とMMLを維持します。effect を呼び出すには音源が必要です
  - 接続中の effect のお気に入りには `Connected - click again to remove` を表示し、名前の再クリックでその effect を外します。呼び出し後に設定を編集した場合や Bypass 中も同じ動作です。音源・他のeffect・保存済みのお気に入りは残ります。同じ形式＋IDの別の音色は現在の位置で設定を切り替え、未接続の effect は末尾に追加します。各effectの選択中のお気に入りも次回起動時に復元します
  - 行末の `…` から名前変更・削除ができます。呼び出した後の編集や従来の自動保存は、お気に入りを上書きしません。`Last favorite` は最後に保存・呼び出したお気に入りを示します
  - お気に入りは `%LOCALAPPDATA%\cat-plugin-player\favorites\` に保存します。保存・復帰時には短い音切れが入る場合があり、保存できる範囲は従来のstate保存と同じです
- リサジュー表示
  - 右側の解析欄のオシロスコープ下に `Lissajous` を表示します。解析表示のハンバーガーメニューは `a` キーで開き、項目キーは開いている間だけ有効です。キーで項目を実行すると閉じ、`Esc` またはメニュー外クリックで取り消せます。開くキーの制限はSettingsと同じです。オシロスコープの `☰` → `On right`（`a` → `p` キー）を外すと、スペクトル・オシロスコープ・リサジューが画面下に横並びになります
  - 45°回転したリサジューに、effect・Bypassを反映した直近50msの出力を描きます（最大4096サンプル）。左右共通の自動倍率で音量差と図形の縦横比を保ち、同レベルの同相は縦線、逆相は横線、位相差のある正弦波は楕円や円になります。左だけの出力は左上〜右下、右だけの出力は右上〜左下の斜線、monoは縦線、無音は中央の点になります
  - ノート・周期数・トリガー設定に依存せず更新し、`Show analysis labels`（`a` → `a` キー）で見出しを表示できます。軸と描画方式はグラフのツールチップでも確認できます
  - リサジュー直下の `Correlation` は、直近150msの左右の相関を−1〜＋1のバーと数値で表示します。＋1は同相、0付近は相関が弱く、−1は逆相です。負の値は赤で表示し、モノラルに混ぜたときの打ち消しを判断する目安になります。無音または片側が無音の場合は「—」になります
- 波形表示
  - 画面下の `Oscilloscope` で、effect・Bypassを反映した出力のL（緑）とR（青）を重ねて表示します。monoは両方に同じ波形を表示します
  - 最後にNote OnしたMIDI note number（A4 = 440Hz）から表示幅を求め、`1`・`2`・`4`・`8 cycles` を選べます（`a` → `c` キーで次の周期数へ循環）。Note Off後も最後のnoteを基準に余韻を表示します。和音やデチューンを含む出力は、その幅で完全に繰り返すとは限りません
  - トリガーは `Zero cross` と `Similarity` から選び、`a` → `z` キーで切り替えます。`Show red guides` は `a` → `g` キーでも切り替えられます
  - `Zero cross` は左chの負→正の交差に合わせます。左chが無音なら右chを使い、交差がなければ先頭から表示します
  - `Similarity` は前回表示との相関係数が最大になる位置を、バッファ先頭から表示幅ぶん、1 sample刻みで総なめします。同点は先頭側を採用し、左右には同じ開始位置を使います。初回や比較対象が無音の場合はゼロクロスを使います
  - noteの発音、演奏パターン、表示周期数、トリガー方式、音声ストリームが変わると比較履歴をリセットします。十分なデータが集まるまでは待機表示になります
  - 探索は音声処理とは別のスレッドで行います。低いnote・多い周期数では表示の更新が遅くなることがあります。表示用データが途切れた場合は履歴をリセットします
  - 起動時の表示設定は4周期・ゼロクロスです。表示設定は保存しません

## 設定 : ウィンドウの位置・サイズ

`%LOCALAPPDATA%\cat-plugin-player\config.toml` に本体の起動位置・サイズとプラグインGUIの配置方式を指定できます。アプリを終了してから編集してください。セッションやお気に入りの選択状態は同じディレクトリの `status.json` に保存し、通常の保存では TOML を書き換えません。旧 TOML に混在している状態は初回起動時に移行し、元ファイルを `config.toml.before-status-migration.bak` に保存してから状態項目だけを除去します。設定のコメント・書式・未知の項目を保持し、削除項目に付いていた独立したコメントは末尾に残します。既存の `status.json` がある場合はそちらを優先します。

```toml
[window.main]
x = 100.0
y = 100.0
width = 1200.0
height = 800.0

[window.plugin]
placement = "right_then_bottom_right"
```

本体の `x`・`y` は論理座標で、両方を指定した場合に起動位置として使います。省略すると従来の初期位置になります。`width`・`height` は本体の内側の幅・高さ（論理座標）です。片方だけでも指定でき、省略した側や0以下・非有限の値は幅900・高さ600に戻ります。変更は次回起動時に反映されます。プラグインの配置方式は現在 `right_then_bottom_right` のみで、省略時も同じ動作です。GUIサイズと配置は実際のウィンドウ枠・画面の作業領域・DPIに基づいて計算します。本体やGUIを手で動かした位置の自動保存、プラグインごとの位置指定は行いません。

## ローカルでのビルドと実行

当アプリと `uapmd`、`clap-mml-render-tui`、`clap-mml-play-server` を同じ親ディレクトリに clone し、当アプリのディレクトリで実行します。TUIの `app` / `patches` / `patch-select` / `tui-core` とplay-serverの `core-lib` / `server-config` はCargo.tomlから常時ローカル参照します。play-serverには従来の `prepare_clap_patch_state` / `PatchStateError` に加えて `prepare_catalog_clap_patch_state` / `supports_catalog_clap_plugin`、TUIの `cmrt-patches` にはその再exportが必要です。音色ブラウザーは公開Role／Preset／filter APIとtui-coreのsample容量判定を使います。Browse effectsはplay-serverの `AudioEffectCatalog` / `EffectRenderer` とTUIのeffect selectorの選択条件、tui-coreの検索条件を利用します。checkout不足やAPI不足はビルドエラーとなり、Git版への自動切替は行いません。

```powershell
cargo build --release
& ./target/release/cat-plugin-player.exe
```

UAPMD が兄弟の `uapmd` ディレクトリ以外にある場合は、先に `$env:UAPMD_DIR = 'X:\projects\uapmd'` を設定してください。ビルド時の UAPMD の場所は環境変数で指定します。config.toml はビルド設定として読みません。

`cargo install` は使わず、既定の `target/release` に生成されたexeを直接実行します。ビルドはローカルのコード（未コミット変更を含む）と既存の `target/shim` を利用しますが、ソース変更によって再構成・再ビルドが必要になる場合があります。`target/shim` を VS 2022 など別の版で作ってある場合はその版のままビルドされ続けるため、`target/shim` を削除してからビルドしてください（[ADR 0020](docs/adr/0020-visual-studio-2026-toolchain.md)）。shim DLL は exe に同梱し、GUI 起動時にキャッシュディレクトリへ展開して読み込みます。

3repoの依存構成と運用方針は [ADR 0012](docs/adr/0012-local-dependencies-and-release-execution.md) を参照してください。catでのビルドにTUIのlocal切替は不要です。TUI自身で横断ビルドする場合はTUIの既存 `python scripts/cross_repo_local.py on` 手順を使い、offは人間が担当します。ローカル参照は未commit変更も使い、Cargo.lockは各checkoutの内容を固定しません。検証時は各repoのHEADと差分を記録してください。

Surge XTの初回導入は`shared/first_instrument_install.rs`に集約し、cat-smf-editorも同じ音源不在判定・規約画面・winget処理を参照します。各アプリは導入成功後の再スキャン・読み込みを担当します。

hostのRust/nativeとSMF処理は`shared/`に集約し、cat-smf-editorも同じ正本をmoduleとして参照します。独立crateは追加せず、各appの既定target/shimから自分のDLLを同梱します。

共通hostはCLAP plain値・VST3 normalized値の直接parameter automationに対応します。UIでparameterを列挙・値表記を取得し、音声処理には解決済みの対象を容量固定のbatchで渡します。editorの専用trackと、playerのVaporizer2・Six Sines・FloeのMacro欄がこの経路を利用します。生成補修の条件・再適用・回帰検証と未送信の作者向け報告文は[parameter automation補修](patches/uapmd/parameter-automation/README.md)、採用理由は[ADR 0037](docs/adr/0037-direct-parameter-automation.md)を参照してください。

引数なしで通常の GUI を起動します。`--help` と `--version` も使えます。

SMF選択モードは `--select-smf <SMF> --browser patch|effect --preserve-position` です。browser省略時はpatch、位置保持は明示指定が必要です。`--patch-filter <program名>`を渡すとpatch browserをALL条件からその名前で検索します。名前中の正規表現記号は文字として扱い、空白はANDです。カタログ読込後に0件の場合だけ単語を`|`でつないだOR検索へ一度切り替えます（例: `church organ` → `church|organ`）。`--patch-filter-file <JSON>`で検索・Role・Presetをそのまま復元し、正常終了時に同じfileへ返します。fileが無ければ初期検索を使い、既存fileの条件を優先します。空の検索も保持し、復元条件には単語ORへの自動切替を行いません。effect／master選択では使いません。検索欄は通常どおり大小無視の正規表現として編集できます。`--preview-smf <SMF>`で返却先とは別の全track試聴用SMFを指定できます。全trackのMIDIイベントを選択中の音源・effectへ送り、format 2ではtrackごとのtempoを反映します。試聴用SMFは書き換えません。`--start-seconds <秒>`で最初の試聴位置、`--loop-end-seconds <秒>`で入力SMF以上のloop終端を指定できます。省略時は先頭から入力SMFの終端までです。有限・非負の秒を使い、開始位置はloop終端より前にします。開始時点のcontroller・持続音を復元し、自然なloop境界は先頭へ戻ります。元SMFのイベントは変更しません。`--browser-only`を付けるとmain画面を表示せず、browserを前面に開きます。このoptionは選択モード専用です。通常のtrack選択ではbrowser内のボタンで別browserへ進めます。`--master-effects`ではeffect browserから始まり、patch browserには進みません。お気に入り・routingが必要なときだけ`Favorites / routing`からmain画面を表示できます。browserを閉じるEnterも保存・player全体の終了として扱います。保存失敗時や変更処理中はbrowserを保持し、エラーはbrowser内に表示します。Esc、検索等の入力に使われないq、本体／browserの×は入力SMFへinstrument・順序付きeffect・bypassのstateを保存して終了します。`--master-effects`ではconductorのmaster effect・bypassだけを保存し、track設定を保持します。検索欄のqは文字入力です。変更処理中は確定を待ち、保存失敗時は画面と選択を保持して再試行できます。通常のstatus.jsonは更新しません。選択中の既存state保存／お気に入り操作は通常と同じ保存先を使います。OS・plugin版をまたぐstate互換や外部sampleの複製は保証しません。[保存契約と共通host（ADR 0029）](docs/adr/0029-shared-host-smf-selection.md)を参照してください。

## 開発者用

### オフラインWAVの取得

`--render-wav` は呼出しごとに新しいhostとinstrumentを生成し、指定したバイナリstateを読み込んで、note48・velocity100・CC1=0の初音200msを取得して終了します。GUI・音声device・外部effect・通常sessionの復元は使いません。

```powershell
& ./target/release/cat-plugin-player.exe --render-wav X:/measure/out.wav --plugin-format CLAP --plugin-id org.surge-synth-team.surge-xt --plugin-path 'C:/Program Files/Common Files/CLAP/Surge Synth Team/Surge XT.clap' --state-file X:/measure/state.bin --wait-ms 10
```

形式は `CLAP` または `VST3`、idとpathはstateを保存したpluginに合わせて指定します。出力先の親フォルダーを先に作成し、WAVと同名の `.json` が存在しない新しい名前を使ってください。入力state、Favorites、History、通常のconfig/status/stateは書き換えません。

48kHz・256frames・stereoの32bit float WAVで、note onから9600framesを保存します。`--wait-ms` は非負整数の音声サンプル時間です。その間は空イベントを処理し、発音後も含め実時間へ同期せずにレンダリングします。通常起動の自動待ちは加えません。JSONにはstate/PCMのSHA-256、plugin識別情報、wait、発音位置・処理frames、無音判定、生成・state適用・音声処理・保存の所要時間を記録します。生成callbackを待つ時間は別項目です。全サンプル0の収録は `status=silent` としてWAV/JSONを残して非zero終了し、読込・処理エラーも非zero終了します。同じ入力でもpluginによってPCMが一致するとは限りません。

### Surge XT CLAPの初音待ち判定表

全catalog音色を、新規process・新規instanceで各条件1回ずつ高速オフライン計測します。通常のconfig/status/state、Favorites、Historyは書き換えません。WAV・state・ログはcache内の計測専用runディレクトリに残し、配布JSONには全音色の開始位置・エラーと0msの採用判断を保存します。

```powershell
& ./target/release/cat-plugin-player.exe measure-surge-start-delay --output ../cat-music-patterns/surge-xt-start-delay.json
```

既定は10ms対100ms、200msの収録、開始位置の差1ms以内を0ms許可とします。`--candidate-wait-ms 0`などで比較待ちを変更でき、`--reference-wait-ms`、`--max-onset-delta-ms`、`--onset-ratio`で判定条件を変更できます。0ms許可は採用方針であり、10ms対100msの結果を0msの実測と扱いません。`--jobs`は並行数（既定4）、`--timeout-seconds`は各renderの上限（既定30秒）、`--limit`は動作確認用の件数制限、`--work-dir`は計測runの保存先親ディレクトリです。無音・エラーは別区分にして先へ進みます。

閾値だけを変える場合は再レンダリング不要です。元の計測条件・値を保持したまま、例えば許容差2msで再選定できます。

```powershell
& ./target/release/cat-plugin-player.exe measure-surge-start-delay --reclassify ../cat-music-patterns/surge-xt-start-delay.json --max-onset-delta-ms 2 --output X:/measure/reclassified.json
```

正常完了時だけ出力表を置き換えます。cat-music-patternsへのcommit・pushは別途明示された操作として行います。起動時に通信完了を待たず、表の更新は演奏を再開しません。[計測の要点と採用判断（ADR 0008）](docs/adr/0008-surge-xt-automatic-note-start-delay.md)を参照してください。

### 起動時間の計測

`CAT_STARTUP_TRACE` に出力先を指定すると、起動処理の各時点をCSVに記録します。未指定なら記録せず、起動順序や自動演奏の条件は変えません。
Releaseビルド済みのリポジトリのルートで実行する例:

```powershell
$env:CAT_STARTUP_TRACE = Join-Path (Get-Location) 'target/startup.csv'
try {
    & ./target/release/cat-plugin-player.exe
} finally {
    Remove-Item Env:CAT_STARTUP_TRACE
}
Get-Content ./target/startup.csv
```

出力先の親ディレクトリは事前に作成してください。同名ファイルは上書きします。
各計測点はprocess中の初回だけ記録するため、比較するたびにアプリを起動し直してください。

- 基準はRustの `main()` 入口で、process生成やOSのDLLロードなど、それ以前の時間は含みません。
- `scan_callback` は走査完了通知がmain threadへ届いた時点です。`load_call_returned` と `instance_callback` で生成API呼び出しと完了通知を分け、`gui_requested` から `app_creation` がGUI初期化区間です。
- `first_audio_callback` は音声callback開始、`first_signal_rendered` は絶対値が0.000001（-120 dBFS）を超えるサンプルを初めて生成したブロックの処理後です。スピーカーからの出音時刻ではありません。
- 音声threadでは時刻の取得とatomicへの記録だけを行い、CSVの保存はUI側で行います。通常は最初の信号生成とcatalog準備完了後に保存します。未完了の場合は10秒経過後のUI更新、または正常終了時に途中結果を保存します。UI停止中や強制終了では保存できない場合があります。
- 未到達の計測点は `# missing:` に出ます。失敗の詳細は画面・標準エラーを確認してください。計測点の定義は [src/startup.rs](src/startup.rs)、起動高速化の判断と制限は [ADR 0034](docs/adr/0034-surge-xt-startup-minimal-data-home.md) を参照してください。

### 録画

紹介録画で初回導入を再現するときは `cargo build --release --features recording-mode` を使います。
起動ごとに `%LOCALAPPDATA%\cat-plugin-player-recordings\` 配下へ空の保存先を作り、
通常の設定・履歴を読み書きせず、Surge XT／Surge XT EffectsのCLAPだけを検出します。
cmrt catalogを参照せずSurge用catalogを新規取得します。
Surge XTの不在は偽装せず、撮影前に実際に人間またはcoding agentがアンインストールする運用とします。
撮影後は `cargo build --release` で通常のexeへ戻してください。
対象範囲・制限は [ADR 0026](docs/adr/0026-recording-mode.md) を参照してください。

### Surge XT Effectsのstate表

Surge XTを更新したときなどは、Surge XT Effectsをインストールした環境で次を実行して表を作り直します。

```
cargo run --release --example export_surge_fx_states -- ..\cat-music-patterns\surge-xt-fx-states.json
```

cat-music-patternsへのcommit・pushは人間が行います。形式と理由は[ADR 0028](docs/adr/0028-surge-xt-fx-prepared-states.md)を参照してください。

※↑AIはこのh1セクションのみに書いてください

</details>

# 今後のブレインストーミング
- ※気分で変更する可能性があります
- VST3とGUIに対応している、という、[cmrt](https://cat2151.github.io/clap-mml-render-tui/README.ja)にない強みの部分を伸ばす
  - GUIのedit結果statusをcmrtと連携
    - editした音色やエフェクトをcmrtで読み込んで利用
  - eguiでいろいろ遊ぶ
    - 鍵盤表示？
    - drum sequencer？
    - SMFビジュアライズ、複数SMFを合奏、音色とエフェクトを差し替え？
    - それらは別アプリに切り出したほうがよい？それとも音色とエフェクトの差し替えがあるからこのアプリでやるほうがよい？どのようなワークフローを想定している？
- cmrtの各種crateを利用して、いろいろやる
  - ※crateの洗練にもなり、こちらは利用することで恩恵があるので、win-win
  - CLI import / export
    - cmrt keyboard画面はCLIから、patchとeffectとアルペジオなどを指定して、一発で鳴らせます。同じことを可能に（cmrt をcat-plugin-player に変更するだけで鳴るレベルに）
      - 頻繁に破壊的変更してよい。あとから手作業で移行作業は可能。変更をおそれて身動きできなくて進歩がとまるよりマシ
  - SMF import / export
    - 現在、SMFにオーディオプラグイン名とpatch名を記録して出力は実装済み、それを発展させる

# コンセプト、目指すもの
- 楽器
  - 例えばMIDI鍵盤のついたシンセサイザーは、電源を入れて鍵盤を押せば音が鳴ります。
    - そのレベルで手軽に使えることを目指します。
      - 音を鳴らすまでの操作を最小化します。
- 鳴ること
  - 「作者の環境で鳴ること」はできるだけ維持することを目指します
- すぐ鳴ること
  - アプリ起動から音が鳴るまで体感0.2秒。演奏までを最速にしています
    - GUI表示準備などの処理も、音の後にしています
      - GUI表示より先に音が鳴るのは、いわばスプラッシュスクリーンのサウンド版かもしれません
- 教育用
  - 商業用よりは、教育用の方向を向いています
- 実験、探索、自分用
  - 頻繁に破壊的変更をします

# 思ったことを雑に書くコーナー
- ※コンセプトに関連します
- オーディオプラグインのプリセット音色が好きだ
  - 再現可能であるから
  - freeにシェアされるおかげで、みんなが恩恵を受けられるから
- 楽器UXを提供したい
  - DAWのUXではなく、楽器のUXを提供したい
- 楽器とは
  - オーディオプラグイン から PCキーボード までの各layerの総合である
    - 中間layerがある
      - それはプラグインホスト、演奏用UIである
        - ここはバイブコーディングできるlayerである
        - ここを作ることでUXの改善ができる
- 誰のための楽器？
  - 猫、赤子
    - つまり猫や赤子（例えばAndroidタブレット）もターゲットに含むくらいレンジを初心者側に広くとる
  - ※あくまで比喩表現です。実際にAndroidタブレットで動くかは未確認です。Androidで動かない場合に動くよう努力をする可能性も低いです

# スコープ外、目指さないもの
- 堅牢さ
  - 何をしても絶対にバグらない。アプリが落ちない
  - 完全互換。あらゆる過去の設定やデータをloadして動作する
- 機能
  - あらゆる機能を搭載。あらゆる用途で利用可
- 利便性
  - 便利さ、使いやすさを極限まで追求
- 性能
  - あらゆる環境でCPU負荷なく高速に動作。音は絶対に途切れない
- 要望
  - ユーザーのあらゆる要望に即応

# ライセンス
- MIT
