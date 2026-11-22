//! Inactive browser instruments have no renderer and never enter the live slot list.
use crate::{config::PluginKey, ffi::PluginInfo, plugin_list::PluginKind, App, Instance};
use std::{ffi::OsString, path::PathBuf, time::SystemTime};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Identity {
    format: String,
    id: String,
    bundle: PathBuf,
    size: u64,
    modified: SystemTime,
    rate: u32,
    surge_home: Option<OsString>,
}

impl Identity {
    fn matches(&self, plugin: &PluginInfo, rate: u32) -> bool {
        plugin.format == self.format
            && plugin.id == self.id
            && Self::read(plugin, rate).as_ref() == Some(self)
    }

    pub(crate) fn read(plugin: &PluginInfo, rate: u32) -> Option<Self> {
        if plugin.format != "CLAP" || plugin.kind != PluginKind::Instrument {
            return None;
        }
        let bundle = std::fs::canonicalize(&plugin.bundle_path).ok()?;
        let metadata = std::fs::metadata(&bundle).ok()?;
        Some(Self {
            format: plugin.format.clone(),
            id: plugin.id.clone(),
            bundle,
            size: metadata.len(),
            modified: metadata.modified().ok()?,
            rate,
            surge_home: (plugin.id == crate::patch_catalog::SURGE_ID)
                .then(|| std::env::var_os("SURGE_DATA_HOME"))
                .flatten(),
        })
    }
}

pub(crate) struct Retained {
    pub(crate) instance: Instance,
    identity: Identity,
}

impl App {
    pub(crate) fn forget_retained(&mut self, key: &PluginKey) {
        let mut index = 0;
        while index < self.retained_instruments.len() {
            let plugin = &self.retained_instruments[index].instance.plugin;
            if plugin.format == key.format && plugin.id == key.id {
                let retained = self.retained_instruments.remove(index);
                self.destroy_inactive(retained.instance);
            } else {
                index += 1;
            }
        }
    }

    pub(crate) fn reconcile_retained(&mut self) {
        let rate = self.sample_rate();
        let mut index = 0;
        while index < self.retained_instruments.len() {
            let retained = &self.retained_instruments[index];
            let valid = self
                .plugins
                .iter()
                .any(|plugin| retained.identity.matches(plugin, rate));
            if valid {
                index += 1;
            } else {
                let retained = self.retained_instruments.remove(index);
                self.destroy_inactive(retained.instance);
            }
        }
    }

    pub(crate) fn take_retained(&mut self, plugin: &PluginInfo) -> Option<Retained> {
        self.reconcile_retained();
        let identity = Identity::read(plugin, self.sample_rate())?;
        let index = self
            .retained_instruments
            .iter()
            .position(|r| r.identity == identity)?;
        Some(self.retained_instruments.remove(index))
    }

    pub(crate) fn retire_browser_instrument(&mut self) {
        let Some(index) = self
            .instances
            .iter()
            .position(|i| i.kind == PluginKind::Instrument)
        else {
            return;
        };
        let instance = self.instances.remove(index);
        let identity = self.instrument_origin.take();
        // Opt-in measurement control reproduces the original destroy-on-switch
        // transaction; it is absent from production builds.
        #[cfg(test)]
        let identity = identity.filter(|_| std::env::var_os("PATCH_RETENTION_BASELINE").is_none());
        // pause_audio already joined the renderer and sent the existing note-offs.
        debug_assert!(instance.voice.is_none());
        self.host.hide_ui(instance.id);
        self.surge_start_delay.editor_closed(instance.id);
        self.favorites.active.retain(|(id, _)| *id != instance.id);
        let safe = !self.unsafe_state.contains(&instance.id);
        self.unsafe_state.remove(&instance.id);
        self.forget_retained(&instance.plugin);
        let identity = identity.filter(|identity| {
            safe && self
                .plugins
                .iter()
                .any(|p| identity.matches(p, self.sample_rate()))
        });
        if let Some(identity) = identity {
            self.retained_instruments
                .push(Retained { instance, identity });
        } else {
            self.destroy_inactive(instance);
        }
    }

    pub(crate) fn destroy_inactive(&mut self, mut instance: Instance) {
        instance.voice.take();
        self.surge_start_delay.forget(instance.id);
        self.unsafe_state.remove(&instance.id);
        self.favorites.active.retain(|(id, _)| *id != instance.id);
        self.host.destroy_instance(instance.id);
    }

    pub(crate) fn clear_retained(&mut self) {
        for retained in std::mem::take(&mut self.retained_instruments) {
            self.destroy_inactive(retained.instance);
        }
    }

    pub(crate) fn restore_retained_patch(
        &mut self,
        retained: Retained,
        replacement: crate::patch_apply::Replacement,
    ) {
        let Retained { instance, identity } = retained;
        self.patch_replacement(instance, Some(identity), replacement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuse_identity_checks_bundle_generation_rate_and_kind_not_scan_index() {
        let root =
            std::env::temp_dir().join(format!("cat-retained-identity-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let bundle = root.join("fixture.clap");
        std::fs::write(&bundle, b"first").unwrap();
        let mut plugin = PluginInfo {
            index: 1,
            name: "fixture".into(),
            vendor: String::new(),
            format: "CLAP".into(),
            id: "fixture".into(),
            bundle_path: bundle.to_string_lossy().into_owned(),
            kind: PluginKind::Instrument,
        };
        let original = Identity::read(&plugin, 48000).unwrap();
        plugin.index = 42;
        assert!(Identity::read(&plugin, 48000).as_ref() == Some(&original));
        assert!(Identity::read(&plugin, 44100).as_ref() != Some(&original));
        plugin.kind = PluginKind::Effect;
        assert!(Identity::read(&plugin, 48000).is_none());
        plugin.kind = PluginKind::Instrument;
        plugin.format = "VST3".into();
        assert!(Identity::read(&plugin, 48000).is_none());
        plugin.format = "CLAP".into();
        std::fs::write(&bundle, b"changed bundle").unwrap();
        assert!(Identity::read(&plugin, 48000).as_ref() != Some(&original));
        std::fs::remove_file(&bundle).unwrap();
        assert!(Identity::read(&plugin, 48000).is_none());
        std::fs::remove_dir(&root).unwrap();
    }
}
