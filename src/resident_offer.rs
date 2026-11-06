//! Prepared state and its matching catalog notes cross the resident boundary together.
use crate::{
    cli::Browser,
    config::PluginKey,
    resident::{browser, log, Offer},
    resident_ipc::{Open, Selected, ToEditor},
    App,
};
struct Choice {
    browser: Browser,
    plugin: PluginKey,
    preset_name: Option<String>,
    label: String,
    source: crate::patch_reference::Source,
    kit: Option<crate::drum_kit::Kit>,
}

impl App {
    /// Sends a prepared patch to the editor in place of loading it here.
    pub(crate) fn offer_patch(&mut self, prepared: crate::patch_preparation::Prepared) {
        let candidate = &prepared.candidate;
        let kit = if self.resident_kit() {
            let layer = match self.resident.as_ref().and_then(|r| r.shown) {
                Some(Open::Kit { layer, .. }) => layer.unwrap_or(1),
                _ => return,
            };
            match crate::drum_browser::kit_metadata(candidate, layer) {
                Ok(kit) => Some(kit),
                Err(error) => {
                    self.status = error;
                    return;
                }
            }
        } else {
            None
        };
        let label = format!("{} — {}", candidate.plugin_name, candidate.display);
        let result = self
            .plugins
            .iter()
            .find(|plugin| {
                plugin.format == candidate.format
                    && plugin.id == candidate.plugin_id
                    && plugin.kind == crate::plugin_list::PluginKind::Instrument
            })
            .map(PluginKey::from_plugin)
            .ok_or_else(|| "target plugin no longer available".to_string())
            .and_then(|plugin| {
                let name = Some(candidate.display.clone());
                let file = if candidate.plugin_id == cmrt_core::DEXED_PLUGIN_ID {
                    std::path::PathBuf::from(
                        cmrt_core::parse_cartridge_patch_path(
                            candidate.path.to_str().ok_or("Patch path is not UTF-8")?,
                        )
                        .map_err(|e| e.to_string())?
                        .cartridge_path,
                    )
                } else {
                    candidate.path.clone()
                };
                let source = crate::patch_reference::Source::Catalog {
                    path: candidate.path.clone(),
                    file,
                };
                self.offer(
                    Choice {
                        browser: Browser::Patch,
                        plugin,
                        preset_name: name,
                        label: label.clone(),
                        source,
                        kit,
                    },
                    &prepared.state,
                )
            });
        log(format_args!("offer patch {label}: {result:?}"));
        match result {
            Ok(()) => self.status = format!("Sending browse patch: {label}..."),
            Err(error) => {
                self.patch_browser.requests.finish(false);
                self.status = format!("Browse patch failed: {error}");
            }
        }
    }

    /// Sends a prepared effect preset to the editor in place of loading it here.
    pub(crate) fn offer_effect(&mut self, prepared: crate::effect_preparation::Prepared) {
        let candidate = &prepared.candidate;
        let label = candidate.preset.display.clone();
        let result = self
            .plugins
            .iter()
            .find(|plugin| crate::effect_catalog::matches_plugin(candidate, plugin))
            .map(PluginKey::from_plugin)
            .ok_or_else(|| "plugin no longer available".to_string())
            .and_then(|plugin| {
                let name = Some(label.clone());
                self.offer(
                    Choice {
                        browser: Browser::Effect,
                        plugin,
                        preset_name: name,
                        label: label.clone(),
                        source: crate::patch_reference::Source::Effect {
                            path: candidate.preset.path.clone(),
                            value: candidate.preset.value.clone(),
                        },
                        kit: None,
                    },
                    &prepared.state,
                )
            });
        log(format_args!("offer effect {label}: {result:?}"));
        self.status = match result {
            Ok(()) => format!("Sending effect: {label}..."),
            Err(error) => format!("Browse effect failed: {error}"),
        };
    }

    fn offer(&mut self, choice: Choice, state: &[u8]) -> Result<(), String> {
        let Choice {
            browser,
            plugin,
            preset_name,
            label,
            source,
            kit,
        } = choice;
        let resident = self.resident.as_mut().unwrap();
        let target = resident
            .shown
            .filter(|&open| self::browser(open) == browser)
            .ok_or("the browser was closed")?;
        if resident.preparation_target != Some(target) {
            return Err("Selection target changed during state preparation".into());
        }
        let shared = resident.sender.share(state).map_err(|e| e.to_string())?;
        resident
            .sender
            .send(&ToEditor::Selected(Selected {
                kit,
                reference: Some(Box::new(crate::patch_reference::PatchReference {
                    plugin: plugin.clone(),
                    source,
                    display: preset_name.clone().unwrap_or_else(|| plugin.name.clone()),
                })),
                target,
                plugin,
                preset_name,
                state: shared.reference.clone(),
            }))
            .map_err(|e| e.to_string())?;
        resident.offered = Some(Offer {
            browser,
            label,
            _state: shared,
        });
        Ok(())
    }
}
