//! Content-sized columns and a viewport wide enough for both panes.
use super::*;

// Installed catalog measurement: 797.3 + 326.7 for the panes, plus 24 for spacing.
// Recheck with installed_drum_window_measurement after catalog/layout changes.
pub(super) const INITIAL_WIDTH: f32 = 1150.0;

impl App {
    pub(super) fn drum_kit_widths(&self, ui: &egui::Ui) -> [f32; 6] {
        kit_widths(&self.drum_browser, ui)
    }

    pub(super) fn fit_drum_window(&self, ui: &egui::Ui) {
        let [left, right] = pane_widths(&self.drum_browser, ui);
        let required = left + right + ui.spacing().item_spacing.x + 16.0;
        let size = ui.ctx().input(|input| {
            let viewport = input.viewport();
            let rect = viewport.inner_rect?;
            let limit = viewport.monitor_size.map_or(required, |size| size.x - 40.0);
            let width = required.min(limit).max(360.0);
            (rect.width() + 1.0 < width).then_some(egui::vec2(width, rect.height()))
        });
        if let Some(size) = size {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
    }
}

fn kit_widths(browser: &crate::drum_browser::Browser, ui: &egui::Ui) -> [f32; 6] {
    let measure = |text: &str| {
        ui.painter()
            .layout_no_wrap(
                text.into(),
                egui::TextStyle::Body.resolve(ui.style()),
                ui.visuals().text_color(),
            )
            .size()
            .x
            + 8.0
    };
    let mut widths = [
        "Plugin",
        "Kit / SFZ",
        "Path",
        "Notes",
        "Samples",
        "unavailable",
    ]
    .map(measure);
    widths[5] = widths[5]
        .max(measure("Space: load"))
        .max(measure("loading..."));
    for &index in &browser.results {
        let candidate = &browser.kits[index];
        for (width, text) in widths.iter_mut().zip([
            candidate.plugin_name.clone(),
            super::rows::kit_name(&candidate.display).into(),
            super::rows::kit_parent(candidate),
            notes(candidate).map_or("?".into(), |n| n.len().to_string()),
            super::rows::sample_size(candidate),
        ]) {
            *width = width.max(measure(&text));
        }
    }
    widths
}

pub(super) fn pane_widths(browser: &crate::drum_browser::Browser, ui: &egui::Ui) -> [f32; 2] {
    let left = kit_widths(browser, ui).iter().sum::<f32>()
        + super::rows::GAP * 5.0
        + ui.spacing().scroll.bar_width
        + 16.0;
    let right = browser.selected().map_or(260.0, |candidate| {
        super::rows::note_widths(ui, candidate).iter().sum::<f32>()
            + super::rows::GAP * 4.0
            + ui.spacing().scroll.bar_width
            + 16.0
    });
    [left, right]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "read-only measurement of installed drum catalog and saved cursor"]
    fn installed_drum_window_measurement() {
        let mut browser = crate::drum_browser::Browser::restored(&crate::config::path());
        assert!(browser.error.is_none(), "{:?}", browser.error);
        browser.kits = crate::patch_catalog::load()
            .unwrap()
            .into_iter()
            .filter(|candidate| candidate.measurement.drum_kit)
            .collect();
        assert!(!browser.kits.is_empty());
        let ctx = egui::Context::default();
        crate::japanese_font::add(&ctx);
        for unfiltered in [false, true] {
            if unfiltered {
                browser.saved.query.clear();
            }
            browser.filter();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let columns = kit_widths(&browser, ui);
                    let panes = pane_widths(&browser, ui);
                    println!("unfiltered={unfiltered}, kits={}, columns={columns:?}, panes={panes:?}, window={:.1}",
                        browser.results.len(), panes.iter().sum::<f32>() + ui.spacing().item_spacing.x + 16.0);
                });
            });
        }
    }
}
