//! First step: prove that egui and plugin editors can share the main thread.
//! Scan plugins, instantiate one, open its editor, and play a hardcoded phrase
//! using the sequence controls.

mod audio;
mod audio_fade;
mod boot;
mod browser_load;
mod cli;
mod config;
#[path = "../shared/correlation.rs"]
mod correlation;
#[path = "../shared/correlation_ui.rs"]
mod correlation_ui;
mod effect_apply;
mod effect_catalog;
mod effect_preparation;
mod error_log;
mod favorite_playback;
mod favorites;
mod favorites_store;
mod favorites_ui;
#[path = "../shared/ffi.rs"]
mod ffi;
mod first_instrument_install;
mod history;
mod host_ui;
use ffi as host;
use first_instrument_install::japanese_font;
#[path = "../shared/chain.rs"]
mod chain;
#[path = "../shared/chain_settings.rs"]
mod chain_settings;
mod launch;
#[path = "../shared/lissajous.rs"]
mod lissajous;
#[path = "../shared/lissajous_ui.rs"]
mod lissajous_ui;
#[path = "../shared/master_effects.rs"]
mod master_effects;
mod mml_input;
mod mml_preview;
mod native_library;
mod patch_apply;
mod patch_catalog;
mod patch_preparation;
#[path = "../shared/patch_reference.rs"]
mod patch_reference;
// The editor converts SMF references; the player only builds the table.
#[allow(dead_code)]
#[path = "../shared/patch_roots.rs"]
mod patch_roots;
mod patch_roots_request;
mod plugin_cache;
mod plugin_icons;
mod plugin_list;
mod plugin_scan;
mod plugin_specific;
mod plugins_ui;
mod preview;
#[cfg(feature = "recording-mode")]
mod recording;
mod repaint_heartbeat;
mod resident;
mod resident_cursor;
#[cfg(windows)]
#[allow(dead_code)]
#[path = "../shared/resident_ipc.rs"]
mod resident_ipc;
#[path = "../shared/resident_log.rs"]
mod resident_log;
mod resident_panes;
mod routing_ui;
mod scope;
mod scope_boundary;
mod scope_capture;
mod scope_drift;
mod scope_stream;
mod scope_trigger;
mod scope_ui;
mod selection;
mod selection_exchange;
mod state_request;
use chain as selection_host;
mod selection_ui;
mod seq;
mod sequence_modulation;
#[path = "../shared/smf.rs"]
mod smf;
#[path = "../shared/smf_extract.rs"]
mod smf_extract;
#[path = "../shared/smf_timeline.rs"]
mod smf_timeline;
#[path = "../shared/spectrum.rs"]
mod spectrum;
#[path = "../shared/spectrum_ui.rs"]
mod spectrum_ui;
mod surge_fx_states;
mod surge_patch_catalog;
mod timed_sequence;
use sequence_modulation::SequenceModulation;
mod effect_browser;
mod effect_browser_ui;
mod patch_browser;
mod patch_browser_catalog;
mod patch_browser_initial_filter;
mod patch_browser_requests;
mod patch_browser_snapshot;
mod patch_browser_ui;
mod patch_filter_store;
#[path = "../shared/patch_identity.rs"]
mod patch_identity;
mod playback_ui;
mod sequence_pattern;
mod sequence_ui;
mod sequence_velocity;
use sequence_pattern::SequencePattern;
use sequence_velocity::SequenceVelocity;
mod session;
mod session_load;
mod shutdown;
mod startup;
mod state_store;
mod status;
mod status_migration;
mod window_config;

use eframe::egui;
use startup::Stage;
use std::ffi::{c_char, c_void};
use std::sync::Mutex;

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
    /// Whether the most recently delivered automatic CC1 came from sweep.
    sweep_cc1: bool,
    /// Last preset applied by a browser; plugin GUI edits are not observable.
    preset_name: Option<String>,
}

struct App {
    // Declared before `host` so that it drops first: the audio streams and
    // processors in here must be gone before the host is destroyed.
    instances: Vec<Instance>,
    selection: Option<selection::Selection>,
    resident: Option<resident::Resident>,
    window_config: window_config::WindowConfig,
    host: ffi::Host,
    /// None if no usable audio device was found; plugins still load, silently.
    output: Option<audio::Output>,
    plugins: Vec<ffi::PluginInfo>,
    prefer_clap: bool,
    pending: Option<session_load::PendingLoad>,
    restore_effect: std::collections::VecDeque<config::PluginKey>,
    effect_bypassed: bool,
    sequence_pattern: SequencePattern,
    selected_sequence: SequencePattern,
    sequence_velocity: SequenceVelocity,
    sequence_modulation: SequenceModulation,
    mml_input: mml_input::MmlInput,
    restore: Option<config::PluginKey>,
    restored: Option<config::PluginKey>,
    restoring: bool,
    config_error: Option<String>,
    config_path: Result<std::path::PathBuf, String>,
    scanning: bool,
    catalog_startup: plugin_scan::Startup,
    confirm_rescan: bool,
    first_instrument: first_instrument_install::FirstInstrument,
    deferred_scan: bool,
    fast_restore: bool,
    filter: String,
    status: String,
    favorites: favorites::Favorites,
    scope_ui: scope_ui::ScopeUi,
    plugin_icons: plugin_icons::PluginIcons,
    repaint_heartbeat: Option<repaint_heartbeat::RepaintHeartbeat>,
    patch_catalog: patch_catalog::Catalog,
    patch_browser: patch_browser::Browser,
    effect_browser: effect_browser::EffectBrowser,
    auto_start_pending: bool,
    browser_auto_instrument: bool,
    browser_auto_effect: Option<browser_load::EffectLoad>,
    effect_preparation: effect_preparation::Preparation,
    /// Voices started while an effect swap is in progress fade in (ADR 0027).
    effect_swap_fade: bool,
    effect_catalog: effect_catalog::Catalog,
    unsafe_state: std::collections::HashSet<i32>,
    #[cfg(test)]
    patch_failures: Vec<patch_apply::Operation>,
    // Keep the overlay alive through native instance and host destruction.
    shutdown: shutdown::Shutdown,
}

impl App {
    fn sample_rate(&self) -> u32 {
        self.output
            .as_ref()
            .map_or(FALLBACK_SAMPLE_RATE, |o| o.sample_rate())
    }

    fn start_scan(&mut self, rescan: bool) {
        if self.preset_change_busy() || self.catalog_startup.running {
            return;
        }
        startup::mark(Stage::ScanRequested);
        self.patch_browser.requests.invalidate();
        if unsafe {
            self.host
                .scan_async(rescan, on_scan_done, std::ptr::null_mut())
        } {
            self.catalog_startup.cancel_refresh();
            self.scanning = true;
            self.status = if rescan {
                "Rescanning plugins...".into()
            } else {
                "Loading plugin list...".into()
            };
        }
    }

    fn log_status_error(&self, previous: &str) {
        if self.status == previous || !error_log::is_error(&self.status) {
            return;
        }
        if let Ok(path) = &self.config_path {
            if let Err(error) = error_log::append(path, &self.status) {
                eprintln!("{error}");
            }
        }
    }

    fn handle_events(&mut self) {
        let events = std::mem::take(&mut *EVENTS.lock().unwrap());
        for event in events {
            match event {
                Event::ScanDone(error) => {
                    if self.finish_background_scan(error.as_deref()) {
                        continue;
                    }
                    self.scanning = false;
                    self.plugins = self.host.plugins();
                    self.patch_catalog.reconcile(&self.plugins);
                    startup::mark(Stage::CatalogReady);
                    let scan_succeeded = error.is_none();
                    self.status = match error {
                        Some(e) => format!("Scan failed: {e}"),
                        None => String::new(),
                    };
                    if scan_succeeded {
                        self.save_plugin_cache();
                        if let Some(index) = self.first_instrument.on_scan(&self.plugins) {
                            // Restoring too would leave `restoring` set behind this pending load.
                            self.restore = None;
                            self.restore_effect.clear();
                            self.load_auto_instrument(index);
                        } else {
                            self.restore_session();
                        }
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

    /// Waits for the first audio callback; a resident player has no audio to wait for.
    fn poll_patch_catalog(&mut self, ctx: &egui::Context) {
        let ready = self.resident.is_some()
            || self.instances.iter().any(|instance| {
                instance
                    .voice
                    .as_ref()
                    .is_some_and(audio::Voice::has_rendered)
            });
        if let Some(message) = self.patch_catalog.update(ready, ctx) {
            self.patch_catalog.reconcile(&self.plugins);
            if !message.is_empty() {
                eprintln!("{message}");
                self.status = message;
            }
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.begin_shutdown();
        self.repaint_heartbeat.take();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.poll_resident() {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        }
        if self.resident.is_some() || self.selection.as_ref().is_some_and(|s| !s.main_visible) {
            // eframe shows the root after its first paint, overriding the builder.
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if let Some(unsaved) = self.selection.as_mut().filter(|s| !s.saved) {
            if selection::exit_key(ctx) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                unsaved.exit_requested = true;
            }
        } else if ctx.input(|input| input.viewport().close_requested()) {
            self.begin_shutdown();
            self.save_on_shutdown();
            return;
        }
        let previous_status = self.status.clone();
        // Run what uapmd and the plugins queued for the main thread.
        self.host.pump();
        for voice in self.instances.iter().filter_map(|i| i.voice.as_ref()) {
            voice.pump_preview();
        }
        self.handle_events();
        if self
            .selection
            .as_ref()
            .is_some_and(|s| s.audio_failed.load(std::sync::atomic::Ordering::Acquire))
        {
            self.config_error = Some(
                "SMF audio processing failed; input kept. Choose valid settings before saving."
                    .into(),
            );
        }
        self.initialize_favorites();
        self.capture_plugin_icons(ctx);
        self.poll_effect_preparation();
        if let Some(notice) = self.effect_catalog.update(
            !self.scanning && !self.deferred_scan && !self.restoring,
            ctx,
        ) {
            eprintln!("{notice}");
            self.status = notice;
        }
        self.poll_patch_catalog(ctx);
        self.poll_catalog_startup(ctx);
        startup::flush_if_ready();

        self.poll_browser_load();
        self.browser_ui(ctx);
        self.poll_browser_patch(ctx);
        self.poll_effect_browser(ctx);
        self.effect_browser_ui(ctx);
        self.log_resident_state();
        if self.selection_exit(ctx) {
            return;
        }
        if self.selection.is_none() {
            self.mml_editor(ctx);
        }
        // Keep sound selection in its own pane beside the sequence controls.
        egui::TopBottomPanel::top("sequence_controls").show(ctx, |ui| {
            self.playback_controls(ui);
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
        self.first_instrument_dialog(ctx);
        self.log_status_error(&previous_status);
    }
}

fn main() -> eframe::Result {
    launch::run()
}

#[cfg(test)]
mod modulation_tests;

#[cfg(test)]
mod clap_state_tests;

#[cfg(test)]
use chain as shared_chain;
#[cfg(test)]
mod shared_host_tests;
#[cfg(test)]
#[path = "../shared/note_render.rs"]
mod shared_note_render;

#[cfg(test)]
mod selection_tests;

#[cfg(test)]
mod selection_timing_tests;

#[cfg(test)]
mod master_selection_tests;
