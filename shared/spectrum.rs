//! Fixed-window stereo spectrum, computed exclusively on the scope worker.
use std::collections::VecDeque;

pub const FFT_SIZE: usize = 4096;
pub const FLOOR_DB: f32 = -100.0;

pub struct Display {
    pub bins: Vec<[f32; 2]>,
    pub sample_rate: u32,
}

pub struct Analyzer {
    samples: VecDeque<[f32; 2]>,
    sample_rate: u32,
    dirty: bool,
}

impl Analyzer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            samples: VecDeque::with_capacity(FFT_SIZE),
            sample_rate,
            dirty: false,
        }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
        self.dirty = false;
    }

    pub fn push(&mut self, stereo: [f32; 2]) {
        if self.samples.len() == FFT_SIZE {
            self.samples.pop_front();
        }
        self.samples.push_back(stereo);
        self.dirty = true;
    }

    pub fn display(&mut self) -> Option<Display> {
        if !self.dirty || self.samples.len() < FFT_SIZE {
            return None;
        }
        self.dirty = false;
        let mut bins = vec![[FLOOR_DB; 2]; FFT_SIZE / 2 + 1];
        for channel in 0..2 {
            // Periodic Hann window: coherent gain 0.5. Normalize a full-scale
            // bin-centered sine to 0 dBFS; retain stereo levels independently.
            let mut data: Vec<[f64; 2]> = self
                .samples
                .iter()
                .enumerate()
                .map(|(i, sample)| {
                    let window =
                        0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / FFT_SIZE as f64).cos();
                    let value = if sample[channel].is_finite() {
                        f64::from(sample[channel])
                    } else {
                        0.0
                    };
                    [value * window, 0.0]
                })
                .collect();
            fft(&mut data);
            for (i, bin) in bins.iter_mut().enumerate() {
                let scale = if i == 0 || i == FFT_SIZE / 2 {
                    2.0
                } else {
                    4.0
                };
                let amplitude = data[i][0].hypot(data[i][1]) * scale / FFT_SIZE as f64;
                bin[channel] = (20.0 * amplitude.log10()).max(f64::from(FLOOR_DB)) as f32;
            }
        }
        Some(Display {
            bins,
            sample_rate: self.sample_rate,
        })
    }
}

// Iterative radix-2 FFT. The fixed power-of-two window avoids extra dependencies.
fn fft(data: &mut [[f64; 2]]) {
    let n = data.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            data.swap(i, j);
        }
    }
    let mut width = 2;
    while width <= n {
        let angle = -std::f64::consts::TAU / width as f64;
        let step = [angle.cos(), angle.sin()];
        for start in (0..n).step_by(width) {
            let mut phase = [1.0, 0.0];
            for offset in 0..width / 2 {
                let a = data[start + offset];
                let b = data[start + offset + width / 2];
                let rotated = [
                    b[0] * phase[0] - b[1] * phase[1],
                    b[0] * phase[1] + b[1] * phase[0],
                ];
                data[start + offset] = [a[0] + rotated[0], a[1] + rotated[1]];
                data[start + offset + width / 2] = [a[0] - rotated[0], a[1] - rotated[1]];
                phase = [
                    phase[0] * step[0] - phase[1] * step[1],
                    phase[0] * step[1] + phase[1] * step[0],
                ];
            }
        }
        width *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_tones_have_correct_frequency_and_absolute_level() {
        let mut analyzer = Analyzer::new(48_000);
        for i in 0..FFT_SIZE {
            let phase = std::f64::consts::TAU * i as f64 / FFT_SIZE as f64;
            analyzer.push([
                (phase * 80.0).sin() as f32,
                (phase * 160.0).sin() as f32 * 0.5,
            ]);
        }
        let display = analyzer.display().unwrap();
        for (ch, expected_bin, expected_db) in [(0, 80, 0.0), (1, 160, -6.0206)] {
            let (peak, value) = display
                .bins
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a[ch].total_cmp(&b[ch]))
                .unwrap();
            assert_eq!(peak, expected_bin);
            assert!((value[ch] - expected_db).abs() < 0.01);
        }
        assert!(analyzer.display().is_none());
    }

    #[test]
    fn silence_invalid_samples_and_discontinuities_do_not_leave_old_peaks() {
        let mut analyzer = Analyzer::new(48_000);
        for _ in 0..FFT_SIZE {
            analyzer.push([f32::NAN, f32::INFINITY]);
        }
        assert!(analyzer
            .display()
            .unwrap()
            .bins
            .iter()
            .flatten()
            .all(|db| *db == FLOOR_DB));
        analyzer.clear();
        for _ in 0..FFT_SIZE - 1 {
            analyzer.push([0.0; 2]);
        }
        assert!(analyzer.display().is_none());
        analyzer.push([0.0; 2]);
        assert!(analyzer
            .display()
            .unwrap()
            .bins
            .iter()
            .flatten()
            .all(|db| *db == FLOOR_DB));
    }
}
