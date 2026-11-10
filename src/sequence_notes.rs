//! Read-only note feedback from the MIDI events actually delivered to the instrument.
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub(crate) struct Meter([AtomicU64; 2]);

impl Meter {
    pub(crate) fn snapshot(&self) -> [u64; 2] {
        self.0.each_ref().map(|word| word.load(Ordering::Relaxed))
    }
}

#[derive(Default)]
pub(crate) struct Tracker([u64; 2]);

impl Tracker {
    /// The main sequence and chord/MML preview use MIDI 1.0 on channel zero.
    /// Timestamps and controller messages must not be interpreted as notes.
    pub(crate) fn record(&mut self, events: &[u32], meter: &Meter) {
        for &word in events {
            if word & 0xffff_0000 != 0x2090_0000 && word & 0xffff_0000 != 0x2080_0000 {
                continue;
            }
            let note = ((word >> 8) & 127) as usize;
            let bit = 1 << (note % 64);
            if word & 0x00f0_0000 == 0x0090_0000 && word & 127 != 0 {
                self.0[note / 64] |= bit;
            } else {
                self.0[note / 64] &= !bit;
            }
        }
        for (destination, value) in meter.0.iter().zip(self.0) {
            destination.store(value, Ordering::Relaxed);
        }
    }
}

pub(crate) fn contains(notes: [u64; 2], note: u8) -> bool {
    notes[usize::from(note) / 64] & (1 << (note % 64)) != 0
}

#[cfg(test)]
mod tests;
