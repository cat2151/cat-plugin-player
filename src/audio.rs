//! Audio output for one plugin instance: a cpal stream whose callback pulls
//! audio from the shim, plus a lock-free queue for sending it UMP events and
//! the step sequencer that plays the hardcoded phrase.

use crate::ffi;
use crate::seq::{EventBuf, Sequencer};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub use crate::seq::{note_off, note_on};

/// The largest block handed to the plugin at once. The device may ask for
/// more frames per callback; those are rendered in several blocks.
pub const MAX_BLOCK_FRAMES: u32 = 1024;

/// How many UMP words can wait between two audio callbacks.
const EVENT_QUEUE_WORDS: usize = 1024;

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
    _stream: cpal::Stream,
    _processor: ffi::Processor,
    events: rtrb::Producer<u32>,
    sequence_on: Arc<AtomicBool>,
}

impl Voice {
    /// `sequence_on`: whether the phrase starts playing right away.
    pub fn start(
        output: &Output,
        processor: ffi::Processor,
        sequence_on: bool,
    ) -> Result<Self, String> {
        let channels = output.config.channels as usize;
        let sequence_on = Arc::new(AtomicBool::new(sequence_on));
        let sequence_flag = Arc::clone(&sequence_on);
        let mut sequencer = Sequencer::new(output.sample_rate());
        let mut ump = EventBuf::new();
        let (events, mut incoming) = rtrb::RingBuffer::<u32>::new(EVENT_QUEUE_WORDS);
        // SAFETY: `_stream` is declared before `_processor`, so the callback is
        // gone before the processor is; only this one callback renders.
        let mut render = unsafe { processor.audio_ref() };

        let stream = output
            .device
            .build_output_stream(
                &output.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Audio thread: no allocation, no locks.
                    let enabled = sequence_flag.load(Ordering::Relaxed);
                    let block = MAX_BLOCK_FRAMES as usize * channels;
                    let mut first = true;
                    for chunk in data.chunks_mut(block) {
                        ump.clear();
                        if first {
                            // What the UI sent goes to the start of the first block.
                            first = false;
                            while let Ok(word) = incoming.pop() {
                                if !ump.push(word) {
                                    break;
                                }
                            }
                        }
                        sequencer.render(enabled, chunk.len() / channels, &mut ump);
                        render.process(ump.as_slice(), chunk, channels);
                    }
                },
                |err| eprintln!("audio stream error: {err}"),
                None,
            )
            .map_err(|e| format!("cannot open the audio stream: {e}"))?;
        stream
            .play()
            .map_err(|e| format!("cannot start the audio stream: {e}"))?;

        Ok(Self {
            _stream: stream,
            _processor: processor,
            events,
            sequence_on,
        })
    }

    pub fn sequence_on(&self) -> bool {
        self.sequence_on.load(Ordering::Relaxed)
    }

    /// Starts or stops the phrase. Takes effect at the next audio block; stopping
    /// releases the sounding note, and starting again begins with the first note.
    pub fn set_sequence(&self, on: bool) {
        self.sequence_on.store(on, Ordering::Relaxed);
    }

    /// Queues one UMP word for the next audio block. Returns false if the queue is full.
    pub fn send(&mut self, word: u32) -> bool {
        self.events.push(word).is_ok()
    }
}
