//! Bounded single-producer/single-consumer transport. All storage is allocated
//! before playback; the audio callback only performs atomic loads and stores.
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;

const CAPACITY: usize = 16_384;

#[derive(Clone, Copy)]
pub struct Sample {
    pub stereo: [f32; 2],
    pub note: u8,
    pub epoch: u64,
    pub serial: u64,
}

struct Slot {
    audio: AtomicU64,
    note: AtomicU8,
    epoch: AtomicU64,
    serial: AtomicU64,
}

pub struct Queue {
    slots: Box<[Slot]>,
    read: AtomicUsize,
    write: AtomicUsize,
}

impl Queue {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            slots: (0..CAPACITY)
                .map(|_| Slot {
                    audio: AtomicU64::new(0),
                    note: AtomicU8::new(128),
                    epoch: AtomicU64::new(0),
                    serial: AtomicU64::new(0),
                })
                .collect(),
            read: AtomicUsize::new(0),
            write: AtomicUsize::new(0),
        })
    }

    // Called only by Capture on the audio thread.
    fn push(&self, sample: Sample) {
        let write = self.write.load(Ordering::Relaxed);
        if write.wrapping_sub(self.read.load(Ordering::Acquire)) >= CAPACITY {
            return;
        }
        let slot = &self.slots[write % CAPACITY];
        slot.audio.store(
            u64::from(sample.stereo[0].to_bits()) | (u64::from(sample.stereo[1].to_bits()) << 32),
            Ordering::Relaxed,
        );
        slot.note.store(sample.note, Ordering::Relaxed);
        slot.epoch.store(sample.epoch, Ordering::Relaxed);
        slot.serial.store(sample.serial, Ordering::Relaxed);
        self.write.store(write.wrapping_add(1), Ordering::Release);
    }

    // Called only by the scope worker. Release prevents slot reuse until read.
    pub fn pop(&self) -> Option<Sample> {
        let read = self.read.load(Ordering::Relaxed);
        if read == self.write.load(Ordering::Acquire) {
            return None;
        }
        let slot = &self.slots[read % CAPACITY];
        let audio = slot.audio.load(Ordering::Relaxed);
        let sample = Sample {
            stereo: [
                f32::from_bits(audio as u32),
                f32::from_bits((audio >> 32) as u32),
            ],
            note: slot.note.load(Ordering::Relaxed),
            epoch: slot.epoch.load(Ordering::Relaxed),
            serial: slot.serial.load(Ordering::Relaxed),
        };
        self.read.store(read.wrapping_add(1), Ordering::Release);
        Some(sample)
    }
}

pub struct Capture {
    queue: Arc<Queue>,
    note: u8,
    epoch: u64,
    serial: u64,
}

impl Capture {
    pub fn new(queue: Arc<Queue>) -> Self {
        Self {
            queue,
            note: 128,
            epoch: 0,
            serial: 0,
        }
    }

    pub fn reset(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// UMP JR timestamps are sample deltas, matching the shim's event timing.
    pub fn record(&mut self, audio: &[f32], channels: usize, events: &[u32]) {
        let mut event = 0;
        let mut at = 0;
        for (frame, values) in audio.chunks_exact(channels).enumerate() {
            while event < events.len() {
                let word = events[event];
                if word & 0xFFFF_0000 == 0x0020_0000 {
                    at += (word & 0xFFFF) as usize;
                    event += 1;
                    continue;
                }
                if at > frame {
                    break;
                }
                if word & 0xFFF0_0000 == 0x2090_0000 && word & 0x7F != 0 {
                    self.note = ((word >> 8) & 0x7F) as u8;
                    self.reset();
                }
                event += 1;
            }
            let clean = |v: f32| if v.is_finite() { v } else { 0.0 };
            self.queue.push(Sample {
                stereo: [
                    clean(values[0]),
                    clean(values.get(1).copied().unwrap_or(values[0])),
                ],
                note: self.note,
                epoch: self.epoch,
                serial: self.serial,
            });
            self.serial = self.serial.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn note_changes_at_timestamp_and_note_off_retains_tail_reference() {
        let queue = Queue::new();
        let mut capture = Capture::new(Arc::clone(&queue));
        capture.record(
            &[0.1, 0.2, 0.3, 0.4],
            1,
            &[
                crate::seq::note_on(0, 48, 100),
                0x0020_0002,
                crate::seq::note_on(0, 60, 100),
            ],
        );
        let first = queue.pop().unwrap();
        assert_eq!(first.stereo, [0.1, 0.1]);
        assert_eq!(first.note, 48);
        assert_eq!(queue.pop().unwrap().note, 48);
        let third = queue.pop().unwrap();
        assert_eq!(third.note, 60);
        assert_ne!(first.epoch, third.epoch);
        queue.pop();
        capture.record(&[0.5], 1, &[crate::seq::note_off(0, 60)]);
        assert_eq!(queue.pop().unwrap().note, 60);
    }

    #[test]
    fn overflow_is_detectable_instead_of_joining_discontinuous_audio() {
        let queue = Queue::new();
        let mut capture = Capture::new(Arc::clone(&queue));
        capture.record(&vec![1.0; CAPACITY + 10], 1, &[]);
        for expected in 0..CAPACITY {
            assert_eq!(queue.pop().unwrap().serial, expected as u64);
        }
        capture.record(&[1.0], 1, &[]);
        assert_eq!(queue.pop().unwrap().serial, (CAPACITY + 10) as u64);
    }

    #[test]
    fn concurrent_transport_keeps_stereo_and_metadata_together_across_wraps() {
        use std::sync::atomic::AtomicBool;
        let queue = Queue::new();
        let finished = Arc::new(AtomicBool::new(false));
        let producer_queue = Arc::clone(&queue);
        let producer_finished = Arc::clone(&finished);
        let producer = std::thread::spawn(move || {
            let mut capture = Capture::new(Arc::clone(&producer_queue));
            for block in 0..256 {
                // Test-only pacing ensures every sample is delivered regardless
                // of OS scheduling. Overflow behavior is covered separately.
                while producer_queue
                    .write
                    .load(Ordering::Relaxed)
                    .wrapping_sub(producer_queue.read.load(Ordering::Acquire))
                    > CAPACITY - 256
                {
                    std::thread::yield_now();
                }
                let mut audio = [0.0; 512];
                for (i, stereo) in audio.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                    let value = (block * 256 + i) as f32;
                    stereo.copy_from_slice(&[value, -value]);
                }
                capture.record(&audio, 2, &[crate::seq::note_on(0, 60, 100)]);
                std::thread::yield_now();
            }
            producer_finished.store(true, Ordering::Release);
        });
        let mut previous = None;
        loop {
            // Observe completion before pop, so an empty queue after completion
            // cannot race with the producer's final publication.
            let done = finished.load(Ordering::Acquire);
            if let Some(sample) = queue.pop() {
                assert_eq!(
                    sample.stereo,
                    [sample.serial as f32, -(sample.serial as f32)]
                );
                assert_eq!(sample.note, 60);
                assert_eq!(sample.epoch, sample.serial / 256 + 1);
                assert!(previous.is_none_or(|serial| sample.serial > serial));
                previous = Some(sample.serial);
            } else if done {
                break;
            } else {
                std::thread::yield_now();
            }
        }
        producer.join().unwrap();
        assert_eq!(previous, Some(256 * 256 - 1));
    }
}
