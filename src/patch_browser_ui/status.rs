//! Show pending selection and actionable warnings; ordinary success needs no footer.
use super::*;
use crate::patch_browser_requests::same;

impl Browser {
    pub(super) fn status_panel(&self, ctx: &egui::Context, status: &str) {
        let selected = self
            .requests
            .selected
            .as_ref()
            .map(|ticket| &ticket.candidate);
        let differs = selected.is_some_and(|selected| {
            !self
                .requests
                .applied
                .as_ref()
                .is_some_and(|applied| same(selected, applied))
        });
        let heavy = selected.is_some_and(|selected| selected.measurement.is_heavy_offline_load());
        // Do not interpret words in a patch name as operation failures.
        let success = self.requests.applied.as_ref().map(|applied| {
            format!(
                "Browse patch: {} — {}",
                applied.plugin_name, applied.display
            )
        });
        let message = if status.starts_with("Preparing browse patch:") {
            ""
        } else {
            success
                .as_ref()
                .and_then(|prefix| status.strip_prefix(prefix))
                .unwrap_or(status)
        }
        .to_ascii_lowercase();
        let warning = ["failed", "error", "no audio", "suppressed"]
            .iter()
            .any(|term| message.contains(term));
        let save_error = self.store.as_ref().and_then(|store| store.error.as_ref());
        if !differs && !heavy && !warning && save_error.is_none() {
            return;
        }
        // egui otherwise reuses the previous panel height, clipping newly added
        // warnings for a frame. All notices occupy one truncated line.
        let style = ctx.style();
        let frame = egui::Frame::side_top_panel(&style);
        let lines = usize::from(differs)
            + usize::from(heavy)
            + usize::from(warning)
            + usize::from(save_error.is_some());
        let font = egui::TextStyle::Body.resolve(&style);
        let line_height = ctx.fonts(|fonts| fonts.row_height(&font));
        let height = lines as f32 * line_height
            + (lines - 1) as f32 * style.spacing.item_spacing.y
            + frame.inner_margin.sum().y;
        egui::TopBottomPanel::bottom("browser_status")
            .frame(frame)
            .exact_height(height)
            .show(ctx, |ui| {
                if differs {
                    let selected = selected.unwrap();
                    let playing = self.requests.applied.as_ref().map_or_else(
                        || "current instrument state".into(),
                        |applied| format!("{} — {}", applied.plugin_name, applied.display),
                    );
                    compact_label(
                        ui,
                        format!(
                            "Not yet applied: {} — {}; playing: {playing}",
                            selected.plugin_name, selected.display,
                        ),
                    );
                }
                if heavy {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("Space: confirm sample load").color(PINK),
                        )
                        .truncate(),
                    )
                    .on_hover_text("Space: confirm sample load");
                }
                if warning {
                    ui.add(
                        egui::Label::new(egui::RichText::new(status).color(Color32::YELLOW))
                            .truncate(),
                    )
                    .on_hover_text(status);
                }
                if let Some(error) = save_error {
                    compact_label(ui, format!("Saved conditions: {error}"));
                }
            });
    }
}
