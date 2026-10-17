//! Audio output for one instrument and its serial effects: a cpal stream whose callback pulls
//! audio from the shim and the sequencer that plays the selected phrase.

use crate::seq::{EventBuf, Sequencer};
use crate::sequence_modulation::SequenceModulation;
use crate::sequence_velocity::SequenceVelocity;
use crate::startup::{self, Stage};
use crate::{ffi, sequence_pattern::SequencePattern};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;

use crate::seq::note_off;

/// The largest block handed to the plugin at once. The device may ask for
/// more frames per callback; those are rendered in several blocks.
pub const MAX_BLOCK_FRAMES: u32 = 1024;

/// The default output device and the format it runs at.
pub struct Output {
    device: cpal::Device,
    config: cpal::StreamConfig,
}

impl Output {
    pub fn open_default() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no audio output device")?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("cannot query the audio device: {e}"))?;
        if supported.sample_format() != cpal::SampleFormat::F32 {
            return Err(format!(
                "audio device format {:?} is not supported yet (need f32)",
                supported.sample_format()
            ));
        }
        Ok(Self {
            device,
            config: supported.config(),
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }
}

/// A running audio stream for one instance.
///
/// Field order matters: fields drop in declaration order, and the stream has to
/// stop before the processor it renders with is destroyed.
pub struct Voice {
    _stream: Option<cpal::Stream>,
    _processor: ffi::Processor,
    _preview_chain: Option<crate::chain::Chain>,
    // Low byte: pattern. High bytes: restart generation, published atomically.
    sequence_pattern: Arc<AtomicU64>,
    sequence_velocity: Arc<AtomicU8>,
    sequence_modulation: Arc<AtomicU8>,
    current_velocity: Arc<AtomicU8>,
    current_modulation: Arc<AtomicU8>,
    cc1_source: Arc<AtomicU8>,
    rendered: Arc<AtomicBool>,
    scope: crate::scope::Scope,
    preview: Arc<crate::preview::Mailbox>,
    fade_out: Arc<AtomicBool>,
    faded: Arc<AtomicBool>,
}

impl Voice {
    pub(crate) fn pump_preview(&self) {
        if let Some(chain) = &self._preview_chain {
            chain.pump();
        }
    }
    /// Starts with the selected pattern, including the stopped state.
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        output: &Output,
        processor: ffi::Processor,
        sequence_pattern: SequencePattern,
        velocity: SequenceVelocity,
        modulation: SequenceModulation,
        start_delay: std::time::Duration,
        phrase: Option<Arc<crate::timed_sequence::Phrase>>,
        fade_in: bool,
        selection: Option<(
            Arc<crate::smf_timeline::Timeline>,
            Arc<AtomicU64>,
            Arc<AtomicBool>,
        )>,
        preview_chain: Option<crate::chain::Chain>,
    ) -> Result<Self, String> {
        let mut fade = crate::audio_fade::Fade::new(output.sample_rate(), fade_in);
        let fade_out = Arc::new(AtomicBool::new(false));
        let fade_out_flag = Arc::clone(&fade_out);
        let faded = Arc::new(AtomicBool::new(false));
        let faded_flag = Arc::clone(&faded);
        let preview = Arc::new(crate::preview::Mailbox::default());
        let preview_flag = preview.clone();
        let mut seen_preview = 0;
        let channels = output.config.channels as usize;
        let sequence_pattern = Arc::new(AtomicU64::new(sequence_pattern as u64));
        let sequence_flag = Arc::clone(&sequence_pattern);
        let mut seen_restart = 0;
        let sequence_velocity = Arc::new(AtomicU8::new(velocity as u8));
        let velocity_flag = Arc::clone(&sequence_velocity);
        let sequence_modulation = Arc::new(AtomicU8::new(modulation as u8));
        let modulation_flag = Arc::clone(&sequence_modulation);
        let current_velocity = Arc::new(AtomicU8::new(0));
        let current_modulation = Arc::new(AtomicU8::new(0));
        // 0: no CC1 delivered, 1: fixed modulation, 2: sweep.
        let cc1_source = Arc::new(AtomicU8::new(0));
        let cc1_source_flag = Arc::clone(&cc1_source);
        let velocity_meter = Arc::clone(&current_velocity);
        let modulation_meter = Arc::clone(&current_modulation);
        let mut sequencer = Sequencer::new(output.sample_rate());
        sequencer.set_phrase(phrase);
        sequencer.delay_start(start_delay);
        let selection_failure = selection.as_ref().map(|s| s.2.clone());
        if let Some(failed) = &selection_failure {
            failed.store(false, Ordering::Release);
        }
        let mut selected = selection.map(|(timeline, position, _)| {
            crate::smf_timeline::LoopPlayer::new(
                timeline,
                position,
                (start_delay.as_secs_f64() * f64::from(output.sample_rate())).ceil() as u64,
            )
        });
        let mut ump = EventBuf::new();
        // SAFETY: `_stream` is declared before `_processor`, so the callback is
        // gone before the processor is; only this one callback renders.
        let mut render = unsafe { processor.audio_ref() };
        let mut preview_render = preview_chain
            .as_ref()
            .map(|chain| unsafe { chain.audio_ref() });
        let mut trace_callback = startup::enabled();
        let mut trace_signal = trace_callback;
        let rendered = Arc::new(AtomicBool::new(false));
        let rendered_flag = Arc::clone(&rendered);
        let mut first_render = true;
        let (mut capture, scope) = crate::scope::Scope::start(output.sample_rate())?;
        let mut captured_pattern =
            SequencePattern::from_u8(sequence_pattern.load(Ordering::Relaxed) as u8);

        startup::mark(Stage::StreamBuildRequested);
        let stream = output
            .device
            .build_output_stream(
                &output.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Audio thread: no allocation, no locks.
                    if trace_callback {
                        startup::mark(Stage::FirstAudioCallback);
                        trace_callback = false;
                    }
                    let command = sequence_flag.load(Ordering::Acquire);
                    let pattern = SequencePattern::from_u8(command as u8);
                    let restart = command >> 8;
                    if restart != seen_restart {
                        sequencer.restart();
                        capture.reset();
                        seen_restart = restart;
                    }
                    if pattern != captured_pattern {
                        capture.reset();
                        captured_pattern = pattern;
                    }
                    let velocity = SequenceVelocity::from_u8(velocity_flag.load(Ordering::Relaxed));
                    let modulation =
                        SequenceModulation::from_u8(modulation_flag.load(Ordering::Relaxed));
                    sequencer.set_modulation(modulation);
                    let fading_out = fade_out_flag.load(Ordering::Acquire);
                    let block = MAX_BLOCK_FRAMES as usize * channels;
                    for chunk in data.chunks_mut(block) {
                        ump.clear();
                        if let Some(notes) = preview_flag.poll(&mut seen_preview) {
                            sequencer.preview(notes);
                        }
                        let events = if let Some(selected) = &mut selected {
                            selected.render(chunk.len() / channels);
                            selected.words.as_slice()
                        } else {
                            sequencer.render(pattern, velocity, chunk.len() / channels, &mut ump);
                            ump.as_slice()
                        };
                        let rendered = if let Some(source) = &mut preview_render {
                            source.process(events, chunk, channels)
                                && render.process_input(chunk, channels)
                        } else {
                            render.process(events, chunk, channels)
                        };
                        if !rendered {
                            if let Some(failed) = &selection_failure {
                                failed.store(true, Ordering::Release);
                            }
                        }
                        fade.apply(chunk, channels, fading_out);
                        if let Some(source) = delivered_cc1_source(events, modulation, rendered) {
                            cc1_source_flag.store(source, Ordering::Release);
                        }
                        let (velocity, modulation) = sequencer.current_values();
                        velocity_meter.store(velocity, Ordering::Relaxed);
                        modulation_meter.store(modulation, Ordering::Relaxed);
                        capture.record(chunk, channels, events);
                        if trace_signal && chunk.iter().any(|sample| sample.abs() > 0.000001) {
                            startup::mark(Stage::FirstSignalRendered);
                            trace_signal = false;
                        }
                    }
                    if fade.finish_callback(fading_out) {
                        faded_flag.store(true, Ordering::Release);
                    }
                    if first_render {
                        rendered_flag.store(true, Ordering::Release);
                        first_render = false;
                    }
                },
                |err| eprintln!("audio stream error: {err}"),
                None,
            )
            .map_err(|e| format!("cannot open the audio stream: {e}"))?;
        startup::mark(Stage::StreamBuilt);
        stream
            .play()
            .map_err(|e| format!("cannot start the audio stream: {e}"))?;
        startup::mark(Stage::PlayReturned);

        Ok(Self {
            _stream: Some(stream),
            _processor: processor,
            _preview_chain: preview_chain,
            sequence_pattern,
            sequence_velocity,
            sequence_modulation,
            current_velocity,
            current_modulation,
            cc1_source,
            rendered,
            scope,
            preview,
            fade_out,
            faded,
        })
    }

    /// Ramps the output to silence and returns once the device has played it,
    /// or after a timeout when callbacks stall. The stream keeps running silent.
    pub(crate) fn fade_out(&self) {
        self.fade_out.store(true, Ordering::Release);
        let deadline = std::time::Instant::now()
            + crate::audio_fade::FADE
            + std::time::Duration::from_millis(250);
        while !self.faded.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    pub(crate) fn preview(&self, notes: Option<crate::preview::Notes>) {
        self.preview.publish(notes);
    }

    pub fn sequence_pattern(&self) -> SequencePattern {
        SequencePattern::from_u8(self.sequence_pattern.load(Ordering::Acquire) as u8)
    }

    /// Join the callback before capturing the origin of its last delivered CC1.
    pub(crate) fn stop_and_capture_sweep(&mut self) -> Option<bool> {
        self._stream = None;
        match self.cc1_source.load(Ordering::Acquire) {
            1 => Some(false),
            2 => Some(true),
            _ => None,
        }
    }

    pub fn current_values(&self) -> (u8, u8) {
        (
            self.current_velocity.load(Ordering::Relaxed),
            self.current_modulation.load(Ordering::Relaxed),
        )
    }

    pub fn scope(&self) -> &crate::scope::Scope {
        &self.scope
    }

    pub fn has_rendered(&self) -> bool {
        self.rendered.load(Ordering::Acquire)
    }

    /// Changes pattern at the next audio block, releasing all sounding sequence
    /// notes and restarting the selected phrase from its first note.
    pub fn set_sequence(&self, pattern: SequencePattern) {
        self.sequence_pattern
            .try_update(Ordering::Release, Ordering::Relaxed, |command| {
                Some((command.wrapping_add(256) & !0xff) | pattern as u64)
            })
            .unwrap();
    }

    pub fn set_modulation(&self, modulation: SequenceModulation) {
        self.sequence_modulation
            .store(modulation as u8, Ordering::Relaxed);
    }

    /// Applies to subsequent note ons without restarting the phrase.
    pub fn set_velocity(&self, velocity: SequenceVelocity) {
        self.sequence_velocity
            .store(velocity as u8, Ordering::Relaxed);
    }
}

impl Drop for Voice {
    fn drop(&mut self) {
        // Wait for the callback before using the processor on this thread.
        self._stream = None;
        // Release every MIDI note before rebuilding a stream. The new sequencer
        // cannot otherwise know which notes the previous callback left sounding.
        let events = shutdown_events();
        let mut silence = [0.0; 2];
        let mut render = unsafe { self._processor.audio_ref() };
        for batch in events.chunks(1024) {
            render.process(batch, &mut silence, 2);
        }
    }
}

fn shutdown_events() -> [u32; 2064] {
    // Native-only CLAP ports cannot carry MIDI CC 120. Explicit note offs
    // cover sequence notes on channel 0 after losing tracking.
    std::array::from_fn(|index| {
        if index < 2048 {
            note_off((index / 128) as u8, (index % 128) as u8)
        } else {
            0x20B0_0000 | (((index - 2048) as u32) << 16) | (120 << 8)
        }
    })
}

fn delivered_cc1_source(
    events: &[u32],
    modulation: SequenceModulation,
    rendered: bool,
) -> Option<u8> {
    (rendered
        && events
            .iter()
            .any(|event| event & 0xffff_ff00 == 0x20b0_0100))
    .then_some(if modulation == SequenceModulation::Sweep {
        2
    } else {
        1
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_successfully_delivered_cc1_changes_automation_origin() {
        let events = [
            0x0020_0001,
            0x2090_3064,
            crate::sequence_modulation::cc1(64),
        ];
        assert_eq!(
            delivered_cc1_source(&events, SequenceModulation::Sweep, true),
            Some(2)
        );
        assert_eq!(
            delivered_cc1_source(&events, SequenceModulation::Full, true),
            Some(1)
        );
        assert_eq!(
            delivered_cc1_source(&events, SequenceModulation::Zero, true),
            Some(1)
        );
        assert_eq!(
            delivered_cc1_source(&events, SequenceModulation::Sweep, false),
            None
        );
        assert_eq!(
            delivered_cc1_source(&events[..2], SequenceModulation::Sweep, true),
            None
        );
        assert_eq!(
            delivered_cc1_source(&[0x20b0_027f], SequenceModulation::Sweep, true),
            None
        );
    }

    #[test]
    fn rebuilding_stream_releases_notes_without_requiring_cc_support() {
        let events = shutdown_events();
        for note in crate::seq::GUITAR_NOTES.into_iter().chain([60]) {
            assert!(events[..128].contains(&note_off(0, note)));
        }
        assert_eq!(events[2048], 0x20B0_7800);
        assert_eq!(events[2063], 0x20BF_7800);
    }
}
