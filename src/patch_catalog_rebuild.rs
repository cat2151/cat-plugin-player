//! Explicit CMRT catalog rebuild, off the UI and audio threads.
use crate::{patch_catalog, App};
use eframe::egui;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, TryRecvError};

type Result = std::result::Result<Vec<patch_catalog::Candidate>, String>;

#[derive(Default)]
pub(crate) struct Rebuild {
    result: Option<Receiver<Result>>,
}

impl Rebuild {
    fn busy(&self) -> bool {
        self.result.is_some()
    }

    fn start(&mut self, ctx: &egui::Context, work: impl FnOnce() -> Result + Send + 'static) {
        if self.busy() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.result = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(work());
            ctx.request_repaint();
        });
    }

    fn poll(&mut self) -> Option<Result> {
        let result = match self.result.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("Catalog rebuild worker disconnected".into()),
        };
        self.result = None;
        Some(result)
    }
}

fn run(command: &mut Command) -> std::result::Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = command
        .output()
        .map_err(|error| format!("Could not run cmrt: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = stderr.lines().rev().find(|line| !line.trim().is_empty());
    Err(format!(
        "cmrt failed ({}): {}",
        output.status,
        detail.unwrap_or("No error details")
    ))
}

impl App {
    pub(crate) fn can_rebuild_patch_catalog(&self) -> bool {
        !self.patch_catalog_rebuild.busy()
            && self.can_rescan()
            && !self.catalog_startup.running
            && !self.deferred_scan
    }

    pub(crate) fn rebuild_patch_catalog(&mut self, ctx: &egui::Context) {
        if !self.can_rebuild_patch_catalog() {
            return;
        }
        self.patch_catalog_rebuild.start(ctx, || {
            run(Command::new("cmrt").arg("build-patch-catalog-cache"))?;
            patch_catalog::load()
        });
        self.status = "Rebuilding patch catalog...".into();
    }

    pub(crate) fn poll_patch_catalog_rebuild(&mut self, ctx: &egui::Context) {
        // Adopt between sound changes, using the same generation invalidation as a rescan.
        if self.preset_change_busy() || self.scanning || self.restoring {
            return;
        }
        if let Some(result) = self.patch_catalog_rebuild.poll() {
            match result {
                Ok(candidates) => {
                    self.patch_catalog.replace(candidates, &self.plugins);
                    self.status = "Patch catalog rebuilt".into();
                }
                Err(error) => self.status = format!("Patch catalog rebuild failed: {error}"),
            }
        }
        if self.patch_catalog_rebuild.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests;
