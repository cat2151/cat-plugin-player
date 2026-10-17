use crate::{
    chain_settings::{self, PluginState, Settings},
    cli::Browser,
    smf,
    smf_timeline::Timeline,
    App,
};
use eframe::egui;
use std::{
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

pub struct Selection {
    pub(super) path: PathBuf,
    expected: Vec<u8>,
    music: smf::model::Music,
    pub settings: Settings,
    pub preview_chain: Option<Settings>,
    pub timeline: Arc<Timeline>,
    pub position: Arc<AtomicU64>,
    pub audio_failed: Arc<AtomicBool>,
    pub exit_requested: bool,
    pub saved: bool,
    pub restoration_failed: bool,
    pub browser_only: Option<Browser>,
    pub main_visible: bool,
    focus_browser: Option<Browser>,
    pub filter_path: Option<PathBuf>,
    pub effect_filter_path: Option<PathBuf>,
}

impl Selection {
    pub fn set_timing(&mut self, rate: u32, start: f64, end: Option<f64>) -> Result<(), String> {
        let samples = |seconds: f64| -> Result<u64, String> {
            let value = (seconds * f64::from(rate)).round();
            if rate == 0 || !value.is_finite() || value < 0.0 || value >= u64::MAX as f64 {
                return Err("Invalid selection time".into());
            }
            Ok(value as u64)
        };
        let duration = end
            .map(samples)
            .transpose()?
            .unwrap_or(self.timeline.duration);
        let start = samples(start)?;
        if duration < self.timeline.duration || start >= duration {
            return Err(
                "Selection start must precede loop end; loop end must cover input SMF".into(),
            );
        }
        let timeline = Arc::get_mut(&mut self.timeline).ok_or("Selection audio already started")?;
        timeline.duration = duration;
        self.position.store(start, Ordering::Release);
        Ok(())
    }

    #[cfg(test)]
    pub fn load(path: PathBuf, rate: u32) -> Result<Self, String> {
        Self::load_mode(path, rate, false)
    }

    pub fn load_mode(path: PathBuf, rate: u32, master: bool) -> Result<Self, String> {
        let expected = std::fs::read(&path).map_err(|e| e.to_string())?;
        let music = smf::ffi::parse(&expected)?;
        let timeline = Arc::new(Timeline::new(&music, rate)?);
        let mut settings = chain_settings::read(&music.tracks[0])?;
        let preview_chain = if master {
            if music.format != 1 {
                return Err("Master effects editing requires SMF format 1".into());
            }
            let preview = settings.clone();
            let effects = crate::master_effects::read(&music)?;
            settings.effects = effects.effects;
            Some(preview)
        } else {
            None
        };
        Ok(Self {
            path,
            expected,
            music,
            settings,
            preview_chain,
            timeline,
            position: Arc::new(AtomicU64::new(0)),
            audio_failed: Arc::new(AtomicBool::new(false)),
            exit_requested: false,
            saved: false,
            restoration_failed: false,
            browser_only: None,
            main_visible: true,
            focus_browser: None,
            filter_path: None,
            effect_filter_path: None,
        })
    }

    pub fn save(&mut self, settings: &Settings) -> Result<(), String> {
        let mut music = self.music.clone();
        if self.preview_chain.is_some() {
            crate::master_effects::write(
                &mut music,
                &crate::master_effects::Settings {
                    version: settings.version,
                    effects: settings.effects.clone(),
                },
            )?;
        } else {
            chain_settings::write(&mut music.tracks[0], settings)?;
        }
        let bytes = smf::io::encode(&music)?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temporary = self.path.with_file_name(format!(
            ".cat-player-selection-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut created = false;
        let result = (|| -> Result<(), String> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|e| e.to_string())?;
            created = true;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            drop(file);
            if std::fs::read(&self.path).map_err(|e| e.to_string())? != self.expected {
                return Err("Selection file changed while editing".into());
            }
            std::fs::rename(&temporary, &self.path).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() && created {
            let _ = std::fs::remove_file(&temporary);
        }
        result?;
        self.expected = bytes;
        self.settings = settings.clone();
        self.saved = true;
        Ok(())
    }
}

pub fn exit_key(ctx: &egui::Context) -> bool {
    let escape = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    let quit = !crate::patch_browser_ui::editing_text(ctx)
        && !ctx.input(|i| i.pointer.any_pressed())
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Q));
    escape || quit || ctx.input(|i| i.viewport().close_requested())
}

impl App {
    pub(crate) fn configure_browser_only(&mut self, browser: Browser) {
        if let Some(selection) = &mut self.selection {
            selection.browser_only = Some(browser);
            selection.main_visible = false;
            selection.focus_browser = Some(browser);
        }
    }

    pub(crate) fn take_browser_focus(&mut self, browser: Browser) -> bool {
        if self.take_resident_focus(browser) {
            return true;
        }
        let Some(selection) = &mut self.selection else {
            return false;
        };
        if selection.focus_browser == Some(browser) {
            selection.focus_browser = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn selection_browser_closed(&mut self, browser: Browser) {
        let closed = match browser {
            Browser::Patch => !self.patch_browser.open,
            Browser::Effect => !self.effect_browser.open,
        };
        if closed {
            self.resident_browser_closed(browser);
        }
        if closed
            && self
                .selection
                .as_ref()
                .is_some_and(|s| s.browser_only.is_some())
        {
            self.selection.as_mut().unwrap().exit_requested = true;
            // Keep a usable browser until any pending change/save has actually succeeded.
            match browser {
                Browser::Patch => self.patch_browser.open = true,
                Browser::Effect => self.effect_browser.open = true,
            }
        }
    }

    #[cfg(test)]
    pub fn configure_selection(&mut self, path: PathBuf, browser: Browser) -> Result<(), String> {
        self.configure_selection_mode(path, browser, false)
    }

    pub fn configure_selection_mode(
        &mut self,
        path: PathBuf,
        browser: Browser,
        master: bool,
    ) -> Result<(), String> {
        self.shutdown.selection_requested = true;
        let selection = Selection::load_mode(path, self.sample_rate(), master)?;
        self.restore = Some(
            selection
                .settings
                .instrument
                .as_ref()
                .map(|s| s.plugin.clone())
                .map_or_else(crate::selection_host::default_instrument, Ok)?,
        );
        self.restore_effect = selection
            .settings
            .effects
            .iter()
            .map(|s| s.plugin.clone())
            .collect();
        self.effect_bypassed = selection.settings.bypass;
        self.mml_input = Default::default();
        self.sequence_pattern = crate::SequencePattern::Steps;
        self.selected_sequence = crate::SequencePattern::Steps;
        self.sequence_modulation = crate::SequenceModulation::Zero;
        self.auto_start_pending = false;
        self.favorites.restore = Default::default();
        self.favorites.active.clear();
        self.patch_browser.open = !master && browser == Browser::Patch;
        self.effect_browser.open = master || browser == Browser::Effect;
        self.config_error = None;
        self.status = format!(
            "SMF selection: {} · {}",
            selection.music.tracks[0]
                .name()
                .unwrap_or_else(|| "track".into()),
            selection.settings.label()
        );
        if master {
            self.status = format!(
                "Master effects selection: {} · preview track",
                crate::master_effects::read(&selection.music)?.label()
            );
        }
        self.selection = Some(selection);
        Ok(())
    }

    pub(crate) fn selection_committed(&mut self) {
        if let Some(selection) = &mut self.selection {
            selection.restoration_failed = false;
            self.config_error = None;
        }
    }

    pub(crate) fn selecting_master(&self) -> bool {
        self.resident_master()
            || self
                .selection
                .as_ref()
                .is_some_and(|s| s.preview_chain.is_some())
    }

    /// Esc, Q, or close inside a browser: hide when resident, else save and exit.
    pub(crate) fn browser_exit_requested(&mut self) {
        if self.resident.is_some() {
            self.hide_resident();
        } else if let Some(selection) = &mut self.selection {
            selection.exit_requested = true;
        }
    }

    pub(crate) fn selection_exit(&mut self, ctx: &egui::Context) -> bool {
        let Some(selection) = &self.selection else {
            return false;
        };
        if selection.saved {
            return true;
        }
        if !selection.exit_requested {
            return false;
        }
        if self.pending.is_some()
            || self.preset_change_busy()
            || self.restoring
            || self.scanning
            || self.deferred_scan
            || self.patch_browser.requests.desired.is_some()
            || self.effect_browser.desired.is_some()
        {
            ctx.request_repaint();
            return false;
        }
        self.pause_audio();
        let result = (|| {
            if !self.unsafe_state.is_empty() || self.selection.as_ref().unwrap().restoration_failed
            {
                return Err(
                    "Settings were not restored safely; choose a valid sound before saving".into(),
                );
            }
            if self
                .selection
                .as_ref()
                .unwrap()
                .audio_failed
                .load(Ordering::Acquire)
            {
                return Err("Audio processing failed; choose a valid sound before saving".into());
            }
            // State queues must reach DSP before capture, including silent/no-device mode.
            // Audio is joined above; the temporary processor drops before state APIs.
            if let Some(source) = self.instrument_id() {
                let processor = self.chain_processor(source, &self.effect_ids())?;
                self.host.settle_processor(&processor, self.sample_rate())?;
            }
            let capture = |instance: &crate::Instance| -> Result<PluginState, String> {
                Ok(PluginState {
                    reference: None,
                    plugin: instance.plugin.clone(),
                    state: self.host.save_state(instance.id)?,
                    preset_name: instance.preset_name.clone(),
                    bypass: false,
                })
            };
            let instrument = self
                .instances
                .iter()
                .find(|i| i.kind == crate::plugin_list::PluginKind::Instrument)
                .map(capture)
                .transpose()?;
            let effects = self
                .instances
                .iter()
                .filter(|i| i.kind == crate::plugin_list::PluginKind::Effect)
                .map(capture)
                .collect::<Result<Vec<_>, _>>()?;
            let settings = Settings {
                instrument,
                effects,
                bypass: self.effect_bypassed,
                ..Default::default()
            };
            self.save_selection_filter()?;
            self.selection.as_mut().unwrap().save(&settings)
        })();
        match result {
            Ok(()) => {
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
                true
            }
            Err(error) => {
                self.selection.as_mut().unwrap().exit_requested = false;
                self.config_error = Some(format!("Could not save selection: {error}"));
                self.status = self.config_error.clone().unwrap();
                if let Err(error) = self.resume_audio() {
                    self.status = format!("Selection kept; no audio: {error}");
                }
                false
            }
        }
    }
}

pub(crate) fn show_error(ctx: &egui::Context, error: Option<&str>) {
    if let Some(error) = error {
        egui::TopBottomPanel::top("selection_save_error").show(ctx, |ui| {
            ui.colored_label(egui::Color32::YELLOW, error);
        });
    }
}
