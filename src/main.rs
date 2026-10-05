//! First step: prove that egui and plugin editors can share the main thread.
//! Scan plugins, instantiate one, open its editor, and play a hardcoded phrase
//! (on by default) or a single test note.

mod audio;
mod config;
mod ffi;
mod seq;
mod session;

use eframe::egui;
use std::ffi::{c_char, c_void};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Used when there is no audio device to take the rate from.
const FALLBACK_SAMPLE_RATE: u32 = 48_000;

const TEST_NOTE: u8 = 60;
const TEST_VELOCITY: u8 = 127;
const TEST_NOTE_LENGTH: Duration = Duration::from_secs(1);

/// What the shim reported. The callbacks run on the main thread inside
/// `Host::pump()`, i.e. while `update()` holds `&mut App`, so they cannot touch
/// the app directly; they leave a note here and `update()` picks it up.
enum Event {
    ScanDone(Option<String>),
    InstanceCreated { id: i32, error: Option<String> },
}

static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

fn push_event(event: Event) {
    EVENTS.lock().unwrap().push(event);
}

unsafe extern "C" fn on_wake(user: *mut c_void) {
    // Any thread. `user` is the leaked egui::Context from App::new().
    let ctx = &*(user as *const egui::Context);
    ctx.request_repaint();
}

unsafe extern "C" fn on_scan_done(_user: *mut c_void, error: *const c_char) {
    push_event(Event::ScanDone(ffi::error_string(error)));
}

unsafe extern "C" fn on_instance(_user: *mut c_void, instance_id: i32, error: *const c_char) {
    push_event(Event::InstanceCreated {
        id: instance_id,
        error: ffi::error_string(error),
    });
}

struct Instance {
    id: i32,
    label: String,
    plugin: config::PluginKey,
    /// None if audio could not be started for this instance.
    voice: Option<audio::Voice>,
    /// When the test note that is sounding has to be released.
    note_off_at: Option<Instant>,
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
    restore: Option<config::PluginKey>,
    config_error: Option<String>,
    scanning: bool,
    filter: String,
    status: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Leaked on purpose: the shim may call on_wake() from other threads for
        // as long as the process lives.
        let ctx: *mut egui::Context = Box::into_raw(Box::new(cc.egui_ctx.clone()));
        let host = ffi::Host::new(on_wake, ctx as *mut c_void)
            .expect("uh_create() failed: could not set up the uapmd plugin host");

        let (output, audio_error) = match audio::Output::open_default() {
            Ok(output) => (Some(output), None),
            Err(e) => (None, Some(e)),
        };

        let (restore, config_error) =
            match config::path().and_then(|path| config::Config::load(&path)) {
                Ok(config) => (config.last_played, None),
                Err(error) => (None, Some(format!("Could not read session: {error}"))),
            };
        let mut app = Self {
            instances: Vec::new(),
            host,
            output,
            plugins: Vec::new(),
            pending: None,
            restore,
            config_error,
            scanning: false,
            filter: String::new(),
            status: String::new(),
        };
        app.start_scan(false);
        if let Some(e) = audio_error {
            app.status = format!("No audio: {e}");
        }
        app
    }

    fn sample_rate(&self) -> u32 {
        self.output
            .as_ref()
            .map_or(FALLBACK_SAMPLE_RATE, |o| o.sample_rate())
    }

    /// Starts audio for a freshly created instance.
    fn start_voice(&self, instance_id: i32) -> Result<audio::Voice, String> {
        let output = self.output.as_ref().ok_or("no audio device")?;
        let processor = self
            .host
            .create_processor(instance_id, output.sample_rate(), audio::MAX_BLOCK_FRAMES)
            .ok_or("could not set up audio processing for the plugin")?;
        // The phrase starts as soon as the plugin is ready.
        audio::Voice::start(output, processor, true)
    }

    /// Releases test notes whose time is up.
    fn release_due_notes(&mut self) {
        let now = Instant::now();
        for instance in &mut self.instances {
            if instance.note_off_at.is_some_and(|at| now >= at) {
                instance.note_off_at = None;
                if let Some(voice) = &mut instance.voice {
                    voice.send(audio::note_off(0, TEST_NOTE));
                }
            }
        }
    }

    fn start_scan(&mut self, rescan: bool) {
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
                    let scan_succeeded = error.is_none();
                    self.status = match error {
                        Some(e) => format!("Scan failed: {e}"),
                        None => format!("{} plugins", self.plugins.len()),
                    };
                    if scan_succeeded {
                        self.restore_session();
                    }
                }
                Event::InstanceCreated { id, error } => {
                    let Some(plugin) = self.pending.take() else {
                        continue;
                    };
                    let label = format!("{} [{}]", plugin.name, plugin.format);
                    match error {
                        None if id >= 0 => {
                            let voice = match self.start_voice(id) {
                                Ok(voice) => {
                                    // Only the newest instance plays the phrase by
                                    // itself: the ones loaded earlier fall silent.
                                    for earlier in &self.instances {
                                        if let Some(v) = &earlier.voice {
                                            v.set_sequence(false);
                                        }
                                    }
                                    self.status = format!("Loaded {label}");
                                    Some(voice)
                                }
                                Err(e) => {
                                    self.status = format!("Loaded {label}, but no audio: {e}");
                                    None
                                }
                            };
                            let playing = voice.is_some();
                            self.instances.push(Instance {
                                id,
                                label,
                                plugin: config::PluginKey::from_plugin(&plugin),
                                voice,
                                note_off_at: None,
                            });
                            if playing {
                                self.remember_played(id);
                            }
                        }
                        Some(e) => self.status = format!("Could not load {label}: {e}"),
                        None => self.status = format!("Could not load {label}"),
                    }
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
        self.release_due_notes();
        // egui only repaints on input. Plugins need the main thread at any time,
        // so keep a slow heartbeat going in addition to on_wake().
        ctx.request_repaint_after(Duration::from_millis(50));

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.scanning, egui::Button::new("Rescan"))
                    .clicked()
                {
                    self.start_scan(true);
                }
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.filter);
                ui.label(&self.status);
            });
            if let Some(error) = &self.config_error {
                ui.colored_label(egui::Color32::YELLOW, error);
            }
        });

        // Clicks are collected first and acted on after the panels are drawn.
        let mut show: Option<i32> = None;
        let mut hide: Option<i32> = None;
        let mut remove: Option<i32> = None;
        let mut test_note: Option<i32> = None;
        let mut set_sequence: Option<(i32, bool)> = None;
        egui::SidePanel::right("instances")
            .min_width(420.0)
            .show(ctx, |ui| {
                ui.heading("Instances");
                for instance in &self.instances {
                    ui.horizontal(|ui| {
                        ui.label(&instance.label);
                        if let Some(voice) = &instance.voice {
                            let mut on = voice.sequence_on();
                            if ui.toggle_value(&mut on, "Sequence").changed() {
                                set_sequence = Some((instance.id, on));
                            }
                        }
                        if ui
                            .add_enabled(instance.voice.is_some(), egui::Button::new("Test note"))
                            .on_disabled_hover_text("No audio for this instance")
                            .clicked()
                        {
                            test_note = Some(instance.id);
                        }
                        if ui.button("Show UI").clicked() {
                            show = Some(instance.id);
                        }
                        if ui.button("Hide UI").clicked() {
                            hide = Some(instance.id);
                        }
                        if ui.button("Remove").clicked() {
                            remove = Some(instance.id);
                        }
                    });
                }
            });

        let mut load: Option<usize> = None;
        egui::CentralPanel::default().show(ctx, |ui| {
            let filter = self.filter.to_lowercase();
            let can_load = self.pending.is_none() && !self.scanning;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, plugin) in self.plugins.iter().enumerate() {
                    if !filter.is_empty() && !plugin.name.to_lowercase().contains(&filter) {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(can_load, egui::Button::new("Load"))
                            .clicked()
                        {
                            load = Some(i);
                        }
                        ui.label(format!(
                            "{} - {} [{}]",
                            plugin.name, plugin.vendor, plugin.format
                        ))
                        .on_hover_text(&plugin.id);
                    });
                }
            });
        });

        if let Some(i) = load {
            self.load_plugin(i);
        }
        if let Some((id, on)) = set_sequence {
            if let Some(instance) = self.instances.iter().find(|i| i.id == id) {
                if let Some(voice) = &instance.voice {
                    voice.set_sequence(on);
                    if on {
                        self.remember_played(id);
                    }
                }
            }
        }
        if let Some(id) = test_note {
            if let Some(instance) = self.instances.iter_mut().find(|i| i.id == id) {
                if let Some(voice) = &mut instance.voice {
                    // Pressed again while sounding: release the old note first.
                    if instance.note_off_at.is_some() {
                        voice.send(audio::note_off(0, TEST_NOTE));
                    }
                    if voice.send(audio::note_on(0, TEST_NOTE, TEST_VELOCITY)) {
                        instance.note_off_at = Some(Instant::now() + TEST_NOTE_LENGTH);
                        self.remember_played(id);
                    }
                }
            }
        }
        if let Some(id) = show {
            if let Err(code) = self.host.show_ui(id) {
                self.status = format!("Could not open the plugin UI (code {code})");
            }
        }
        if let Some(id) = hide {
            self.host.hide_ui(id);
        }
        if let Some(id) = remove {
            // Order matters: dropping the Instance stops its audio stream and frees
            // its processor; only then may the plugin instance itself go.
            self.instances.retain(|instance| instance.id != id);
            self.host.destroy_instance(id);
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "cat plugin player",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
