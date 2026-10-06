//! Causal display translation; the reference is a position, not a waveform shape.
use crate::scope_boundary::Record;

pub const FOLLOW_SECONDS: f64 = 0.5;

fn wrap(value: f64) -> f64 {
    value - value.round()
}

#[derive(Default)]
pub struct Drift {
    key: Option<(u64, u8, u64)>,
    channel: usize,
    serial: u64,
    target: Option<f64>,
    reference: Option<f64>,
    enabled: bool,
    shift: f64,
}

impl Drift {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn observe(&mut self, record: &Record, rate: u32) -> f64 {
        let key = (record.epoch, record.note, record.decision_serial);
        let first = self.key != Some(key) || self.channel != record.channel;
        let phase = record.measured.phase;
        if first {
            let reference = (record.reference_serial.is_some() && self.channel == record.channel)
                .then_some(self.reference)
                .flatten();
            self.target = if reference.is_some() {
                self.target.or(reference)
            } else {
                phase
            };
            self.shift = phase.zip(reference).map_or(0.0, |(p, r)| wrap(r - p));
            self.enabled = phase.is_some();
            self.reference = None;
        } else if self.enabled {
            if let Some((phase, target)) = phase.zip(self.target) {
                let dt =
                    record.available_serial.saturating_sub(self.serial) as f64 / f64::from(rate);
                let alpha = -(-dt / FOLLOW_SECONDS).exp_m1();
                self.shift += alpha * wrap(target - phase - self.shift);
            }
        }
        if let Some(phase) = phase {
            self.reference = Some((phase + self.shift).rem_euclid(1.0));
        }
        self.key = Some(key);
        self.channel = record.channel;
        self.serial = record.available_serial;
        self.shift
    }
}

#[cfg(test)]
#[path = "scope_drift_tests.rs"]
mod tests;
