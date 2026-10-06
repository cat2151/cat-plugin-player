//! CC1 choices and sample-timed modulation, independent of note timing.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum SequenceModulation {
    #[default]
    Zero,
    Full,
    Sweep,
}

impl SequenceModulation {
    pub const TYPES: &'static [Self] = &[Self::Zero, Self::Full, Self::Sweep];

    pub fn label(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::Full => "127",
            Self::Sweep => "sweep",
        }
    }

    pub fn adjacent(self, forward: bool) -> Self {
        let index = Self::TYPES.iter().position(|v| *v == self).unwrap_or(0);
        let count = Self::TYPES.len();
        Self::TYPES[(index + if forward { 1 } else { count - 1 }) % count]
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Full,
            2 => Self::Sweep,
            _ => Self::Zero,
        }
    }
}

pub(crate) fn cc1(value: u8) -> u32 {
    0x20B0_0100 | u32::from(value & 0x7F)
}

#[derive(Default)]
pub(crate) struct Modulation {
    selected: Option<SequenceModulation>,
    pending: bool,
    elapsed: u64,
    step: u64,
}

impl Modulation {
    pub fn select(&mut self, selected: SequenceModulation) {
        if self.selected != Some(selected) {
            self.selected = Some(selected);
            self.restart();
        }
    }

    pub fn restart(&mut self) {
        self.pending = self.selected.is_some();
        self.elapsed = 0;
        self.step = 0;
    }

    pub fn next(&self, rate: u32, playing: bool) -> Option<usize> {
        if self.pending {
            Some(0)
        } else if playing && self.selected == Some(SequenceModulation::Sweep) {
            let at = (self.step * 2 * u64::from(rate)).div_ceil(127);
            Some(at.saturating_sub(self.elapsed) as usize)
        } else {
            None
        }
    }

    pub fn value(&self) -> u8 {
        match self.selected {
            Some(SequenceModulation::Full) => 127,
            Some(SequenceModulation::Sweep) => {
                let phase = (self.step % 254) as u8;
                if phase <= 127 {
                    phase
                } else {
                    254 - phase
                }
            }
            _ => 0,
        }
    }

    pub fn sent(&mut self) {
        self.pending = false;
        self.step += 1;
    }

    pub fn advance(&mut self, frames: usize, playing: bool) {
        if playing && self.selected == Some(SequenceModulation::Sweep) {
            self.elapsed += frames as u64;
        }
    }
}
