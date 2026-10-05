//! A hardcoded step sequencer that runs on the audio thread.
//!
//! It counts samples, so its timing does not depend on the UI or on how the
//! audio device slices its callbacks. No allocation, no locks.

/// The phrase: C3, G3, then the same an octave up.
pub const NOTES: [u8; 4] = [48, 55, 60, 67];
pub const STEP_MILLIS: u32 = 250;
const VELOCITY: u8 = 100;
const CHANNEL: u8 = 0;

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
}

pub struct Sequencer {
    step_samples: usize,
    /// Samples from the start of the next block to the next step.
    until_next_step: usize,
    step: usize,
    sounding: Option<u8>,
    running: bool,
}

impl Sequencer {
    pub fn new(sample_rate: u32) -> Self {
        let step_samples = (sample_rate as u64 * STEP_MILLIS as u64 / 1000).max(1) as usize;
        Self {
            step_samples,
            until_next_step: 0,
            step: 0,
            sounding: None,
            running: false,
        }
    }

    /// Appends this block's events to `out`. Call exactly once per rendered block,
    /// with that block's length; `frames` must be at most 65535.
    ///
    /// Events already in `out` are taken to be at sample 0 of the block.
    pub fn render(&mut self, enabled: bool, frames: usize, out: &mut EventBuf) {
        if !enabled {
            if self.running {
                // Switched off: release what is sounding; start from the top next time.
                if let Some(note) = self.sounding.take() {
                    out.push(note_off(CHANNEL, note));
                }
                self.running = false;
            }
            return;
        }
        if !self.running {
            self.running = true;
            self.until_next_step = 0;
            self.step = 0;
        }

        let mut at = self.until_next_step; // sample offset of the next step in this block
        let mut cursor = 0; // offset the events emitted so far are at
        while at < frames {
            if at > cursor {
                out.push(jr_timestamp(at - cursor));
                cursor = at;
            }
            if let Some(note) = self.sounding.take() {
                out.push(note_off(CHANNEL, note));
            }
            let note = NOTES[self.step];
            out.push(note_on(CHANNEL, note, VELOCITY));
            self.sounding = Some(note);
            self.step = (self.step + 1) % NOTES.len();
            at += self.step_samples;
        }
        self.until_next_step = at - frames;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders `total` samples in blocks of `block` and returns (absolute sample, word)
    /// for every note on/off.
    fn run(
        seq: &mut Sequencer,
        enabled: bool,
        total: usize,
        block: usize,
        start: usize,
    ) -> Vec<(usize, u32)> {
        let mut events = Vec::new();
        let mut done = 0;
        while done < total {
            let frames = block.min(total - done);
            let mut buf = EventBuf::new();
            seq.render(enabled, frames, &mut buf);
            let mut offset = 0;
            for &word in buf.as_slice() {
                if word >> 16 == 0x0020 {
                    offset += (word & 0xFFFF) as usize;
                } else {
                    assert!(offset < frames, "event outside its block");
                    events.push((start + done + offset, word));
                }
            }
            done += frames;
        }
        events
    }

    fn note_ons(events: &[(usize, u32)]) -> Vec<(usize, u8)> {
        events
            .iter()
            .filter(|(_, w)| w & 0xFFF0_0000 == 0x2090_0000)
            .map(|&(t, w)| (t, ((w >> 8) & 0x7F) as u8))
            .collect()
    }

    #[test]
    fn steps_land_on_exact_samples_whatever_the_block_size() {
        for block in [64, 480, 1024, 1000, 12000, 30000] {
            let mut seq = Sequencer::new(48_000);
            let events = run(&mut seq, true, 96_000, block, 0);
            let ons = note_ons(&events);
            let expected: Vec<(usize, u8)> = (0..8).map(|i| (i * 12_000, NOTES[i % 4])).collect();
            assert_eq!(ons, expected, "block size {block}");
        }
    }

    #[test]
    fn every_note_is_released_when_the_next_one_starts() {
        let mut seq = Sequencer::new(44_100);
        let events = run(&mut seq, true, 44_100, 512, 0);
        let mut sounding: Option<u8> = None;
        for &(_, w) in &events {
            let note = ((w >> 8) & 0x7F) as u8;
            if w & 0xFFF0_0000 == 0x2080_0000 {
                assert_eq!(sounding.take(), Some(note));
            } else {
                assert_eq!(sounding, None, "note on while another note sounds");
                sounding = Some(note);
            }
        }
    }

    #[test]
    fn switching_off_releases_the_note_and_restarts_from_the_top() {
        let mut seq = Sequencer::new(48_000);
        let first = run(&mut seq, true, 13_000, 1024, 0); // steps at 0 and 12000
        assert_eq!(note_ons(&first), vec![(0, NOTES[0]), (12_000, NOTES[1])]);

        let off = run(&mut seq, false, 2048, 1024, 13_000);
        assert_eq!(off, vec![(13_000, note_off(0, NOTES[1]))]);

        let again = run(&mut seq, true, 1024, 1024, 15_048);
        assert_eq!(note_ons(&again), vec![(15_048, NOTES[0])]);
    }
}
