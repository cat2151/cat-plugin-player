//! Scope worker lifecycle and latest completed display frame.
use crate::scope_capture::{Capture, Queue};
use crate::scope_trigger::{self, Trigger};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub struct Display {
    pub samples: Vec<[f32; 2]>,
    pub note: u8,
    pub sample_rate: u32,
    pub settings: u8,
    pub shift_samples: f64,
    pub markers: Vec<usize>,
}

struct Shared {
    settings: AtomicU8,
    stop: AtomicBool,
    display: Mutex<Option<Arc<Display>>>,
    spectrum: Mutex<Option<Arc<crate::spectrum::Display>>>,
    lissajous: Mutex<Option<Arc<crate::lissajous::Display>>>,
}

pub struct Scope {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

pub fn settings(cycles: u8, trigger: Trigger) -> u8 {
    cycles
        | if trigger == Trigger::Similarity {
            16
        } else {
            0
        }
}

impl Scope {
    pub fn start(sample_rate: u32) -> Result<(Capture, Self), String> {
        let queue = Queue::new();
        let shared = Arc::new(Shared {
            settings: AtomicU8::new(settings(4, Trigger::Similarity)),
            stop: AtomicBool::new(false),
            display: Mutex::new(None),
            spectrum: Mutex::new(None),
            lissajous: Mutex::new(None),
        });
        let worker_shared = Arc::clone(&shared);
        let worker_queue = Arc::clone(&queue);
        let worker = thread::Builder::new()
            .name("oscilloscope".into())
            .spawn(move || run(worker_queue, worker_shared, sample_rate))
            .map_err(|e| format!("cannot start oscilloscope: {e}"))?;
        Ok((
            Capture::new(queue),
            Self {
                shared,
                worker: Some(worker),
            },
        ))
    }

    pub fn configure(&self, cycles: u8, trigger: Trigger) {
        self.shared
            .settings
            .store(settings(cycles, trigger), Ordering::Relaxed);
    }

    pub fn display(&self) -> Option<Arc<Display>> {
        self.shared.display.try_lock().ok()?.clone()
    }

    pub fn spectrum(&self) -> Option<Arc<crate::spectrum::Display>> {
        self.shared.spectrum.try_lock().ok()?.clone()
    }

    pub fn lissajous(&self) -> Option<Arc<crate::lissajous::Display>> {
        self.shared.lissajous.try_lock().ok()?.clone()
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(queue: Arc<Queue>, shared: Arc<Shared>, sample_rate: u32) {
    let mut samples = VecDeque::new();
    let mut epoch = None;
    let mut serial = None;
    let mut note = 128;
    let mut current_settings = 0;
    let mut last_display = Instant::now();
    let mut dirty = false;
    let mut stream = crate::scope_stream::Stream::new(sample_rate, 0);
    let mut spectrum = crate::spectrum::Analyzer::new(sample_rate);
    let mut lissajous = crate::lissajous::Analyzer::new(sample_rate);
    let mut last_spectrum = Instant::now();
    while !shared.stop.load(Ordering::Relaxed) {
        let settings = shared.settings.load(Ordering::Relaxed);
        if settings != current_settings {
            current_settings = settings;
            samples.clear();
            stream = crate::scope_stream::Stream::new(sample_rate, settings);
            *shared.display.lock().unwrap() = None;
            dirty = true;
        }
        let cycles = settings & 15;
        let trigger = if settings & 16 == 0 {
            Trigger::ZeroCross
        } else {
            Trigger::Similarity
        };
        // Bound each drain so continuous production cannot starve processing/shutdown.
        for _ in 0..16_384 {
            let Some(sample) = queue.pop() else {
                break;
            };
            let gap = serial.is_some_and(|s: u64| sample.serial != s.wrapping_add(1));
            if gap {
                spectrum.clear();
                *shared.spectrum.lock().unwrap() = None;
                lissajous.clear();
                *shared.lissajous.lock().unwrap() = None;
            }
            spectrum.push(sample.stereo);
            lissajous.push(sample.stereo);
            if epoch != Some(sample.epoch) || note != sample.note || gap {
                samples.clear();
                if trigger == Trigger::ZeroCross || gap || sample.note >= 128 {
                    *shared.display.lock().unwrap() = None;
                }
                epoch = Some(sample.epoch);
            }
            serial = Some(sample.serial);
            note = sample.note;
            if trigger == Trigger::Similarity {
                if let Some(frame) = stream.sample(sample) {
                    if shared.settings.load(Ordering::Relaxed) == settings {
                        *shared.display.lock().unwrap() = Some(Arc::new(frame));
                    }
                }
                continue;
            }
            if note < 128 {
                samples.push_back(sample.stereo);
                let required = 2 * scope_trigger::width(sample_rate, note, cycles) - 1;
                while samples.len() > required {
                    samples.pop_front();
                }
                dirty = true;
            }
        }
        if last_spectrum.elapsed() >= Duration::from_millis(50) {
            if let Some(frame) = lissajous.display() {
                *shared.lissajous.lock().unwrap() = Some(Arc::new(frame));
            }
            if let Some(frame) = spectrum.display() {
                *shared.spectrum.lock().unwrap() = Some(Arc::new(frame));
            }
            last_spectrum = Instant::now();
        }
        if trigger == Trigger::ZeroCross
            && dirty
            && note < 128
            && last_display.elapsed() >= Duration::from_millis(50)
        {
            let width = scope_trigger::width(sample_rate, note, cycles);
            if samples.len() >= 2 * width - 1 {
                let data = samples.make_contiguous();
                let start = scope_trigger::zero_cross(data, width);
                if shared.settings.load(Ordering::Relaxed) == settings {
                    *shared.display.lock().unwrap() = Some(Arc::new(Display {
                        samples: data[start..start + width].to_vec(),
                        note,
                        sample_rate,
                        settings,
                        shift_samples: 0.0,
                        markers: Vec::new(),
                    }));
                }
                last_display = Instant::now();
                dirty = false;
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spectrum_updates_without_notes_and_decays_to_silence() {
        let (mut capture, scope) = Scope::start(48_000).unwrap();
        let audio: Vec<f32> = (0..crate::spectrum::FFT_SIZE)
            .map(|i| {
                (i as f32 * std::f32::consts::TAU * 80.0 / crate::spectrum::FFT_SIZE as f32).sin()
            })
            .collect();
        capture.record(&audio, 1, &[]);
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if scope.spectrum().is_some_and(|f| f.bins[80][0] > -0.1) {
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert!(scope.display().is_none());
        let stereo = scope.lissajous().unwrap();
        assert_eq!(stereo.samples.len(), 2400);
        assert!(stereo.samples.iter().all(|s| s[0] == s[1]));
        assert!(stereo.samples.iter().any(|s| s[0].abs() > 0.5));
        scope.configure(1, Trigger::ZeroCross);
        capture.record(&vec![0.0; crate::spectrum::FFT_SIZE], 1, &[]);
        loop {
            if scope.spectrum().is_some_and(|f| {
                f.bins
                    .iter()
                    .flatten()
                    .all(|db| *db == crate::spectrum::FLOOR_DB)
            }) {
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert!(scope
            .lissajous()
            .unwrap()
            .samples
            .iter()
            .all(|s| *s == [0.0; 2]));
    }

    #[test]
    fn worker_publishes_stereo_and_applies_new_settings() {
        let (mut capture, scope) = Scope::start(8_000).unwrap();
        let audio: Vec<f32> = (0..512)
            .flat_map(|i| {
                let sample = (i as f32 * std::f32::consts::TAU * 440.0 / 8_000.0).sin();
                [sample, -sample]
            })
            .collect();
        capture.record(&audio, 2, &[crate::seq::note_on(0, 69, 100)]);
        let deadline = Instant::now() + Duration::from_secs(2);
        while scope.display().is_none() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        let frame = scope.display().unwrap();
        assert_eq!(frame.note, 69);
        assert_eq!(frame.samples.len(), scope_trigger::width(8_000, 69, 4));
        assert!(frame.samples.iter().all(|s| s[0] == -s[1]));
        scope.configure(1, Trigger::Similarity);
        // A setting change starts a fresh source window; keep delivering audio.
        loop {
            capture.record(&audio, 2, &[]);
            if scope
                .display()
                .is_some_and(|f| f.settings == settings(1, Trigger::Similarity))
            {
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            scope.display().unwrap().samples.len(),
            scope_trigger::width(8_000, 69, 1)
        );
    }
}
