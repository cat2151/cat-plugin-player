//! Immutable UI-prepared phrase and sample-timed loop playback.
use crate::seq::{jr_timestamp, note_off, note_on, EventBuf};
use crate::sequence_modulation::{cc1, Modulation};
use crate::sequence_velocity::SequenceVelocity;
use std::sync::Arc;

#[cfg(test)]
#[path = "timed_sequence_tests.rs"]
mod tests;

#[derive(Clone, Debug)]
pub struct Phrase {
    pub events: Vec<(u64, [u8; 3])>,
    pub duration: u64,
    pub notes: usize,
}

impl Phrase {
    /// Conversion and allocation happen before building the audio callback.
    pub fn parse(input: &str, rate: u32) -> Result<Self, String> {
        let performance = cmrt_chord::timed_performance(input)?;
        let duration = (performance.duration_seconds * f64::from(rate)).round() as u64;
        if duration == 0 {
            return Err("Phrase is shorter than one sample".into());
        }
        let events: Vec<_> = performance
            .events
            .into_iter()
            .map(|e| ((e.seconds * f64::from(rate)).round() as u64, e.message))
            .collect();
        let notes = events.iter().filter(|(_, m)| m[0] == 0x90).count();
        Ok(Self {
            events,
            duration,
            notes,
        })
    }
}

pub struct TimedPlayer {
    phrase: Option<Arc<Phrase>>,
    position: u64,
    index: usize,
    note_index: usize,
    down: bool,
    sounding: [bool; 128],
    faulted: bool,
    pub velocity: u8,
    pub modulation: u8,
}

impl Default for TimedPlayer {
    fn default() -> Self {
        Self {
            phrase: None,
            position: 0,
            index: 0,
            note_index: 0,
            down: false,
            sounding: [false; 128],
            faulted: false,
            velocity: 0,
            modulation: 0,
        }
    }
}

impl TimedPlayer {
    pub fn set_phrase(&mut self, phrase: Option<Arc<Phrase>>) {
        self.phrase = phrase;
    }

    fn release(&mut self, out: &mut EventBuf) -> bool {
        for (note, active) in self.sounding.iter_mut().enumerate() {
            if *active {
                if !out.push(note_off(0, note as u8)) {
                    return false;
                }
                *active = false;
            }
        }
        true
    }

    pub fn reset(&mut self, out: &mut EventBuf) -> bool {
        if !self.release(out) {
            return false;
        }
        self.position = 0;
        self.index = 0;
        self.note_index = 0;
        self.down = false;
        self.faulted = false;
        self.velocity = 0;
        true
    }

    pub fn render(
        &mut self,
        velocity: SequenceVelocity,
        rate: u32,
        frames: usize,
        out: &mut EventBuf,
        modulation: &mut Modulation,
    ) {
        if self.faulted {
            self.release(out);
            self.velocity = 0;
            return;
        }
        let Some(phrase) = self.phrase.as_ref() else {
            return;
        };
        let mut cursor = 0;
        let mut elapsed = 0;
        while elapsed < frames {
            let event_at = phrase
                .events
                .get(self.index)
                .map_or(phrase.duration, |e| e.0);
            let note_at = elapsed + event_at.saturating_sub(self.position) as usize;
            let cc_at = elapsed.saturating_add(modulation.next(rate, true).unwrap_or(usize::MAX));
            let at = note_at.min(cc_at);
            if at >= frames {
                break;
            }
            // Keep enough room to release every sounding pitch on overload.
            if out.remaining() < 130 {
                self.faulted = true;
                break;
            }
            if at > cursor {
                out.push(jr_timestamp(at - cursor));
                cursor = at;
            }
            let advance = at - elapsed;
            self.position += advance as u64;
            modulation.advance(advance, true);
            elapsed = at;
            if cc_at == at {
                out.push(cc1(modulation.value()));
                self.modulation = modulation.value();
                modulation.sent();
            }
            if note_at != at {
                continue;
            }
            if let Some(&(_, message)) = phrase.events.get(self.index) {
                let note = message[1];
                if message[0] == 0x90 {
                    let value = velocity.value(self.note_index, phrase.notes.max(2), self.down);
                    out.push(note_on(0, note, value));
                    self.sounding[note as usize] = true;
                    self.velocity = value;
                    self.note_index += 1;
                } else {
                    out.push(note_off(0, note));
                    self.sounding[note as usize] = false;
                }
                self.index += 1;
            } else {
                // All final note offs at duration are emitted before looping.
                for (note, active) in self.sounding.iter_mut().enumerate() {
                    if *active {
                        out.push(note_off(0, note as u8));
                        *active = false;
                    }
                }
                self.index = 0;
                self.note_index = 0;
                self.position = 0;
                self.down = !self.down;
            }
        }
        if self.faulted {
            self.release(out);
            self.velocity = 0;
        } else {
            let advance = frames - elapsed;
            self.position += advance as u64;
            modulation.advance(advance, true);
        }
    }
}
