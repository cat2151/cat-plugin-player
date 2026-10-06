//! Playback patterns timed on the audio thread.
//!
//! It counts samples, so its timing does not depend on the UI or on how the
//! audio device slices its callbacks. No allocation, no locks.

/// The phrase: C3, G3, then the same an octave up.
pub const NOTES: [u8; 4] = [48, 55, 60, 67];
pub const STEP_MILLIS: u32 = 250;
#[cfg(test)]
const VELOCITY: u8 = 100;
const CHANNEL: u8 = 0;
use crate::sequence_modulation::{cc1, Modulation, SequenceModulation};
use crate::sequence_pattern::SequencePattern;
use crate::sequence_velocity::SequenceVelocity;

/// Standard guitar tuning, from the sixth string to the first (E2 to E4).
pub const GUITAR_NOTES: [u8; 6] = [40, 45, 50, 55, 59, 64];
/// Gradually accelerate the picking while keeping the phrase's total duration.
const GUITAR_INTERVAL_MILLIS: [u32; 5] = [175, 150, 125, 100, 75];

/// MIDI 1.0 channel voice messages as UMP (group 0).
pub fn note_on(channel: u8, note: u8, velocity: u8) -> u32 {
    0x2090_0000
        | ((channel as u32 & 0x0F) << 16)
        | ((note as u32 & 0x7F) << 8)
        | (velocity as u32 & 0x7F)
}

pub fn note_off(channel: u8, note: u8) -> u32 {
    0x2080_0000 | ((channel as u32 & 0x0F) << 16) | ((note as u32 & 0x7F) << 8)
}

/// JR Timestamp UMP. remidy reads it as "the following events happen this many
/// samples after the previous ones" within the block being rendered.
fn jr_timestamp(delta_samples: usize) -> u32 {
    0x0020_0000 | (delta_samples as u32 & 0xFFFF)
}

/// The UMP words for one audio block. Fixed size; words that do not fit are dropped.
pub struct EventBuf {
    words: [u32; 256],
    len: usize,
}

impl EventBuf {
    pub fn new() -> Self {
        Self {
            words: [0; 256],
            len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn push(&mut self, word: u32) -> bool {
        if self.len < self.words.len() {
            self.words[self.len] = word;
            self.len += 1;
            true
        } else {
            false
        }
    }

    pub fn as_slice(&self) -> &[u32] {
        &self.words[..self.len]
    }

    #[cfg(test)]
    pub fn remaining(&self) -> usize {
        self.words.len() - self.len
    }
}

pub struct Sequencer {
    sample_rate: u32,
    sample_remainder: u64,
    /// Samples from the start of the next block to the next step.
    until_next_step: usize,
    step: usize,
    sounding: [bool; 128],
    pattern: SequencePattern,
    velocity_down: bool,
    current_velocity: u8,
    current_modulation: u8,
    modulation: Modulation,
}

impl Sequencer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            sample_remainder: 0,
            until_next_step: 0,
            step: 0,
            sounding: [false; 128],
            pattern: SequencePattern::Off,
            modulation: Modulation::default(),
            velocity_down: false,
            current_velocity: 0,
            current_modulation: 0,
        }
    }

    pub fn set_modulation(&mut self, modulation: SequenceModulation) {
        self.modulation.select(modulation);
    }

    pub fn current_values(&self) -> (u8, u8) {
        (self.current_velocity, self.current_modulation)
    }

    fn samples(&mut self, micros: u32) -> usize {
        let scaled = self.sample_rate as u64 * micros as u64 + self.sample_remainder;
        self.sample_remainder = scaled % 1_000_000;
        (scaled / 1_000_000).max(1) as usize
    }

    fn release(&mut self, out: &mut EventBuf) {
        for (note, sounding) in self.sounding.iter_mut().enumerate() {
            if *sounding && out.push(note_off(CHANNEL, note as u8)) {
                *sounding = false;
            }
        }
    }

    /// Pick overlapping notes, hold for two seconds, then rest before the next chord.
    /// All chords in a pattern have the same number of notes.
    fn arpeggio(
        &mut self,
        out: &mut EventBuf,
        chords: &[&[u8]],
        interval_divisor: u32,
    ) -> (Option<u8>, u32) {
        let stride = chords[0].len() + 1;
        let chord = chords[self.step / stride];
        let index = self.step % stride;
        self.step = (self.step + 1) % (stride * chords.len());
        if index == chord.len() {
            self.release(out);
            (None, 500_000)
        } else {
            let delay = if index + 1 == chord.len() {
                2_000_000
            } else {
                GUITAR_INTERVAL_MILLIS[index] * 1000 / interval_divisor
            };
            (Some(chord[index]), delay)
        }
    }

    /// Appends this block's events to `out`. Call exactly once per rendered block,
    /// with that block's length; `frames` must be at most 65535.
    ///
    /// Events already in `out` are taken to be at sample 0 of the block.
    pub fn render(
        &mut self,
        pattern: SequencePattern,
        velocity: SequenceVelocity,
        frames: usize,
        out: &mut EventBuf,
    ) {
        if pattern != self.pattern {
            self.release(out);
            if self.pattern == SequencePattern::Off {
                self.modulation.restart();
            }
            self.pattern = pattern;
            self.until_next_step = 0;
            self.step = 0;
            self.velocity_down = false;
            self.sample_remainder = 0;
        }
        if pattern == SequencePattern::Off {
            // A full buffer must not turn a dropped note off into a forgotten note.
            self.release(out);
            self.current_velocity = 0;
            if frames > 0
                && self.modulation.next(self.sample_rate, false).is_some()
                && out.push(cc1(self.modulation.value()))
            {
                self.current_modulation = self.modulation.value();
                self.modulation.sent();
            }
            return;
        }

        let mut at = self.until_next_step; // sample offset of the next step in this block
        let mut cursor = 0; // offset the events emitted so far are at
        loop {
            let modulation_at = self
                .modulation
                .next(self.sample_rate, true)
                .unwrap_or(usize::MAX);
            let next = at.min(modulation_at);
            if next >= frames {
                break;
            }
            if next > cursor {
                if !out.push(jr_timestamp(next - cursor)) {
                    break;
                }
                cursor = next;
            }
            if modulation_at == next {
                if !out.push(cc1(self.modulation.value())) {
                    break;
                }
                self.current_modulation = self.modulation.value();
                self.modulation.sent();
            }
            if at != next {
                continue;
            }
            let (index, count) = match pattern {
                SequencePattern::Steps => (self.step, NOTES.len()),
                SequencePattern::GuitarArpeggio => (self.step, GUITAR_NOTES.len()),
                SequencePattern::Csus4CArpeggio => (self.step / 4 * 3 + self.step % 4, 6),
                SequencePattern::Fmaj7G6Arpeggio => (self.step / 5 * 4 + self.step % 5, 8),
                SequencePattern::Off => unreachable!(),
            };
            let (note, delay) = match pattern {
                SequencePattern::Steps => {
                    self.release(out);
                    let note = NOTES[self.step];
                    self.step = (self.step + 1) % NOTES.len();
                    (Some(note), STEP_MILLIS * 1000)
                }
                SequencePattern::GuitarArpeggio => self.arpeggio(out, &[&GUITAR_NOTES], 1),
                SequencePattern::Csus4CArpeggio => {
                    self.arpeggio(out, &[&[60, 65, 67], &[60, 64, 67]], 2)
                }
                SequencePattern::Fmaj7G6Arpeggio => {
                    self.arpeggio(out, &[&[53, 69, 72, 76], &[55, 71, 74, 76]], 2)
                }
                SequencePattern::Off => unreachable!(),
            };
            if let Some(note) = note {
                let velocity = velocity.value(index, count, self.velocity_down);
                if out.push(note_on(CHANNEL, note, velocity)) {
                    self.sounding[note as usize] = true;
                    self.current_velocity = velocity;
                }
            }
            if self.step == 0 {
                self.velocity_down = !self.velocity_down;
            }
            at += self.samples(delay);
        }
        self.until_next_step = at.saturating_sub(frames);
        self.modulation.advance(frames, true);
    }
}

#[cfg(test)]
#[path = "seq_tests.rs"]
mod tests;
