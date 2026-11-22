//! Build the stopped instrument/effect route and attach its renderer.
#[cfg(test)]
use super::{headless, route_trace};
use crate::{audio, config, ffi, startup, App};

impl App {
    pub(crate) fn chain_processor(
        &self,
        instrument: i32,
        effects: &[i32],
    ) -> Result<ffi::Processor, String> {
        self.host
            .create_chain(
                if self.selecting_master() {
                    -1
                } else {
                    instrument
                },
                effects,
                self.effect_bypassed,
                self.sample_rate(),
                audio::MAX_BLOCK_FRAMES,
            )
            .ok_or_else(|| {
                "Cannot connect main audio buses (mono/stereo input and output required)".into()
            })
    }

    pub(crate) fn prepare_voice(
        &self,
        instrument: i32,
        effects: &[i32],
        waiting: bool,
        new_instrument: Option<&config::PluginKey>,
    ) -> Result<Option<audio::Voice>, String> {
        #[cfg(test)]
        route_trace::record("prepare");
        let instrument_plugin = new_instrument.or_else(|| {
            self.instances
                .iter()
                .find(|i| i.id == instrument)
                .map(|i| &i.plugin)
        });
        // ADR 0044: before the processor exists, as the state APIs require.
        if let Some(plugin) =
            instrument_plugin.filter(|plugin| crate::plugin_specific::checks_playable_state(plugin))
        {
            let state = self.host.save_state(instrument)?;
            if let Some(state) = crate::plugin_specific::playable_state(plugin, &state) {
                self.host.load_state(instrument, &state)?;
            }
        }
        let processor = self.chain_processor(instrument, effects)?;
        #[cfg(test)]
        if headless::enabled() && !waiting {
            let plugin = instrument_plugin.unwrap();
            let bindings =
                crate::macro_parameters::bind(&self.host, &processor, instrument, plugin)?;
            headless::install(processor, bindings, plugin, self.sample_rate());
            return Ok(None);
        }
        if waiting || self.output.is_none() {
            return Ok(None);
        }
        startup::mark(startup::Stage::ProcessorReady);
        let start_delay = instrument_plugin
            .map(|plugin| {
                self.surge_start_delay.delay(
                    instrument,
                    plugin,
                    crate::plugin_specific::automatic_note_start_delay_at_rate(
                        plugin,
                        self.sample_rate(),
                    ),
                )
            })
            .unwrap_or_default();
        // Every audio renderer is stopped here, as preparing parameters requires.
        let macro_parameters = match instrument_plugin {
            Some(plugin) if !self.selecting_master() => {
                crate::macro_parameters::bind(&self.host, &processor, instrument, plugin)
                    .map_err(|error| format!("Macro parameters: {error}"))?
            }
            _ => Default::default(),
        };
        audio::Voice::start(
            self.output.as_ref().unwrap(),
            processor,
            if self.drum_kit_active() {
                crate::SequencePattern::Off
            } else {
                self.sequence_pattern
            },
            self.sequence_velocity,
            self.sequence_modulation,
            &self.macro_lane.active(),
            macro_parameters,
            start_delay,
            self.mml_input.phrase.clone(),
            self.effect_swap_fade,
            self.selection.as_ref().map(|s| {
                (
                    s.timeline.clone(),
                    s.position.clone(),
                    s.audio_failed.clone(),
                )
            }),
            self.selection
                .as_ref()
                .and_then(|s| s.preview_chain.as_ref())
                .map(|settings| crate::chain::Chain::open(self.sample_rate(), settings))
                .transpose()?,
        )
        .map(Some)
    }

    pub(crate) fn start_voice(&self, id: i32) -> Result<audio::Voice, String> {
        self.prepare_voice(id, &self.effect_ids(), false, None)?
            .ok_or_else(|| "no audio device".into())
    }
}
