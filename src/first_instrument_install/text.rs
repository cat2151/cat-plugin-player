//! One language at a time; mixing both doubled the reading load for beginners.

pub(super) struct Text {
    pub offer_heading: &'static str,
    pub offer_body: &'static str,
    pub view_terms: &'static str,
    pub install: &'static str,
    pub later: &'static str,
    pub installing_heading: &'static str,
    pub preparing: &'static str,
    pub downloading: &'static str,
    pub running_installer: &'static str,
    pub loading: &'static str,
    pub seconds: &'static str,
    pub failed_heading: &'static str,
    pub winget_missing: &'static str,
    pub manual_download: &'static str,
    pub close: &'static str,
}

pub(super) const JAPANESE: Text = Text {
    offer_heading: "オーディオプラグインがありません",
    offer_body:
        "無料のオーディオプラグイン「Surge XT」をインストールすると、すぐに音を鳴らせます。",
    view_terms: "利用規約を見る",
    install: "利用規約に同意してインストールする",
    later: "あとで",
    installing_heading: "Surge XT をインストールしています",
    preparing: "1/4 準備しています…",
    downloading: "2/4 ダウンロードしています…",
    running_installer:
        "3/4 インストールしています。\nWindows の確認画面が出たら「はい」を押してください。",
    loading: "4/4 Surge XT を読み込んでいます…",
    seconds: "秒",
    failed_heading: "インストールできませんでした",
    winget_missing: "winget が見つかりません。",
    manual_download: "手動でダウンロードする",
    close: "閉じる",
};

pub(super) const ENGLISH: Text = Text {
    offer_heading: "No instrument found",
    offer_body: "Install the free instrument \"Surge XT\" to start playing right away.",
    view_terms: "View the terms",
    install: "Agree to the terms and install",
    later: "Later",
    installing_heading: "Installing Surge XT",
    preparing: "1/4 Preparing…",
    downloading: "2/4 Downloading…",
    running_installer: "3/4 Installing.\nWhen Windows asks for permission, choose Yes.",
    loading: "4/4 Loading Surge XT…",
    seconds: "s",
    failed_heading: "Could not install",
    winget_missing: "winget was not found.",
    manual_download: "Download it manually",
    close: "Close",
};
