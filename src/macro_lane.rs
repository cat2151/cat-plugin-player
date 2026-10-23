//! ADR 0036: automatic controllers for the instrument's macros, shown only when found.
use crate::plugin_specific::Controls;
use crate::sequence_modulation::{MacroLanes, MACROS};
use crate::{audio, plugin_list::PluginKind, App, SequenceModulation};
#[derive(Default)]
pub(crate) struct MacroLane {
    /// What the current instrument state reports; macro rows follow its CCs.
    pub controls: Controls,
    /// Per macro, `None` until chosen: sending nothing keeps the macro value saved in the patch.
    pub choices: [Option<SequenceModulation>; MACROS],
    /// The last read state, to tell a patch change from moved controls.
    state: Option<Vec<u8>>,
    /// (instrument, its parameter list changes) when `state` was read.
    seen: Option<(i32, u64)>,
    /// Read the state again, even without a patch change.
    pub reread: bool,
}

impl MacroLane {
    pub fn new(choices: [Option<SequenceModulation>; MACROS]) -> Self {
        Self {
            choices,
            ..Default::default()
        }
    }

    pub fn active(&self) -> MacroLanes {
        std::array::from_fn(|i| self.controls.targets[i].zip(self.choices[i]))
    }
}

impl App {
    /// Reads the instrument state only after the plugin rescans its parameter
    /// list, which follows every patch change, even one in its editor: reading
    /// a Surge XT state blocks the UI thread for 6-12 ms.
    pub(crate) fn poll_macro_lane(&mut self) {
        if let Some(code) = self
            .instances
            .iter()
            .filter_map(|i| i.voice.as_ref())
            .find_map(audio::Voice::take_parameter_failure)
        {
            self.status = format!("Sending macro parameters failed ({code}); macros stopped");
        }
        let Some(instrument) = self
            .instances
            .iter()
            .find(|i| i.kind == PluginKind::Instrument)
            .filter(|i| crate::plugin_specific::reports_controls(&i.plugin))
        else {
            self.macro_lane.seen = None;
            self.macro_lane.state = None;
            self.set_controls(Controls::default());
            return;
        };
        let id = instrument.id;
        let seen = Some((id, self.host.parameter_list_changes(id)));
        if (seen == self.macro_lane.seen && !self.macro_lane.reread)
            || self.actions_busy()
            || self.selection.is_some()
        {
            return;
        }
        let plugin = instrument.plugin.clone();
        self.macro_lane.seen = seen;
        self.macro_lane.reread = false;
        let Ok(state) = self.host.save_state(id) else {
            self.macro_lane.state = None;
            self.set_controls(Controls::default());
            return;
        };
        let changed = patch_changed(&plugin, self.macro_lane.state.as_deref(), &state);
        self.set_controls(crate::plugin_specific::controls(&plugin, &state));
        self.macro_lane.state = Some(state);
        if changed && self.release_fixed_controls() {
            self.save_session();
        }
    }

    fn set_controls(&mut self, controls: Controls) {
        let rerouted = controls.targets != self.macro_lane.controls.targets;
        self.macro_lane.controls = controls;
        if rerouted {
            self.apply_macro_lane();
        }
    }

    pub(crate) fn apply_macro_lane(&self) {
        let lanes = self.macro_lane.active();
        for voice in self.instances.iter().filter_map(|i| i.voice.as_ref()) {
            voice.set_macro_modulation(&lanes);
        }
    }

    /// Call before playing a patch the app loads. TyrellN6, Vaporizer2, Six Sines and
    /// Floe never rescan their parameter lists, so read the new patch's state anyway.
    pub(crate) fn patch_changing(&mut self) {
        self.macro_lane.state = None;
        self.macro_lane.reread = true;
        self.release_fixed_controls();
    }

    /// A fixed CC1 or macro value would replace the value saved in a new patch.
    /// Sweeps continue.
    fn release_fixed_controls(&mut self) -> bool {
        let mut released = SequenceModulation::release_fixed(&mut self.sequence_modulation);
        for choice in &mut self.macro_lane.choices {
            released |= SequenceModulation::release_fixed(choice);
        }
        if released {
            for voice in self.instances.iter().filter_map(|i| i.voice.as_ref()) {
                voice.set_modulation(self.sequence_modulation);
            }
            self.apply_macro_lane();
        }
        released
    }
}

/// The favorite comparison ignores the macro values the rows move, so moving them is
/// no patch change.
pub(crate) fn patch_changed(
    plugin: &crate::config::PluginKey,
    old: Option<&[u8]>,
    state: &[u8],
) -> bool {
    old.is_some_and(|old| !crate::plugin_specific::same_favorite_state(plugin, old, state))
}
