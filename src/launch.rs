//! CLI configuration and native/audio startup before opening the GUI.
use super::*;
use clap::Parser;
use std::time::Instant;

pub(super) fn run() -> eframe::Result {
    let cli = cli::Cli::parse();
    if cli.restore_state {
        return crate::state_request::run().map_err(|e| eframe::Error::AppCreation(e.into()));
    }
    if cli.patch_roots {
        return crate::patch_roots_request::run().map_err(|e| eframe::Error::AppCreation(e.into()));
    }
    startup::init(Instant::now());
    let _native_library = native_library::load().map_err(eframe::Error::AppCreation)?;
    let mut app = App::prepare_mode(cli.select_smf.is_some() || cli.resident.is_some());
    app.load_plugin_cache();
    if let Some(name) = &cli.resident {
        let resident = resident::Resident::connect(name)
            .map_err(|e| eframe::Error::AppCreation(e.to_string().into()))?;
        app.resident = Some(resident);
    }
    let hidden = cli.browser_only || cli.resident.is_some();
    if let Some(path) = cli.select_smf {
        let browser = if cli.master_effects {
            cli::Browser::Effect
        } else {
            cli.browser.unwrap_or(cli::Browser::Patch)
        };
        app.configure_selection_mode(path, browser, cli.master_effects)
            .map_err(|e| eframe::Error::AppCreation(e.into()))?;
        if browser == cli::Browser::Patch {
            app.patch_browser = patch_browser::Browser {
                open: true,
                ..Default::default()
            };
            if let Some(name) = cli.patch_filter {
                app.patch_browser.set_initial_filter(&name);
            }
            app.configure_selection_filter(cli.patch_filter_file)
                .map_err(|e| eframe::Error::AppCreation(e.into()))?;
        }
        app.configure_effect_selection_filter(cli.effect_filter_file)
            .map_err(|e| eframe::Error::AppCreation(e.into()))?;
        let rate = app.sample_rate();
        if let Some(path) = cli.preview_smf {
            app.configure_selection_preview(&path)
                .map_err(|e| eframe::Error::AppCreation(e.into()))?;
        }
        app.selection
            .as_mut()
            .unwrap()
            .set_timing(rate, cli.start_seconds.unwrap_or(0.0), cli.loop_end_seconds)
            .map_err(|e| eframe::Error::AppCreation(e.into()))?;
        if cli.browser_only {
            app.configure_browser_only(browser);
        }
    }
    app.restore_before_gui();
    startup::mark(Stage::GuiRequested);
    let mut viewport = egui::ViewportBuilder::default()
        .with_visible(!hidden)
        .with_active(!hidden)
        .with_inner_size(app.window_config.main.inner_size());
    if let Some(position) = app.window_config.main.position() {
        viewport = viewport.with_position(position);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let result = eframe::run_native(
        "cat plugin player",
        options,
        Box::new(move |cc| {
            startup::mark(Stage::AppCreation);
            app.shutdown.set_window(cc);
            app.host
                .configure_editor_placement(cc, app.window_config.plugin.placement);
            *GUI_CONTEXT.lock().unwrap() = Some(cc.egui_ctx.clone());
            app.repaint_heartbeat = Some(repaint_heartbeat::RepaintHeartbeat::start(cc));
            Ok(Box::new(app))
        }),
    );
    *GUI_CONTEXT.lock().unwrap() = None;
    startup::flush();
    result
}
