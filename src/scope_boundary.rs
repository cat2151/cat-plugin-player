//! Causal note-boundary alignment of four-cycle source windows.
//! Changes display position only; no samples or amplitudes are modified.

#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    pub phase: Option<f64>,
    pub accepted: usize,
    pub rejected: usize,
    pub markers: Vec<usize>,
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    }
}

fn wrap(delta: f64) -> f64 {
    delta - delta.round()
}

pub fn measure(samples: &[[f32; 2]], channel: usize) -> Measurement {
    measure_near(samples, channel, None)
}

fn measure_near(samples: &[[f32; 2]], channel: usize, previous: Option<f64>) -> Measurement {
    let empty = Measurement {
        phase: None,
        accepted: 0,
        rejected: 0,
        markers: Vec::new(),
    };
    if samples.len() < 8 || samples.iter().any(|s| !s[channel].is_finite()) {
        return empty;
    }
    let period = samples.len() as f64 / 4.0;
    let mut phases = Vec::new();
    let mut positions = Vec::new();
    for cycle in 0..4 {
        let begin = ((cycle as f64 * period).ceil() as usize).max(1);
        let end = (((cycle + 1) as f64 * period).ceil() as usize).min(samples.len() - 1);
        let best = (begin..end)
            .filter(|&j| {
                samples[j][channel] > 0.0
                    && samples[j][channel] > samples[j - 1][channel]
                    && samples[j][channel] >= samples[j + 1][channel]
            })
            .filter(|&j| previous.is_none_or(|p| wrap(j as f64 / period - p).abs() <= 0.12))
            .max_by(|&a, &b| {
                if let Some(p) = previous {
                    wrap(b as f64 / period - p)
                        .abs()
                        .total_cmp(&wrap(a as f64 / period - p).abs())
                } else {
                    samples[a][channel].total_cmp(&samples[b][channel])
                }
            });
        if let Some(j) = best {
            positions.push(j);
            let phase = j as f64 / period - cycle as f64;
            phases.push(previous.map_or(phase, |p| p + wrap(phase - p)));
        }
    }
    if phases.len() < 3 {
        return Measurement {
            accepted: phases.len(),
            markers: positions,
            ..empty
        };
    }
    let phase = median(&mut phases.clone());
    let accepted = phases
        .iter()
        .filter(|&&v| (v - phase).abs() <= 0.12)
        .count();
    Measurement {
        phase: (accepted >= 3).then_some(phase.rem_euclid(1.0)),
        accepted,
        rejected: phases.len() - accepted,
        markers: positions
            .into_iter()
            .zip(phases)
            .filter_map(|(j, p)| ((p - phase).abs() <= 0.12).then_some(j))
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub available_serial: u64,
    pub epoch: u64,
    pub note: u8,
    pub channel: usize,
    pub measured: Measurement,
    pub reference: Option<f64>,
    pub shift: f64,
    pub reason: &'static str,
    pub decision_serial: u64,
    pub reference_serial: Option<u64>,
}

#[derive(Default, Clone)]
pub struct Boundary {
    key: Option<(u64, u8)>,
    channel: Option<usize>,
    previous: Option<(f64, u64)>,
    reference: Option<f64>,
    reference_serial: Option<u64>,
    shift: f64,
    reason: &'static str,
    decision_serial: u64,
    marker_phase: Option<f64>,
}

impl Boundary {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn observe(&mut self, epoch: u64, note: u8, serial: u64, samples: &[[f32; 2]]) -> Record {
        // Lock the stereo reference channel per note; do not switch phase on a quiet frame.
        let first = self.key != Some((epoch, note));
        if first {
            let next_channel = crate::scope_trigger::channel(samples);
            if self.channel.is_some_and(|ch| ch != next_channel) {
                self.previous = None;
            }
            self.channel = Some(next_channel);
        }
        let channel = self.channel.unwrap();
        if first {
            self.marker_phase = None;
        }
        let measured = if self.marker_phase.is_some() {
            measure_near(samples, channel, self.marker_phase)
        } else {
            measure(samples, channel)
        };
        if first {
            self.reference = self.previous.map(|(phase, _)| phase);
            self.reference_serial = self.previous.map(|(_, serial)| serial);
            self.shift = measured
                .phase
                .zip(self.reference)
                .map_or(0.0, |(p, r)| wrap(r - p));
            self.reason = if measured.phase.is_none() {
                "first_window_unreliable_hold"
            } else if self.reference.is_none() {
                "no_previous_reference_hold"
            } else {
                "boundary_aligned"
            };
            self.decision_serial = serial;
            self.key = Some((epoch, note));
            self.previous = None;
        }
        if let Some(phase) = measured.phase {
            self.marker_phase = Some(phase);
            self.previous = Some(((phase + self.shift).rem_euclid(1.0), serial));
        }
        Record {
            available_serial: serial,
            epoch,
            note,
            channel,
            measured,
            reference: self.reference,
            shift: self.shift,
            reason: self.reason,
            decision_serial: self.decision_serial,
            reference_serial: self.reference_serial,
        }
    }
}

#[cfg(test)]
#[path = "scope_boundary_tests.rs"]
mod tests;
