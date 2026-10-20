# ADR 0035: 共有hostでstb revisionを固定する

状態: 採用。

rtlogの未固定stb取得がヘッダ移動で壊れたため、shared/host.cmakeで
UAPMDより先に既知SHAをFetchContentへ登録する。target生成はrtlogに任せる。
editor/playerのCRT方針は維持し、依存checkoutを書き換えない。
既存cacheは取得前に検査し、不一致やoverrideは停止する。
共有driverの監視とstampで既存configureへ反映する。

最新stbへのinclude追加、editorだけの補修、UAPMD全体の旧版固定は採用しない。
検証・再clone・復旧・撤去条件・未送信の上流報告は
[補修説明書](../../patches/uapmd/stb-revision-pin/README.md)にまとめる。
新規runnerでの検証は既存cacheのローカル成功とは別に必要。
