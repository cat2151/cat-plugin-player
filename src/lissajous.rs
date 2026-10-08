//! Rolling stereo window, independent of MIDI notes and scope triggers.
use std::collections::VecDeque;

pub struct Display {
    pub samples: Vec<[f32; 2]>,
    pub correlation: Option<f32>,
}

pub struct Analyzer {
    samples: VecDeque<[f32; 2]>,
    capacity: usize,
    dirty: bool,
    correlation: crate::correlation::Meter,
}

impl Analyzer {
    pub fn new(sample_rate: u32) -> Self {
        let capacity = (sample_rate as usize / 20).clamp(2, 4096);
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
            dirty: false,
            correlation: crate::correlation::Meter::new(sample_rate),
        }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
        self.dirty = false;
        self.correlation.clear();
    }

    pub fn push(&mut self, stereo: [f32; 2]) {
        self.correlation.push(stereo);
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(stereo);
        self.dirty = true;
    }

    pub fn display(&mut self) -> Option<Display> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        Some(Display {
            samples: self.samples.iter().copied().collect(),
            correlation: self.correlation.value(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlation_averages_longer_than_xy_and_resets_with_it() {
        let mut analyzer = Analyzer::new(100);
        for _ in 0..15 {
            analyzer.push([1.0, 1.0]);
        }
        assert_eq!(analyzer.display().unwrap().correlation, Some(1.0));
        for _ in 0..5 {
            analyzer.push([1.0, -1.0]);
        }
        let frame = analyzer.display().unwrap();
        assert_eq!(frame.samples, vec![[1.0, -1.0]; 5]);
        assert!((frame.correlation.unwrap() - 1.0 / 3.0).abs() < 1e-6);
        analyzer.clear();
        analyzer.push([0.0; 2]);
        assert_eq!(analyzer.display().unwrap().correlation, None);
    }

    #[test]
    fn rolling_window_replaces_signal_with_silence_and_clears_gaps() {
        let mut analyzer = Analyzer::new(100);
        for i in 0..10 {
            analyzer.push([i as f32, -(i as f32)]);
        }
        assert_eq!(
            analyzer.display().unwrap().samples,
            (5..10).map(|i| [i as f32, -(i as f32)]).collect::<Vec<_>>()
        );
        assert!(analyzer.display().is_none());
        for _ in 0..5 {
            analyzer.push([0.0; 2]);
        }
        assert_eq!(analyzer.display().unwrap().samples, vec![[0.0; 2]; 5]);
        analyzer.clear();
        assert!(analyzer.display().is_none());
        analyzer.push([0.2, -0.1]);
        assert_eq!(analyzer.display().unwrap().samples, vec![[0.2, -0.1]]);
    }
}
