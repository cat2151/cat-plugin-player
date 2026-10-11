//! One-choice loading for startup and preset browsers; uses the normal load transaction.
use crate::{
    ffi::PluginInfo,
    plugin_list::{self, PluginKind},
    session_load::LoadPurpose,
    App, SequencePattern,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectLoad {
    NeedSource,
    SourceRequested,
}

fn only_plugin(plugins: &[PluginInfo], prefer_clap: bool, kind: PluginKind) -> Option<usize> {
    let mut matches = plugin_list::visible_indices(plugins, None, prefer_clap)
        .into_iter()
        .filter(|&index| plugins[index].kind == kind)
        // The effect preset browser supports CLAP only.
        .filter(|&index| kind != PluginKind::Effect || plugins[index].format == "CLAP");
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

impl App {
    pub(crate) fn load_auto_instrument(&mut self, index: usize) {
        if self.actions_busy() {
            return;
        }
        self.load_plugin(index);
        if let Some(pending) = &mut self.pending {
            pending.purpose = LoadPurpose::AutoInstrument;
        }
    }

    fn load_only_instrument(&mut self) {
        if self.instrument_id().is_none() && !self.actions_busy() {
            if let Some(index) =
                only_plugin(&self.plugins, self.prefer_clap, PluginKind::Instrument)
            {
                self.load_auto_instrument(index);
            }
        }
    }

    pub(crate) fn open_patch_browser(&mut self) {
        self.patch_browser.open_window();
        self.browser_auto_instrument = true;
        self.poll_browser_load();
    }

    /// Wait for startup scan/restoration or browser-triggered source creation.
    /// Consume each request once: a failure must not cause an automatic retry loop.
    pub(crate) fn poll_browser_load(&mut self) {
        if !self.effect_browser.open {
            self.browser_auto_effect = None;
        }
        if !self.patch_browser.open {
            self.browser_auto_instrument = false;
        }
        if self.actions_busy() {
            return;
        }
        if std::mem::take(&mut self.auto_start_pending)
            && only_plugin(&self.plugins, self.prefer_clap, PluginKind::Instrument).is_some()
        {
            if self.instrument_id().is_none() {
                self.load_only_instrument();
                self.browser_auto_instrument = false;
                if self.browser_auto_effect.is_some() {
                    self.browser_auto_effect = Some(EffectLoad::SourceRequested);
                }
            } else if self.sequence_pattern == SequencePattern::Off {
                self.sequence_pattern = self.sequence_pattern.selection(self.selected_sequence);
                for instance in &self.instances {
                    if let Some(voice) = &instance.voice {
                        voice.set_sequence(self.sequence_pattern);
                    }
                }
                self.save_session();
            }
        }
        if !self.actions_busy() && std::mem::take(&mut self.browser_auto_instrument) {
            self.load_only_instrument();
        }
        if self.actions_busy() || self.browser_auto_effect.is_none() {
            return;
        }
        if self.instrument_id().is_none() {
            if self.browser_auto_effect == Some(EffectLoad::SourceRequested) {
                self.browser_auto_effect = None;
                return;
            }
            self.load_only_instrument();
            if self.pending.is_some() {
                self.browser_auto_effect = Some(EffectLoad::SourceRequested);
                return;
            }
            self.browser_auto_effect = None;
            self.status = "Browse effect unavailable: load an instrument first".into();
            return;
        }
        self.browser_auto_effect = None;
        let Some(index) = only_plugin(&self.plugins, self.prefer_clap, PluginKind::Effect) else {
            return;
        };
        let key = crate::config::PluginKey::from_plugin(&self.plugins[index]);
        if let Some(id) = self.connected_effect(&key) {
            self.effect_browser.target = Some(id);
        } else {
            self.load_plugin(index);
            if let Some(pending) = &mut self.pending {
                pending.purpose = LoadPurpose::BrowserEffect;
            }
        }
    }
}

#[cfg(test)]
mod tests;
