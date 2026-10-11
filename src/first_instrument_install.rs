//! Offer Surge XT through winget when a scan finds no instrument (ADR 0024).
mod text;

use crate::{ffi::PluginInfo, plugin_list::PluginKind, random_patch_catalog::SURGE_ID, App};
use eframe::egui;
use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};
use text::Text;

const PACKAGE_ID: &str = "SurgeSynth.SurgeXT";
const MANUAL_URL: &str = "https://surge-synthesizer.github.io/";
/// The package manifest's LicenseUrl.
const LICENSE_URL: &str = "https://github.com/surge-synthesizer/surge/blob/main/LICENSE";
/// Tried in order; egui's built-in fonts have no Japanese glyphs.
const JAPANESE_FONTS: [&str; 2] = [
    r"C:\Windows\Fonts\meiryo.ttc",
    r"C:\Windows\Fonts\msgothic.ttc",
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    Preparing,
    Downloading,
    RunningInstaller,
    Loading,
}

#[derive(Debug, Default, PartialEq)]
enum Dialog {
    #[default]
    Hidden,
    Offer,
    Installing {
        step: Step,
        since: Instant,
    },
    Failed(Failure),
}

#[derive(Debug, PartialEq)]
enum Failure {
    WingetMissing,
    /// winget's last line, already in the user's language.
    Exit(String),
}

impl Failure {
    fn log_detail(&self) -> &str {
        match self {
            Self::WingetMissing => "winget was not found",
            Self::Exit(detail) => detail,
        }
    }
}

enum Report {
    Step(Step),
    Done(Result<(), Failure>),
}

#[derive(Default)]
pub(crate) struct FirstInstrument {
    dialog: Dialog,
    /// Kept after the dialog is closed, so a finished install still rescans.
    job: Option<Receiver<Report>>,
    /// Set when winget succeeded; the next scan loads Surge XT.
    installed: bool,
    text: Option<&'static Text>,
}

impl FirstInstrument {
    /// Returns the Surge XT to load when this scan follows a successful install.
    pub(crate) fn on_scan(&mut self, plugins: &[PluginInfo]) -> Option<usize> {
        if self.job.is_some() {
            return None;
        }
        if std::mem::take(&mut self.installed) {
            let surge = plugins
                .iter()
                .position(|p| p.format == "CLAP" && p.id == SURGE_ID);
            if surge.is_some() {
                return surge;
            }
        }
        self.dialog = if plugins.iter().any(|p| p.kind == PluginKind::Instrument) {
            Dialog::Hidden
        } else {
            Dialog::Offer
        };
        None
    }
}

fn command_line() -> String {
    format!("winget install --id {PACKAGE_ID} -e --source winget --silent --disable-interactivity --accept-package-agreements --accept-source-agreements")
}

/// Piped winget prints no byte progress, and its wording follows the Windows language,
/// so steps come from line order: the installer URL starts the download, the next line ends it.
fn next_step(step: Step, line: &str) -> Step {
    match step {
        Step::Preparing if line.contains("https://") => Step::Downloading,
        Step::Downloading if !line.contains("https://") => Step::RunningInstaller,
        step => step,
    }
}

/// The installer raises its own UAC prompt; winget itself runs unelevated.
fn run_winget(reports: &Sender<Report>) -> Result<(), Failure> {
    let mut child = Command::new("winget")
        .args(command_line().split(' ').skip(1))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => Failure::WingetMissing,
            _ => Failure::Exit(format!("could not start winget: {error}")),
        })?;
    let mut step = Step::Preparing;
    let mut last_line = String::new();
    if let Some(stdout) = child.stdout.take() {
        for line in std::io::BufReader::new(stdout)
            .split(b'\n')
            .map_while(Result::ok)
        {
            let line = String::from_utf8_lossy(&line).trim().to_owned();
            if line.is_empty() {
                continue;
            }
            let next = next_step(step, &line);
            if next != step {
                step = next;
                let _ = reports.send(Report::Step(step));
            }
            last_line = line;
        }
    }
    let status = child
        .wait()
        .map_err(|error| Failure::Exit(format!("winget: {error}")))?;
    match status.code() {
        Some(0) => Ok(()),
        // winget reports HRESULT-style codes such as 0x8A15002B.
        Some(code) => Err(Failure::Exit(
            format!("{last_line} (0x{:08X})", code as u32)
                .trim()
                .to_owned(),
        )),
        None => Err(Failure::Exit("winget was terminated".into())),
    }
}

fn windows_ui_is_japanese() -> bool {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    const LANG_JAPANESE: u16 = 0x11;
    // SAFETY: takes no arguments and only reads the user's setting.
    unsafe { GetUserDefaultUILanguage() & 0x3ff == LANG_JAPANESE }
}

fn add_japanese_font(ctx: &egui::Context) {
    let Some(bytes) = JAPANESE_FONTS
        .iter()
        .find_map(|path| std::fs::read(path).ok())
    else {
        return;
    };
    ctx.add_font(egui::epaint::text::FontInsert::new(
        "first_instrument_japanese",
        egui::FontData::from_owned(bytes),
        vec![egui::epaint::text::InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: egui::epaint::text::FontPriority::Lowest,
        }],
    ));
}

impl App {
    pub(crate) fn first_instrument_dialog(&mut self, ctx: &egui::Context) {
        self.poll_first_instrument();
        let loaded = !self.scanning && self.pending.is_none();
        let state = &mut self.first_instrument;
        if loaded
            && !state.installed
            && matches!(
                state.dialog,
                Dialog::Installing {
                    step: Step::Loading,
                    ..
                }
            )
        {
            state.dialog = Dialog::Hidden;
        }
        if state.dialog == Dialog::Hidden {
            return;
        }
        let text = *state.text.get_or_insert_with(|| {
            if windows_ui_is_japanese() {
                add_japanese_font(ctx);
                &text::JAPANESE
            } else {
                &text::ENGLISH
            }
        });
        let mut install = false;
        let mut close = false;
        let response = egui::Modal::new(egui::Id::new("first_instrument")).show(ctx, |ui| {
            ui.set_max_width(540.0);
            match &state.dialog {
                Dialog::Hidden => {}
                Dialog::Offer => {
                    ui.heading(text.offer_heading);
                    ui.label(text.offer_body);
                    ui.hyperlink_to(text.view_terms, LICENSE_URL);
                    ui.horizontal(|ui| {
                        install = ui
                            .button(text.install)
                            .on_hover_text(command_line())
                            .clicked();
                        close = ui.button(text.later).clicked();
                    });
                }
                Dialog::Installing { step, since } => {
                    ui.heading(text.installing_heading);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(match step {
                            Step::Preparing => text.preparing,
                            Step::Downloading => text.downloading,
                            Step::RunningInstaller => text.running_installer,
                            Step::Loading => text.loading,
                        });
                    });
                    ui.weak(format!("{}{}", since.elapsed().as_secs(), text.seconds));
                    ctx.request_repaint_after(Duration::from_secs(1));
                }
                Dialog::Failed(failure) => {
                    ui.heading(text.failed_heading);
                    ui.label(match failure {
                        Failure::WingetMissing => text.winget_missing,
                        Failure::Exit(detail) => detail,
                    });
                    ui.hyperlink_to(text.manual_download, MANUAL_URL);
                    close = ui.button(text.close).clicked();
                }
            }
        });
        if install {
            let (sender, receiver) = mpsc::channel();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let result = run_winget(&sender);
                let _ = sender.send(Report::Done(result));
                ctx.request_repaint();
            });
            state.job = Some(receiver);
            state.dialog = Dialog::Installing {
                step: Step::Preparing,
                since: Instant::now(),
            };
        } else if close || response.should_close() {
            state.dialog = Dialog::Hidden;
        }
    }

    fn poll_first_instrument(&mut self) {
        let Some(job) = &self.first_instrument.job else {
            return;
        };
        let mut done = None;
        while let Ok(report) = job.try_recv() {
            match report {
                Report::Step(step) => {
                    if let Dialog::Installing { step: shown, .. } =
                        &mut self.first_instrument.dialog
                    {
                        *shown = step;
                    }
                }
                Report::Done(result) => done = Some(result),
            }
        }
        let Some(result) = done else {
            return;
        };
        self.first_instrument.job = None;
        match result {
            Ok(()) => {
                self.first_instrument.installed = true;
                if let Dialog::Installing { step, .. } = &mut self.first_instrument.dialog {
                    *step = Step::Loading;
                }
                self.start_scan(true);
            }
            Err(failure) => {
                self.status = format!("Surge XT install failed: {}", failure.log_detail());
                self.first_instrument.dialog = Dialog::Failed(failure);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin(kind: PluginKind) -> PluginInfo {
        PluginInfo {
            index: 0,
            name: "Plugin".into(),
            vendor: "Vendor".into(),
            format: "CLAP".into(),
            id: "id".into(),
            bundle_path: String::new(),
            kind,
        }
    }

    #[test]
    fn offers_only_when_no_instrument_was_found() {
        let mut state = FirstInstrument::default();
        state.on_scan(&[]);
        assert_eq!(state.dialog, Dialog::Offer);
        state.on_scan(&[plugin(PluginKind::Effect), plugin(PluginKind::Unknown)]);
        assert_eq!(state.dialog, Dialog::Offer);
        state.on_scan(&[plugin(PluginKind::Effect), plugin(PluginKind::Instrument)]);
        assert_eq!(state.dialog, Dialog::Hidden);
    }

    #[test]
    fn a_running_install_keeps_its_dialog_through_rescans() {
        let (_sender, receiver) = mpsc::channel();
        let installing = || Dialog::Installing {
            step: Step::Downloading,
            since: Instant::now(),
        };
        let mut state = FirstInstrument {
            dialog: installing(),
            job: Some(receiver),
            installed: false,
            text: None,
        };
        state.on_scan(&[plugin(PluginKind::Instrument)]);
        assert!(matches!(state.dialog, Dialog::Installing { .. }));
    }

    #[test]
    fn the_scan_after_an_install_loads_surge_clap_once() {
        let mut surge = plugin(PluginKind::Instrument);
        surge.id = SURGE_ID.into();
        let mut surge_vst3 = surge.clone();
        surge_vst3.format = "VST3".into();
        let plugins = [plugin(PluginKind::Effect), surge_vst3, surge];
        let loading = || Dialog::Installing {
            step: Step::Loading,
            since: Instant::now(),
        };
        let mut state = FirstInstrument {
            dialog: loading(),
            installed: true,
            ..Default::default()
        };
        assert_eq!(state.on_scan(&plugins), Some(2));
        assert!(matches!(state.dialog, Dialog::Installing { .. }));
        assert_eq!(state.on_scan(&plugins), None);
        assert_eq!(state.dialog, Dialog::Hidden);

        let mut missing = FirstInstrument {
            dialog: loading(),
            installed: true,
            ..Default::default()
        };
        assert_eq!(missing.on_scan(&[plugin(PluginKind::Effect)]), None);
        assert_eq!(missing.dialog, Dialog::Offer);
    }

    #[test]
    fn steps_follow_the_recorded_winget_output() {
        // Piped `winget download` output on a Japanese Windows, in order.
        let lines = [
            ("見つかりました Surge XT [SurgeSynth.SurgeXT] バージョン 1.3.4", Step::Preparing),
            ("このアプリケーションは所有者からライセンス供与されます。", Step::Preparing),
            (
                "ダウンロード中 https://github.com/surge-synthesizer/releases-xt/releases/download/1.3.4/surge-xt-win64-1.3.4-setup.exe",
                Step::Downloading,
            ),
            ("インストーラーハッシュが正常に検証されました", Step::RunningInstaller),
            ("インストーラがダウンロードされました: D:\\dl\\setup.exe", Step::RunningInstaller),
        ];
        let mut step = Step::Preparing;
        for (line, expected) in lines {
            step = next_step(step, line);
            assert_eq!(step, expected, "{line}");
        }
    }

    #[test]
    fn command_installs_the_exact_package_without_prompts() {
        let line = command_line();
        assert!(line.starts_with("winget install --id SurgeSynth.SurgeXT -e "));
        assert!(line.contains("--silent --disable-interactivity"));
        assert!(!line.contains("  "), "args are split on single spaces");
    }
}
