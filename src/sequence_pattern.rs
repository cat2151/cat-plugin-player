//! Stable persisted identities for the cyclic playback selector.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum SequencePattern {
    Off,
    #[default]
    Steps,
    GuitarArpeggio,
    Csus4CArpeggio,
    Fmaj7G6Arpeggio,
    Custom,
}

impl SequencePattern {
    /// Display order shared by the dropdown and both arrows. Off is transport state.
    pub const TYPES: &'static [Self] = &[
        Self::Steps,
        Self::GuitarArpeggio,
        Self::Csus4CArpeggio,
        Self::Fmaj7G6Arpeggio,
    ];

    pub fn selection(self, remembered: Self) -> Self {
        let candidate = if self == Self::Off { remembered } else { self };
        if Self::TYPES.contains(&candidate) || candidate == Self::Custom {
            candidate
        } else {
            Self::default()
        }
    }

    pub fn adjacent(self, forward: bool) -> Self {
        let index = Self::TYPES.iter().position(|p| *p == self).unwrap_or(0);
        let count = Self::TYPES.len();
        Self::TYPES[(index + if forward { 1 } else { count - 1 }) % count]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Stopped",
            Self::Steps => "4 notes",
            Self::GuitarArpeggio => "Em7(add11)",
            Self::Csus4CArpeggio => "Csus4-C",
            Self::Fmaj7G6Arpeggio => "FM7-G6",
            Self::Custom => "MML / Chord",
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Off,
            2 => Self::GuitarArpeggio,
            3 => Self::Csus4CArpeggio,
            4 => Self::Fmaj7G6Arpeggio,
            5 => Self::Custom,
            _ => Self::Steps,
        }
    }
    /// Input phrases are transient, so their identity is not persisted.
    pub fn persisted(self) -> Self {
        if self == Self::Custom {
            Self::Steps
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_wrap_and_never_select_stopped_state() {
        for (i, &pattern) in SequencePattern::TYPES.iter().enumerate() {
            let next = pattern.adjacent(true);
            assert_eq!(
                next,
                SequencePattern::TYPES[(i + 1) % SequencePattern::TYPES.len()]
            );
            assert_eq!(next.adjacent(false), pattern);
            assert_ne!(next, SequencePattern::Off);
            assert_eq!(SequencePattern::from_u8(pattern as u8), pattern);
        }
        assert_eq!(
            SequencePattern::Steps.adjacent(false),
            SequencePattern::Fmaj7G6Arpeggio
        );
    }
}
