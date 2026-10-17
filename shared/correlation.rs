//! Stereo phase correlation over 150 ms of audio, independent of UI refresh rate.
use std::collections::VecDeque;

pub struct Meter {
    samples: VecDeque<[f32; 2]>,
    capacity: usize,
}

impl Meter {
    pub fn new(sample_rate: u32) -> Self {
        let capacity = (sample_rate as usize * 3 / 20).max(2);
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn push(&mut self, stereo: [f32; 2]) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples
            .push_back(stereo.map(|v| if v.is_finite() { v } else { 0.0 }));
    }

    pub fn value(&self) -> Option<f32> {
        // f64 preserves both very quiet and above-full-scale f32 input.
        let (mut left, mut right, mut product) = (0.0_f64, 0.0_f64, 0.0_f64);
        for &[l, r] in &self.samples {
            let (l, r) = (f64::from(l), f64::from(r));
            left += l * l;
            right += r * r;
            product += l * r;
        }
        // A silent channel has no defined correlation, rather than zero correlation.
        if left == 0.0 || right == 0.0 {
            None
        } else {
            Some((product / (left.sqrt() * right.sqrt())).clamp(-1.0, 1.0) as f32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_correlation_preserves_balance_and_handles_extreme_levels() {
        for amplitude in [0.01, f32::MIN_POSITIVE, f32::MAX / 2.0] {
            for (ratio, expected) in [(1.0, 1.0), (0.5, 1.0), (-0.5, -1.0)] {
                let mut meter = Meter::new(100);
                for sign in [-1.0, 1.0] {
                    meter.push([sign * amplitude, sign * amplitude * ratio]);
                }
                assert_eq!(meter.value(), Some(expected));
            }
        }
        let mut meter = Meter::new(100);
        for stereo in [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], [0.0, -1.0]] {
            meter.push(stereo);
        }
        assert_eq!(meter.value(), Some(0.0));
    }

    #[test]
    fn silence_one_sided_and_invalid_input_are_undefined() {
        let mut meter = Meter::new(100);
        assert_eq!(meter.value(), None);
        meter.push([0.0; 2]);
        meter.push([f32::NAN, f32::INFINITY]);
        assert_eq!(meter.value(), None);
        meter.push([1.0, 0.0]);
        assert_eq!(meter.value(), None);
    }

    #[test]
    fn average_uses_audio_time_and_clears_after_silence_or_gap() {
        for sample_rate in [100, 1000] {
            let mut meter = Meter::new(sample_rate);
            let width = sample_rate as usize * 3 / 20;
            for _ in 0..width {
                meter.push([1.0, 1.0]);
            }
            assert_eq!(meter.value(), Some(1.0));
            for _ in 0..width / 3 {
                meter.push([1.0, -1.0]);
            }
            assert!((meter.value().unwrap() - 1.0 / 3.0).abs() < 1e-6);
            for _ in 0..width {
                meter.push([0.0; 2]);
            }
            assert_eq!(meter.value(), None);
            meter.push([1.0, -1.0]);
            assert_eq!(meter.value(), Some(-1.0));
            meter.clear();
            assert_eq!(meter.value(), None);
        }
    }
}
