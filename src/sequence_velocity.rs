//! Velocity choices shared by the UI, persisted session and audio thread.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum SequenceVelocity {
    #[default]
    V100,
    V127,
    #[serde(alias = "random")]
    Range80To127,
    Range40To100,
}

impl SequenceVelocity {
    pub const TYPES: &'static [Self] = &[
        Self::V100,
        Self::V127,
        Self::Range40To100,
        Self::Range80To127,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::V100 => "100",
            Self::V127 => "127",
            Self::Range40To100 => "40-100",
            Self::Range80To127 => "80-127",
        }
    }

    pub fn value(self, index: usize, count: usize, down: bool) -> u8 {
        let (low, high) = match self {
            Self::V100 => return 100,
            Self::V127 => return 127,
            Self::Range40To100 => (40, 100),
            Self::Range80To127 => (80, 127),
        };
        let offset = ((high - low) as usize * index / (count - 1)) as u8;
        if down {
            high - offset
        } else {
            low + offset
        }
    }

    pub fn adjacent(self, forward: bool) -> Self {
        let index = Self::TYPES.iter().position(|v| *v == self).unwrap_or(0);
        let count = Self::TYPES.len();
        Self::TYPES[(index + if forward { 1 } else { count - 1 }) % count]
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::V127,
            2 => Self::Range80To127,
            3 => Self::Range40To100,
            _ => Self::V100,
        }
    }
}
