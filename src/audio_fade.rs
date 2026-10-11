//! Short gain ramps around an effect swap (ADR 0027), applied on the audio thread.

pub(crate) const FADE: std::time::Duration = std::time::Duration::from_millis(50);

/// Silent callbacks written after reaching zero gain, so the device has played
/// the end of the ramp before the stream stops.
const SILENT_CALLBACKS: u8 = 2;

pub(crate) struct Fade {
    gain: f32,
    step: f32,
    silent_callbacks: u8,
}

impl Fade {
    pub fn new(sample_rate: u32, fade_in: bool) -> Self {
        Self {
            gain: if fade_in { 0.0 } else { 1.0 },
            step: 1.0 / (FADE.as_secs_f32() * sample_rate as f32),
            silent_callbacks: 0,
        }
    }

    /// Ramps toward silence while `out` is set, otherwise toward unity gain.
    pub fn apply(&mut self, chunk: &mut [f32], channels: usize, out: bool) {
        if self.gain == 1.0 && !out {
            return;
        }
        for frame in chunk.chunks_mut(channels) {
            self.gain = if out {
                (self.gain - self.step).max(0.0)
            } else {
                (self.gain + self.step).min(1.0)
            };
            for sample in frame {
                *sample *= self.gain;
            }
        }
    }

    /// Call once per device callback; true once the faded-out output has been silent long enough.
    pub fn finish_callback(&mut self, out: bool) -> bool {
        if out && self.gain == 0.0 {
            self.silent_callbacks = self.silent_callbacks.saturating_add(1);
        }
        self.silent_callbacks >= SILENT_CALLBACKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;
    const FRAMES: usize = 2400;

    fn ones() -> Vec<f32> {
        vec![1.0; FRAMES * 2]
    }

    #[test]
    fn fade_in_ramps_from_silence_to_unity_over_fifty_ms() {
        let mut fade = Fade::new(RATE, true);
        let mut chunk = ones();
        fade.apply(&mut chunk, 2, false);
        assert!(chunk[0] < 0.001 && chunk[0] == chunk[1]);
        assert!((chunk[FRAMES] - 0.5).abs() < 0.01);
        assert_eq!(chunk[FRAMES * 2 - 1], 1.0);
        let mut next = ones();
        fade.apply(&mut next, 2, false);
        assert_eq!(next, ones());
    }

    #[test]
    fn without_fade_in_unity_output_is_untouched() {
        let mut fade = Fade::new(RATE, false);
        let mut chunk = ones();
        fade.apply(&mut chunk, 2, false);
        assert_eq!(chunk, ones());
        assert!(!fade.finish_callback(false));
    }

    #[test]
    fn fade_out_reports_done_after_two_silent_callbacks() {
        let mut fade = Fade::new(RATE, false);
        let mut chunk = ones();
        fade.apply(&mut chunk, 2, true);
        assert!((chunk[FRAMES] - 0.5).abs() < 0.01);
        assert_eq!(chunk[FRAMES * 2 - 1], 0.0);
        assert!(!fade.finish_callback(true));
        let mut silent = ones();
        fade.apply(&mut silent, 2, true);
        assert!(silent.iter().all(|&sample| sample == 0.0));
        assert!(fade.finish_callback(true));
    }

    #[test]
    fn fade_out_during_fade_in_starts_from_the_current_gain() {
        let mut fade = Fade::new(RATE, true);
        let mut chunk = vec![1.0; 240 * 2];
        fade.apply(&mut chunk, 2, false);
        let mut out = vec![1.0; 2];
        fade.apply(&mut out, 2, true);
        assert!(out[0] < chunk[chunk.len() - 1]);
        assert!(!fade.finish_callback(true));
    }
}
