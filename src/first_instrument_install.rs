//! App-specific scan and playback adapter for the shared installer.
use crate::ffi;
#[path = "../shared/first_instrument_install.rs"]
mod shared;
pub(crate) use shared::japanese_font;
pub use shared::FirstInstrument;

impl crate::App {
    pub(crate) fn first_instrument_dialog(&mut self, ctx: &eframe::egui::Context) {
        if let Some(result) = self
            .first_instrument
            .show(ctx, !self.scanning && self.pending.is_none())
        {
            match result {
                Ok(()) => {
                    self.effect_catalog.refresh_after_install();
                    self.start_scan(true);
                }
                Err(detail) => self.status = format!("Surge XT install failed: {detail}"),
            }
        }
    }
}
