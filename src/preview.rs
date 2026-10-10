//! Fixed-size latest-request mailbox and one-shot playback. No callback allocations.
use crate::seq::{jr_timestamp, note_off, note_on, EventBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Notes {
    // High bits: gate in samples; low byte: velocity (zero means absent).
    pub pitches: [u64; 128],
}
impl Notes {
    pub const MAX_GATE: u64 = u64::MAX >> 8;
}
impl Default for Notes {
    fn default() -> Self {
        Self { pitches: [0; 128] }
    }
}

pub(crate) struct Mailbox {
    generation: AtomicU64,
    pitches: [AtomicU64; 128],
}
impl Default for Mailbox {
    fn default() -> Self {
        Self {
            generation: AtomicU64::new(0),
            pitches: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }
}
impl Mailbox {
    /// Single UI writer. SeqCst prevents an accepted snapshot spanning two requests.
    pub fn publish(&self, notes: Option<Notes>) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        for (destination, value) in self.pitches.iter().zip(notes.unwrap_or_default().pitches) {
            destination.store(value, Ordering::SeqCst);
        }
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
    /// A concurrent publication is deferred to the next block, never spun on.
    pub fn poll(&self, seen: &mut u64) -> Option<Notes> {
        let generation = self.generation.load(Ordering::SeqCst);
        if generation == *seen || generation & 1 != 0 {
            return None;
        }
        let notes = Notes {
            pitches: std::array::from_fn(|i| self.pitches[i].load(Ordering::SeqCst)),
        };
        if self.generation.load(Ordering::SeqCst) != generation {
            return None;
        }
        *seen = generation;
        Some(notes)
    }
}

#[derive(Default)]
pub(crate) struct Player {
    pending: Option<Notes>,
    notes: Notes,
    pub velocity: u8,
}
impl Player {
    pub fn replace(&mut self, notes: Notes) {
        self.pending = Some(notes);
    }
    pub fn active(&self) -> bool {
        self.pending.is_some() || self.notes.pitches.iter().any(|&n| n != 0)
    }
    pub fn render(&mut self, frames: usize, out: &mut EventBuf) {
        if let Some(notes) = self.pending {
            for (pitch, value) in self.notes.pitches.iter_mut().enumerate() {
                if *value != 0 {
                    if !out.push(note_off(0, pitch as u8)) {
                        return;
                    }
                    *value = 0;
                }
            }
            self.velocity = 0;
            let count = notes.pitches.iter().filter(|&&n| n != 0).count();
            // Room for all ons and a subsequent timestamp; defer if old releases filled the block.
            if out.remaining() < count + 1 {
                return;
            }
            self.notes = notes;
            self.pending = None;
            for (pitch, &value) in self.notes.pitches.iter().enumerate() {
                if value != 0 {
                    self.velocity = value as u8;
                    out.push(note_on(0, pitch as u8, value as u8));
                }
            }
        }
        let mut cursor = 0;
        loop {
            let next = self
                .notes
                .pitches
                .iter()
                .filter(|&&n| n != 0)
                .map(|n| (n >> 8) as usize)
                .min();
            let Some(at) = next.filter(|&at| at < frames) else {
                break;
            };
            let count = self
                .notes
                .pitches
                .iter()
                .filter(|&&n| n != 0 && (n >> 8) as usize == at)
                .count();
            if out.remaining() < count + usize::from(at > cursor) {
                break;
            }
            if at > cursor {
                out.push(jr_timestamp(at - cursor));
                cursor = at;
            }
            for (pitch, value) in self.notes.pitches.iter_mut().enumerate() {
                if *value != 0 && (*value >> 8) as usize == at {
                    out.push(note_off(0, pitch as u8));
                    *value = 0;
                }
            }
        }
        for value in &mut self.notes.pitches {
            if *value != 0 {
                *value = ((*value >> 8).saturating_sub(frames as u64) << 8) | (*value & 0xff);
            }
        }
        if !self.notes.pitches.iter().any(|&n| n != 0) {
            self.velocity = 0;
        }
    }
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
