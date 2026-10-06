//! One editor thumbnail shared across formats of the same plugin.
use crate::{config::PluginKey, App};
use eframe::egui;
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

const CAPTURE_DELAY: Duration = Duration::from_secs(2);
const RETRY_INTERVAL: Duration = Duration::from_secs(1);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(20);

#[path = "plugin_icon_store.rs"]
mod store;
use store::{identity, is_blank, load, save};

struct Opening {
    next: Instant,
    deadline: Instant,
    finished: bool,
}

enum CaptureError {
    Retry(String),
    Stop(String),
}

fn texture(ctx: &egui::Context, key: &PluginKey, image: &image::RgbaImage) -> egui::TextureHandle {
    ctx.load_texture(
        identity(key),
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}

#[derive(Default)]
pub(crate) struct PluginIcons {
    textures: HashMap<String, Option<egui::TextureHandle>>,
    aliases: HashMap<String, Vec<PluginKey>>,
    // Successful captures finish this opening; an unfinished editor can retry.
    openings: HashMap<i32, Opening>,
    pub error: Option<String>,
}

impl PluginIcons {
    fn register(&mut self, key: PluginKey) {
        let id = identity(&key);
        let aliases = self.aliases.entry(id.clone()).or_default();
        if aliases
            .iter()
            .any(|known| known.format == key.format && known.id == key.id)
        {
            return;
        }
        aliases.push(key);
        // A scan or favorite load can reveal an old screenshot after a placeholder was cached.
        if self.textures.get(&id).is_some_and(Option::is_none) {
            self.textures.remove(&id);
        }
    }

    pub(crate) fn draw(
        &mut self,
        ui: &mut egui::Ui,
        key: &PluginKey,
        config: &Result<PathBuf, String>,
    ) {
        let id = identity(key);
        if !self.textures.contains_key(&id) {
            let loaded = config.as_ref().map_err(Clone::clone).and_then(|config| {
                load(
                    config,
                    key,
                    self.aliases.get(&id).map_or(&[], Vec::as_slice),
                )
            });
            let handle = match loaded {
                Ok(Some(image)) => Some(texture(ui.ctx(), key, &image)),
                Ok(None) => None,
                Err(error) => {
                    self.error = Some(format!("Could not read icon for {}: {error}", key.name));
                    None
                }
            };
            self.textures.insert(id.clone(), handle);
        }
        // Match the controls' height so screenshots do not make the list rows taller.
        // Every row reserves the same space so names stay aligned.
        let height = ui.spacing().interact_size.y;
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(height * 1.5, height), egui::Sense::hover());
        if let Some(Some(handle)) = self.textures.get(&id) {
            let size = handle.size_vec2();
            let scale = (rect.width() / size.x).min(rect.height() / size.y);
            ui.put(
                egui::Rect::from_center_size(rect.center(), size * scale),
                egui::Image::new(handle).fit_to_exact_size(size * scale),
            )
            .on_hover_ui(|ui| {
                ui.add(egui::Image::new(handle));
            });
        } else {
            ui.painter()
                .rect_filled(rect.shrink(2.0), 4.0, ui.visuals().faint_bg_color);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "♪",
                egui::FontId::proportional((height - 4.0).min(22.0)),
                ui.visuals().weak_text_color(),
            );
        }
    }

    fn due(&mut self, id: i32, now: Instant) -> bool {
        let opening = self.openings.entry(id).or_insert_with(|| Opening {
            next: now + CAPTURE_DELAY,
            deadline: now + CAPTURE_TIMEOUT,
            finished: false,
        });
        if !opening.finished && now >= opening.next {
            opening.finished = true;
            true
        } else {
            false
        }
    }

    fn retry(&mut self, id: i32, now: Instant) -> bool {
        let Some(opening) = self.openings.get_mut(&id) else {
            return false;
        };
        if now + RETRY_INTERVAL > opening.deadline {
            return false;
        }
        opening.next = now + RETRY_INTERVAL;
        opening.finished = false;
        true
    }
}

impl App {
    pub(crate) fn capture_plugin_icons(&mut self, ctx: &egui::Context) {
        for key in self
            .plugins
            .iter()
            .map(PluginKey::from_plugin)
            .chain(
                self.favorites
                    .library
                    .entries
                    .iter()
                    .map(|favorite| favorite.plugin.clone()),
            )
            .chain(
                self.instances
                    .iter()
                    .map(|instance| instance.plugin.clone()),
            )
        {
            self.plugin_icons.register(key);
        }
        self.plugin_icons.openings.retain(|id, _| {
            self.instances
                .iter()
                .any(|instance| instance.id == *id && self.host.ui_is_visible(*id))
        });
        let Ok(config) = &self.config_path else {
            return;
        };
        for instance in &self.instances {
            if !self.host.ui_is_visible(instance.id)
                || !self.plugin_icons.due(instance.id, Instant::now())
            {
                continue;
            }
            let result = (|| {
                // Read again at capture time: another instance may have saved it.
                let image = match load(
                    config,
                    &instance.plugin,
                    self.plugin_icons
                        .aliases
                        .get(&identity(&instance.plugin))
                        .map_or(&[], Vec::as_slice),
                )
                .map_err(CaptureError::Stop)?
                {
                    Some(image) => image,
                    None => {
                        let image = self
                            .host
                            .ui_thumbnail(instance.id)
                            .map_err(CaptureError::Retry)?;
                        if is_blank(&image) {
                            return Err(CaptureError::Retry("Plugin UI is still blank".into()));
                        }
                        save(config, &instance.plugin, &image).map_err(CaptureError::Stop)?;
                        image
                    }
                };
                Ok::<_, CaptureError>(texture(ctx, &instance.plugin, &image))
            })();
            match result {
                Ok(handle) => {
                    self.plugin_icons.error = None;
                    self.plugin_icons
                        .textures
                        .insert(identity(&instance.plugin), Some(handle));
                }
                Err(CaptureError::Retry(_))
                    if self.plugin_icons.retry(instance.id, Instant::now()) => {}
                Err(CaptureError::Retry(error) | CaptureError::Stop(error)) => {
                    self.plugin_icons.error = Some(format!(
                        "Could not save icon for {}: {error}. Reopen the UI to retry.",
                        instance.plugin.name
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> PluginKey {
        PluginKey {
            format: "CLAP".into(),
            id: "X:/plugins/synth.clap".into(),
            name: "Synth".into(),
            vendor: "Vendor".into(),
            bundle_path: String::new(),
        }
    }

    #[test]
    fn newly_discovered_format_invalidates_a_missing_shared_texture() {
        let mut icons = PluginIcons::default();
        let clap = key();
        icons.register(clap.clone());
        icons.textures.insert(identity(&clap), None);
        icons.register(clap.clone());
        assert!(icons.textures.contains_key(&identity(&clap)));
        let mut vst = clap.clone();
        vst.format = "VST3".into();
        vst.id = "vst3-id".into();
        icons.register(vst);
        assert!(!icons.textures.contains_key(&identity(&clap)));
        assert_eq!(icons.aliases[&identity(&clap)].len(), 2);
    }

    #[test]
    fn capture_waits_and_only_attempts_once_per_opening() {
        let mut icons = PluginIcons::default();
        let now = Instant::now();
        assert!(!icons.due(7, now));
        assert!(!icons.due(7, now + CAPTURE_DELAY / 2));
        assert!(icons.due(7, now + CAPTURE_DELAY));
        assert!(!icons.due(7, now + CAPTURE_DELAY * 2));
        icons.openings.remove(&7);
        assert!(!icons.due(7, now + CAPTURE_DELAY * 3));
        assert!(icons.due(7, now + CAPTURE_DELAY * 4));
    }

    #[test]
    fn unfinished_editor_retries_until_timeout_then_can_reopen() {
        let mut icons = PluginIcons::default();
        let now = Instant::now();
        assert!(!icons.due(7, now));
        let mut attempt = now + CAPTURE_DELAY;
        loop {
            assert!(icons.due(7, attempt));
            if !icons.retry(7, attempt) {
                break;
            }
            assert!(!icons.due(7, attempt + RETRY_INTERVAL / 2));
            attempt += RETRY_INTERVAL;
        }
        assert_eq!(attempt, now + CAPTURE_TIMEOUT);
        assert!(!icons.due(7, attempt + RETRY_INTERVAL));
        icons.openings.remove(&7);
        assert!(!icons.due(7, attempt));
        assert!(icons.due(7, attempt + CAPTURE_DELAY));
    }
}
