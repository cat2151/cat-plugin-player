//! Offer Surge XT through winget when a scan finds no instrument (ADR 0024).
#[path = "japanese_font.rs"]
pub(crate) mod japanese_font;
#[path = "first_instrument_install/text.rs"]
mod text;

use super::ffi::{PluginInfo, PluginKind};
const SURGE_ID: &str = "org.surge-synth-team.surge-xt";
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
pub struct FirstInstrument {
    dialog: Dialog,
    /// Kept after the dialog is closed, so a finished install still rescans.
    job: Option<Receiver<Report>>,
    /// Set when winget succeeded; the next scan loads Surge XT.
    installed: bool,
    text: Option<&'static Text>,
}

impl FirstInstrument {
    pub fn running(&self) -> bool {
        self.job.is_some()
    }

    pub fn dialog_open(&self) -> bool {
        self.dialog != Dialog::Hidden
    }

    /// Returns the Surge XT to load when this scan follows a successful install.
    pub fn on_scan(&mut self, plugins: &[PluginInfo]) -> Option<usize> {
        if self.running() {
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
    let line = format!("winget install --id {PACKAGE_ID} -e --source winget --silent --disable-interactivity --accept-package-agreements --accept-source-agreements");
    // Inno Setup's RestartManager can otherwise close OBS while recording (ADR 0026).
    #[cfg(feature = "recording-mode")]
    let line = format!("{line} --custom /NOCLOSEAPPLICATIONS");
    line
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
    unsafe extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    const LANG_JAPANESE: u16 = 0x11;
    // SAFETY: takes no arguments and only reads the user's setting.
    unsafe { GetUserDefaultUILanguage() & 0x3ff == LANG_JAPANESE }
}

impl FirstInstrument {
    pub fn show(&mut self, ctx: &egui::Context, loaded: bool) -> Option<Result<(), String>> {
        let result = self.poll();
        let state = self;
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
            state.finish_loading();
        }
        if state.dialog == Dialog::Hidden {
            return result;
        }
        let text = *state.text.get_or_insert_with(|| {
            if windows_ui_is_japanese() {
                japanese_font::add(ctx);
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
        result
    }

    pub fn finish_loading(&mut self) {
        self.installed = false;
        self.dialog = Dialog::Hidden;
    }

    fn poll(&mut self) -> Option<Result<(), String>> {
        let Some(job) = &self.job else {
            return None;
        };
        let mut done = None;
        while let Ok(report) = job.try_recv() {
            match report {
                Report::Step(step) => {
                    if let Dialog::Installing { step: shown, .. } = &mut self.dialog {
                        *shown = step;
                    }
                }
                Report::Done(result) => done = Some(result),
            }
        }
        let result = done?;
        self.job = None;
        match result {
            Ok(()) => {
                self.installed = true;
                if let Dialog::Installing { step, .. } = &mut self.dialog {
                    *step = Step::Loading;
                }
                Some(Ok(()))
            }
            Err(failure) => {
                let detail = failure.log_detail().to_owned();
                self.dialog = Dialog::Failed(failure);
                Some(Err(detail))
            }
        }
    }
}

#[cfg(test)]
#[path = "first_instrument_install/tests.rs"]
mod tests;
