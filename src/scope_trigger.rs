//! Sample-domain oscilloscope alignment, independent of audio devices and UI.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Trigger {
    #[default]
    ZeroCross,
    Similarity,
}

pub fn width(sample_rate: u32, note: u8, cycles: u8) -> usize {
    let frequency = 440.0 * 2.0_f64.powf((f64::from(note) - 69.0) / 12.0);
    (f64::from(sample_rate) / frequency * f64::from(cycles))
        .round()
        .max(2.0) as usize
}

pub(crate) fn channel(samples: &[[f32; 2]]) -> usize {
    if samples.iter().any(|s| s[0].abs() > 1e-6) {
        0
    } else {
        1
    }
}

pub fn zero_cross(samples: &[[f32; 2]], width: usize) -> usize {
    let ch = channel(samples);
    (0..width.min(samples.len().saturating_sub(width) + 1))
        .find(|&i| i + 1 < samples.len() && samples[i][ch] <= 0.0 && samples[i + 1][ch] > 0.0)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_controls_period_and_octave_halves_width() {
        assert_eq!(width(48_000, 69, 1), 109);
        assert_eq!(width(48_000, 69, 8), 873);
        assert_eq!(width(48_000, 81, 8), 436);
    }

    #[test]
    fn right_channel_crossing_is_used_when_left_is_silent() {
        let samples = [
            [0.0, -1.0],
            [0.0, -0.5],
            [0.0, 1.0],
            [0.0, 0.5],
            [0.0, -1.0],
        ];
        assert_eq!(zero_cross(&samples, 3), 1);
    }
}
