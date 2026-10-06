//! First step: prove that egui and plugin editors can share the main thread.
//! Scan plugins, instantiate one, open its editor, and play a hardcoded phrase
//! using the sequence controls.

mod audio;
mod boot;
mod cli;
mod config;
mod favorites;
mod favorites_store;
mod favorites_ui;
mod ffi;
mod native_library;
mod plugin_icons;
mod plugin_list;
mod plugin_specific;
mod routing_ui;
mod scope;
mod scope_boundary;
mod scope_capture;
mod scope_drift;
mod scope_stream;
mod scope_trigger;
mod scope_ui;
mod seq;
mod sequence_modulation;
mod spectrum;
mod spectrum_ui;
use sequence_modulation::SequenceModulation;
mod sequence_pattern;
mod sequence_ui;
mod sequence_velocity;
use sequence_pattern::SequencePattern;
use sequence_velocity::SequenceVelocity;
mod session;
mod session_load;
mod startup;
mod state_store;
mod updater;

use clap::Parser;
use eframe::egui;
use startup::Stage;
use std::ffi::{c_char, c_void};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Used when there is no audio device to take the rate from.
const FALLBACK_SAMPLE_RATE: u32 = 48_000;

/// What the shim reported. The callbacks run on the main thread inside
/// `Host::pump()`, i.e. while `update()` holds `&mut App`, so they cannot touch
/// the app directly; they leave a note here and `update()` picks it up.
enum Event {
    ScanDone(Option<String>),
    InstanceCreated { id: i32, error: Option<String> },
}

static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());
static GUI_CONTEXT: Mutex<Option<egui::Context>> = Mutex::new(None);

fn push_event(event: Event) {
    EVENTS.lock().unwrap().push(event);
}

unsafe extern "C" fn on_wake(_user: *mut c_void) {
    let context = GUI_CONTEXT.lock().unwrap().clone();
    if let Some(context) = context {
        context.request_repaint();
    }
}

unsafe extern "C" fn on_scan_done(_user: *mut c_void, error: *const c_char) {
    startup::mark(Stage::ScanCallback);
    push_event(Event::ScanDone(ffi::error_string(error)));
}

unsafe extern "C" fn on_instance(_user: *mut c_void, instance_id: i32, error: *const c_char) {
    startup::mark(Stage::InstanceCallback);
    push_event(Event::InstanceCreated {
        id: instance_id,
        error: ffi::error_string(error),
    });
}

struct Instance {
    id: i32,
    label: String,
    plugin: config::PluginKey,
    kind: plugin_list::PluginKind,
    /// Only the instrument owns the chain audio stream.
    voice: Option<audio::Voice>,
}

struct App {
    // Declared before `host` so that it drops first: the audio streams and
    // processors in here must be gone before the host is destroyed.
    instances: Vec<Instance>,
    host: ffi::Host,
    /// None if no usable audio device was found; plugins still load, silently.
    output: Option<audio::Output>,
    plugins: Vec<ffi::PluginInfo>,
    pending: Option<ffi::PluginInfo>,
    restore_effect: Option<config::PluginKey>,
    effect_bypassed: bool,
    sequence_pattern: SequencePattern,
    selected_sequence: SequencePattern,
    sequence_velocity: SequenceVelocity,
    sequence_modulation: SequenceModulation,
    restore: Option<config::PluginKey>,
    restored: Option<config::PluginKey>,
    restoring: bool,
    config_error: Option<String>,
    config_path: Result<std::path::PathBuf, String>,
    scanning: bool,
    confirm_rescan: bool,
    deferred_scan: bool,
    fast_restore: bool,
    filter: String,
    status: String,
    favorites: favorites::Favorites,
    scope_ui: scope_ui::ScopeUi,
    plugin_icons: plugin_icons::PluginIcons,
}

impl App {
    fn sample_rate(&self) -> u32 {
        self.output
            .as_ref()
            .map_or(FALLBACK_SAMPLE_RATE, |o| o.sample_rate())
    }

    fn start_scan(&mut self, rescan: bool) {
        startup::mark(Stage::ScanRequested);
        if self
            .host
            .scan_async(rescan, on_scan_done, std::ptr::null_mut())
        {
            self.scanning = true;
            self.status = if rescan {
                "Rescanning plugins...".into()
            } else {
                "Loading plugin list...".into()
            };
        }
    }

    fn handle_events(&mut self) {
        let events = std::mem::take(&mut *EVENTS.lock().unwrap());
        for event in events {
            match event {
                Event::ScanDone(error) => {
                    self.scanning = false;
                    self.plugins = self.host.plugins();
                    startup::mark(Stage::CatalogReady);
                    let scan_succeeded = error.is_none();
                    self.status = match error {
                        Some(e) => format!("Scan failed: {e}"),
                        None => format!("{} plugins", self.plugins.len()),
                    };
                    if scan_succeeded {
                        self.restore_session();
                    } else if self.restoring && self.instrument_id().is_some() {
                        self.load_failed(format!(
                            "{}; could not restore effect; playing instrument directly",
                            self.status
                        ));
                    }
                }
                Event::InstanceCreated { id, error } => {
                    self.instance_created(id, error);
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Run what uapmd and the plugins queued for the main thread.
        self.host.pump();
        self.handle_events();
        self.initialize_favorites();
        self.capture_plugin_icons(ctx);
        if self.deferred_scan && self.pending.is_none() {
            self.deferred_scan = false;
            self.start_scan(false);
        }
        startup::flush_if_ready();
        // egui only repaints on input. Plugins need the main thread at any time,
        // so keep a slow heartbeat going in addition to on_wake().
        ctx.request_repaint_after(Duration::from_millis(50));

        // Placement lives here; sequence_controls only draws the pane contents.
        egui::TopBottomPanel::top("sequence_controls").show(ctx, |ui| {
            self.sequence_controls(ui);
        });

        if let Some(error) = &self.config_error {
            egui::TopBottomPanel::top("config_error").show(ctx, |ui| {
                ui.colored_label(egui::Color32::YELLOW, error);
            });
        }

        // Snapshot placement so toggling takes effect together on the next frame.
        let analysis_on_right = self.scope_ui.on_right;
        self.scope_panel(ctx, analysis_on_right);
        self.routing_panel(ctx, analysis_on_right);
        self.library_panel(ctx);
    }
}

fn main() -> eframe::Result {
    if let Some(command) = cli::Cli::parse().command {
        let result = match command {
            cli::Command::Check => updater::check(),
            cli::Command::Update => updater::update(),
        };
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    startup::init(Instant::now());
    let _native_library = native_library::load().map_err(eframe::Error::AppCreation)?;
    let mut app = App::prepare();
    app.restore_before_gui();
    startup::mark(Stage::GuiRequested);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        "cat plugin player",
        options,
        Box::new(move |cc| {
            startup::mark(Stage::AppCreation);
            *GUI_CONTEXT.lock().unwrap() = Some(cc.egui_ctx.clone());
            Ok(Box::new(app))
        }),
    );
    *GUI_CONTEXT.lock().unwrap() = None;
    startup::flush();
    result
}

#[cfg(test)]
mod modulation_tests;
