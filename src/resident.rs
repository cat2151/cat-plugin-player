//! Resident mode: the editor keeps one hidden player and asks it to show a browser.
//!
//! Closing a browser only hides it. The player ends when the editor's pipes close.
//! A chosen patch or effect is not loaded here: its state goes to the editor,
//! which plays it with its own plugin instances.
use crate::{
    cli::Browser,
    resident_ipc::{self, Open, SharedState, ToPlayer},
    resident_log, App,
};
use std::sync::mpsc;

pub(crate) struct Resident {
    incoming: mpsc::Receiver<Incoming>,
    pub(super) sender: resident_ipc::Sender,
    /// The browser on screen, or `None` while hidden.
    pub(crate) shown: Option<Open>,
    pub(super) preparation_target: Option<Open>,
    focus: Option<Browser>,
    /// Sent to the editor and not answered yet.
    pub(super) offered: Option<Offer>,
    /// The last state line written to the trace.
    logged: String,
    pub(super) panes: Option<(String, [String; 2], String)>,
    pub(super) cursor: Option<crate::status::PatchIdentity>,
    pub(super) cursor_cache:
        std::collections::BTreeMap<String, Option<crate::status::PatchIdentity>>,
    pub(super) playing_cache: std::collections::BTreeMap<String, crate::status::PatchIdentity>,
    pub(super) pane_cache: std::collections::BTreeMap<String, ([String; 2], String)>,
}

pub(super) fn log(message: impl std::fmt::Display) {
    resident_log::log("player", message);
}

pub(super) struct Offer {
    pub(super) browser: Browser,
    pub(super) label: String,
    pub(super) _state: SharedState,
}

enum Incoming {
    Message(Box<ToPlayer>),
    Closed,
}

impl Resident {
    pub(crate) fn connect(name: &str) -> std::io::Result<Self> {
        resident_log::start("player");
        let (sender, receiver) = resident_ipc::connect(name)?;
        log(format_args!("connected {name}"));
        Ok(Self::start(sender, receiver))
    }

    /// Reads the editor's lines on a thread so the GUI never waits on the pipe.
    fn start(sender: resident_ipc::Sender, mut receiver: resident_ipc::Receiver) -> Self {
        let (deliver, incoming) = mpsc::channel();
        std::thread::spawn(move || loop {
            let next = match receiver.receive::<ToPlayer>() {
                Ok(Some(message)) => Incoming::Message(Box::new(message)),
                Ok(None) => Incoming::Closed,
                Err(error) => {
                    eprintln!("Resident link failed: {error}");
                    Incoming::Closed
                }
            };
            let closed = matches!(next, Incoming::Closed);
            if deliver.send(next).is_err() {
                return;
            }
            if let Some(context) = crate::GUI_CONTEXT.lock().unwrap().clone() {
                context.request_repaint();
            }
            if closed {
                return;
            }
        });
        Self {
            incoming,
            sender,
            shown: None,
            preparation_target: None,
            focus: None,
            offered: None,
            logged: String::new(),
            panes: None,
            pane_cache: Default::default(),
            playing_cache: Default::default(),
            cursor: None,
            cursor_cache: Default::default(),
        }
    }
}

pub(super) fn browser(open: Open) -> Browser {
    match open {
        Open::Patch { .. } | Open::Layer { .. } | Open::Kit { .. } => Browser::Patch,
        Open::Effect { .. } | Open::MasterEffect { .. } => Browser::Effect,
    }
}

impl App {
    pub(crate) fn remember_preparation_target(&mut self) {
        if let Some(resident) = &mut self.resident {
            resident.preparation_target = resident.shown;
        }
    }
    /// Applies what the editor sent; `true` once the editor has gone.
    pub(crate) fn poll_resident(&mut self) -> bool {
        self.report_resident_panes();
        let Some(resident) = &self.resident else {
            return false;
        };
        let mut messages = Vec::new();
        let closed = loop {
            match resident.incoming.try_recv() {
                Ok(Incoming::Message(message)) => messages.push(message),
                Ok(Incoming::Closed) | Err(mpsc::TryRecvError::Disconnected) => break true,
                Err(mpsc::TryRecvError::Empty) => break false,
            }
        };
        if closed {
            log("editor closed the link");
        }
        for message in messages {
            log(format_args!("received {message:?}"));
            match *message {
                ToPlayer::Open(open) => {
                    self.report_resident_panes();
                    self.resident.as_mut().unwrap().panes = None;
                    self.show_resident(open);
                }
                ToPlayer::OpenFiltered {
                    open,
                    context,
                    panes,
                    query,
                    cursor,
                } => {
                    self.report_resident_panes();
                    // A quick close/reopen can arrive before the editor has read
                    // our last pane notification. Keep the newest local labels.
                    let (panes, query) = self
                        .resident
                        .as_ref()
                        .unwrap()
                        .pane_cache
                        .get(&context)
                        .cloned()
                        .unwrap_or((panes, query));
                    let playing = self
                        .resident
                        .as_ref()
                        .unwrap()
                        .playing_cache
                        .get(&context)
                        .cloned();
                    let cursor = self
                        .resident
                        .as_ref()
                        .unwrap()
                        .cursor_cache
                        .get(&context)
                        .cloned()
                        .unwrap_or(cursor);
                    if matches!(open, Open::Kit { .. }) {
                        self.show_resident(open);
                        continue;
                    }
                    self.restore_resident_panes(open, panes.clone(), query.clone());
                    if matches!(open, Open::Patch { .. } | Open::Layer { .. }) {
                        self.patch_browser.restore_selected = cursor.clone();
                        self.patch_browser.restore_playing = playing;
                        self.patch_browser.resolve_restored_playing();
                    }
                    self.resident.as_mut().unwrap().cursor = cursor;
                    self.show_resident(open);
                    self.resident.as_mut().unwrap().panes = Some((context, panes, query));
                }
                ToPlayer::Applied(result) => self.resident_answered(result),
            }
        }
        closed
    }

    /// One selection at a time travels to the editor.
    pub(crate) fn resident_waiting(&self) -> bool {
        self.resident.as_ref().is_some_and(|r| r.offered.is_some())
    }

    fn resident_answered(&mut self, result: Result<(), String>) {
        let Some(offer) = self.resident.as_mut().and_then(|r| r.offered.take()) else {
            log("answer arrived with nothing offered; ignored");
            return;
        };
        log(format_args!("answer for {}: {result:?}", offer.label));
        if self.resident_kit() && result.is_ok() {
            self.close_drum_browser();
        }
        let kind = match offer.browser {
            Browser::Patch => {
                self.patch_browser.requests.finish(result.is_ok());
                "patch"
            }
            Browser::Effect => "effect",
        };
        self.status = match result {
            Ok(()) => format!("Browse {kind}: {}", offer.label),
            Err(error) => format!("Browse {kind} failed: {error}"),
        };
    }

    fn show_resident(&mut self, open: Open) {
        if self.drum_browser.open {
            self.close_drum_browser();
        }
        if matches!(open, Open::Kit { .. }) {
            self.patch_browser.close();
            self.effect_browser.close();
            self.resident.as_mut().unwrap().shown = Some(open);
            self.open_resident_kit();
            return;
        }
        let browser = browser(open);
        match browser {
            Browser::Patch => {
                if self.effect_browser.open {
                    self.effect_browser.close();
                }
                self.patch_browser.open_window();
            }
            Browser::Effect => {
                if self.patch_browser.open {
                    self.patch_browser.close();
                }
                let ids = self.effect_ids();
                self.effect_browser.open_window(&ids);
            }
        }
        let resident = self.resident.as_mut().unwrap();
        resident.shown = Some(open);
        resident.focus = Some(browser);
    }

    /// Esc, Q, or the window's close button: hide instead of exiting.
    pub(crate) fn hide_resident(&mut self) {
        self.report_resident_panes();
        log("hide browser");
        if self.patch_browser.open {
            self.patch_browser.close();
        }
        if self.effect_browser.open {
            self.effect_browser.close();
        }
        if let Some(resident) = &mut self.resident {
            resident.shown = None;
            resident.panes = None;
            resident.focus = None;
        }
    }

    /// Enter in the effect browser closes it: the resident browser hides too.
    pub(crate) fn resident_browser_closed(&mut self, closed: Browser) {
        if self
            .resident
            .as_ref()
            .is_some_and(|r| r.shown.is_some_and(|open| browser(open) == closed))
        {
            self.hide_resident();
        }
    }

    pub(crate) fn take_resident_focus(&mut self, browser: Browser) -> bool {
        let Some(resident) = &mut self.resident else {
            return false;
        };
        if resident.focus == Some(browser) {
            resident.focus = None;
            true
        } else {
            false
        }
    }

    /// Writes the selection pipeline whenever it changes, so a stuck selection shows why.
    pub(crate) fn log_resident_state(&mut self) {
        let Some(resident) = &self.resident else {
            return;
        };
        let requests = &self.patch_browser.requests;
        let ticket = |t: &Option<crate::patch_browser_requests::Ticket>| {
            t.as_ref().map_or("-".to_string(), |t| {
                format!("#{}g{} {}", t.serial, t.generation, t.candidate.display)
            })
        };
        let line = format!(
            "state shown={:?} patch_open={} effect_open={} selected={} desired={} confirm={} preparing={} applying={} applied={} offered={} busy[scan={} deferred={} restoring={} pending={} effect_prep={} patch_prep={}] catalog={:?} query={:?} role={} preset={} results={} status={:?}",
            resident.shown,
            self.patch_browser.open,
            self.effect_browser.open,
            ticket(&requests.selected),
            ticket(&requests.desired),
            ticket(&requests.confirmation),
            ticket(&requests.preparing),
            ticket(&requests.applying),
            requests.applied.as_ref().map_or("-", |c| c.display.as_str()),
            resident.offered.as_ref().map_or("-", |o| o.label.as_str()),
            self.scanning,
            self.deferred_scan,
            self.restoring,
            self.pending.is_some(),
            self.effect_preparation.busy(),
            self.patch_browser.preparation.busy(),
            self.patch_catalog.notice(),
            self.patch_browser.query,
            self.patch_browser.role,
            self.patch_browser.preset,
            self.patch_browser.results.len(),
            self.status,
        );
        let resident = self.resident.as_mut().unwrap();
        if line != resident.logged {
            log(&line);
            resident.logged = line;
        }
    }

    pub(crate) fn resident_master(&self) -> bool {
        self.resident
            .as_ref()
            .is_some_and(|r| matches!(r.shown, Some(Open::MasterEffect { .. })))
    }
}

#[cfg(test)]
#[path = "resident_tests.rs"]
mod tests;
