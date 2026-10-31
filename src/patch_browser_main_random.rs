//! Main-window `r`: the browser's own Random over its saved filter, with the window closed.
use super::*;

/// The closed browser builds its catalog and starts native work only for this request.
#[derive(Default, PartialEq, Debug)]
pub(crate) enum MainRandom {
    #[default]
    Idle,
    /// Waiting for the catalog snapshot.
    Catalog,
    /// A patch is chosen; waiting for its preparation to start.
    Start,
}

impl App {
    pub(crate) fn request_main_random(&mut self, ctx: &eframe::egui::Context) {
        if !self.patch_browser.open {
            self.patch_browser.begin_main_random(ctx);
        }
    }

    /// Call after the snapshot update, before the patch requests are polled.
    pub(crate) fn poll_main_random(&mut self) {
        if let Some(status) = self.patch_browser.poll_main_random() {
            self.status = status;
        }
    }
}

impl Browser {
    pub(crate) fn begin_main_random(&mut self, ctx: &eframe::egui::Context) {
        self.main_random_overlay = true;
        self.main_random = MainRandom::Catalog;
        // Selection can be slow. Let the keypress frame paint before starting it.
        self.main_random_paint_pass = frame_start_pass(ctx);
    }

    pub(crate) fn main_random_waiting_for_paint(&self, ctx: &eframe::egui::Context) -> bool {
        self.main_random_overlay && self.main_random_paint_pass == frame_start_pass(ctx)
    }

    pub(crate) fn show_main_random_overlay(&mut self, ctx: &eframe::egui::Context) {
        use eframe::egui::{self, Color32, FontId, Id, LayerId, Order};
        if !self.main_random_overlay {
            return;
        }
        let requests = &self.requests;
        if self.main_random == MainRandom::Idle
            && requests.desired.is_none()
            && requests.preparing.is_none()
            && requests.applying.is_none()
            && !self.preparation.busy()
        {
            self.main_random_overlay = false;
            return;
        }
        // Paint directly: an Area's initial sizing pass could hide the first frame.
        // This also stays on screen while native state application blocks update().
        let painter = ctx.layer_painter(LayerId::new(
            Order::Tooltip,
            Id::new("random_patch_overlay"),
        ));
        let center = ctx.screen_rect().center();
        let text = painter.layout_no_wrap(
            "Changing patch...".into(),
            FontId::proportional(24.0),
            Color32::WHITE,
        );
        // Name and catalog size tell a long sample load from a hang.
        let loading = [&requests.applying, &requests.preparing, &requests.desired]
            .into_iter()
            .find_map(|ticket| ticket.as_ref())
            .map(|ticket| &ticket.candidate);
        let detail = loading.map(|candidate| {
            painter.layout_no_wrap(
                format!(
                    "{} — {}\n{}",
                    candidate.plugin_name,
                    candidate.display,
                    crate::patch_browser_ui::sample_size(candidate.measurement.sfz_sample_bytes)
                ),
                FontId::proportional(16.0),
                Color32::LIGHT_GRAY,
            )
        });
        let gap = 8.0;
        let content = detail.as_ref().map_or(text.size(), |detail| {
            egui::vec2(
                text.size().x.max(detail.size().x),
                text.size().y + gap + detail.size().y,
            )
        });
        let rect = egui::Rect::from_center_size(center, content + egui::vec2(56.0, 40.0));
        painter.rect_filled(rect, 12.0, Color32::from_rgb(30, 30, 38));
        let top = center.y - content.y / 2.0;
        let detail_top = top + text.size().y + gap;
        painter.galley(
            egui::pos2(center.x - text.size().x / 2.0, top),
            text,
            Color32::WHITE,
        );
        if let Some(detail) = detail {
            let at = egui::pos2(center.x - detail.size().x / 2.0, detail_top);
            painter.galley(at, detail, Color32::LIGHT_GRAY);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }

    /// Returns the status to show when no sound will start.
    pub(crate) fn poll_main_random(&mut self) -> Option<String> {
        if self.main_random == MainRandom::Start && self.requests.desired.is_none() {
            self.main_random = MainRandom::Idle;
        }
        if self.main_random != MainRandom::Catalog {
            return None;
        }
        if let Some(error) = &self.build_error {
            self.main_random = MainRandom::Idle;
            return Some(format!("Random patch failed: {error}"));
        }
        self.snapshot.as_ref()?;
        self.select_random();
        self.main_random = MainRandom::Idle;
        if self.requests.desired.is_some() {
            self.main_random = MainRandom::Start;
            return None;
        }
        if let Some(error) = &self.error {
            return Some(format!(
                "Random patch failed: invalid browser search: {error}"
            ));
        }
        let Some(ticket) = self
            .requests
            .selected
            .as_ref()
            .filter(|_| !self.results.is_empty())
        else {
            return Some("Random patch: no patch matches the browser filter".into());
        };
        let candidate = &ticket.candidate;
        Some(if candidate.measurement.is_heavy_offline_load() {
            format!(
                "Random chose {} — {}: open Browse patches (T) and press Space to load it",
                candidate.plugin_name, candidate.display
            )
        } else {
            format!(
                "Random chose the current patch again: {} — {}",
                candidate.plugin_name, candidate.display
            )
        })
    }
}

// A discarded sizing pass runs update again without presenting its drawing.
// All passes in one rendered frame must defer the newly requested random work.
fn frame_start_pass(ctx: &eframe::egui::Context) -> u64 {
    ctx.cumulative_pass_nr() - ctx.output(|output| output.num_completed_passes as u64)
}
