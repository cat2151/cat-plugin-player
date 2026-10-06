//! Consume every source sample and publish consecutive complete scope windows.
use crate::scope::Display;
use crate::scope_boundary::Boundary;
use crate::scope_capture::Sample;
use crate::scope_drift::Drift;
use crate::scope_trigger;

pub struct Stream {
    rate: u32,
    settings: u8,
    key: Option<(u64, u8)>,
    serial: Option<u64>,
    samples: Vec<[f32; 2]>,
    boundary: Boundary,
    drift: Drift,
}

impl Stream {
    pub fn new(rate: u32, settings: u8) -> Self {
        Self {
            rate,
            settings,
            key: None,
            serial: None,
            samples: Vec::new(),
            boundary: Boundary::default(),
            drift: Drift::default(),
        }
    }

    pub fn sample(&mut self, sample: Sample) -> Option<Display> {
        if self
            .serial
            .is_some_and(|s| sample.serial != s.wrapping_add(1))
            || sample.note >= 128
        {
            self.samples.clear();
            self.boundary.reset();
            self.drift.reset();
            self.key = None;
        }
        self.serial = Some(sample.serial);
        if sample.note >= 128 {
            return None;
        }
        let key = (sample.epoch, sample.note);
        if self.key != Some(key) {
            self.samples.clear();
            self.key = Some(key);
        }
        let width = scope_trigger::width(self.rate, sample.note, self.settings & 15);
        let analysis_width = scope_trigger::width(self.rate, sample.note, 4);
        self.samples.push(sample.stereo);
        if self.samples.len() < width.max(analysis_width) {
            return None;
        }
        let record = self.boundary.observe(
            sample.epoch,
            sample.note,
            sample.serial + 1,
            &self.samples[..analysis_width],
        );
        let shift = self.drift.observe(&record, self.rate);
        let result = Display {
            samples: self.samples[..width].to_vec(),
            note: sample.note,
            sample_rate: self.rate,
            settings: self.settings,
            shift_samples: shift * analysis_width as f64 / 4.0,
            markers: record
                .measured
                .markers
                .into_iter()
                .filter(|&j| j < width)
                .collect(),
        };
        self.samples.clear();
        Some(result)
    }
}

#[cfg(test)]
#[path = "scope_stream_tests.rs"]
mod tests;
