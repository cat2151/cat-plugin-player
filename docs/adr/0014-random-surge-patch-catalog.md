# 0014: Random patch の Surge XT CLAP catalog 境界

状態: 採用。

初回実験は Surge XT CLAP の patch に限る。既存 cmrt catalog reader を GUI の
Voice callback 完了確認後に一度だけ worker から呼ぶ。起動前半と audio callback
には catalog I/O を追加しない。worker は owned 候補を送信して repaint を要求し、
GUI は try_recv で受領する。終了時に worker を join しない。

候補は upstream の plugin 付き PatchRef から同じ plugin の PatchBase を使って
解決する。CLAP bundle かつ Surge XT の厳密な plugin ID を対象とし、GUI の
installed list と format + ID で照合する。音源名だけの照合、VST3 への置換、
catalog JSON の独自実装は行わない。scan と read の各完了時に再照合する。

read 中、失敗、空、対応 installed plugin がない場合はボタンを表示しない。
失敗理由を既存 status/log へ残し、自動再試行を行わない。scan、restore、生成、
patch 準備中の操作を無効化する。抽選・変換・transaction は次の実装段階で
この候補を利用し、scan index を永続識別子として扱わない。
