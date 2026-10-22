//! CC1 / macro CC choices and sample-timed modulation, independent of note timing.
use crate::seq::EventBuf;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SequenceModulation {
    /// 0-127, from the list or by dragging the value bar.
    Fixed(u8),
    Sweep,
}

impl SequenceModulation {
    /// `None` ("-") sends nothing, so the plugin keeps its own value.
    pub const CHOICES: [Option<Self>; 4] = [
        None,
        Some(Self::Fixed(0)),
        Some(Self::Fixed(127)),
        Some(Self::Sweep),
    ];

    const SWEEP: u8 = 128;

    pub fn label(choice: Option<Self>) -> String {
        match choice {
            None => "-".into(),
            Some(Self::Fixed(value)) => value.to_string(),
            Some(Self::Sweep) => "sweep".into(),
        }
    }

    pub fn adjacent(choice: Option<Self>, forward: bool) -> Option<Self> {
        let index = Self::CHOICES.iter().position(|v| *v == choice).unwrap_or(0);
        let count = Self::CHOICES.len();
        Self::CHOICES[(index + if forward { 1 } else { count - 1 }) % count]
    }

    /// A fixed value would replace the value saved in a newly loaded patch.
    pub fn release_fixed(choice: &mut Option<Self>) -> bool {
        let fixed = matches!(choice, Some(Self::Fixed(_)));
        if fixed {
            *choice = None;
        }
        fixed
    }

    /// One key sets every macro to "-" once each shown macro sweeps, else to sweep.
    /// Hidden macros follow too, so a later patch's macros match the choice.
    pub fn toggle_sweep(choices: &mut [Option<Self>; MACROS], shown: &[Option<u8>; MACROS]) {
        let sweeping = choices
            .iter()
            .zip(shown)
            .all(|(choice, cc)| cc.is_none() || *choice == Some(Self::Sweep));
        *choices = [if sweeping { None } else { Some(Self::Sweep) }; MACROS];
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0..=127 => Some(Self::Fixed(value)),
            Self::SWEEP => Some(Self::Sweep),
            _ => None,
        }
    }

    pub fn to_u8(choice: Option<Self>) -> u8 {
        match choice {
            None => u8::MAX,
            Some(Self::Fixed(value)) => value.min(127),
            Some(Self::Sweep) => Self::SWEEP,
        }
    }
}

pub(crate) fn cc(controller: u8, value: u8) -> u32 {
    0x20B0_0000 | (u32::from(controller & 0x7F) << 8) | u32::from(value & 0x7F)
}

#[cfg(test)]
pub(crate) fn cc1(value: u8) -> u32 {
    cc(1, value)
}

/// One controller's choice; sweep takes `half_period_ms` each way.
pub(crate) struct Modulation {
    controller: u8,
    half_period_ms: u64,
    selected: Option<SequenceModulation>,
    pending: bool,
    elapsed: u64,
    step: u64,
    current: u8,
}

impl Modulation {
    const fn new(controller: u8, half_period_ms: u64) -> Self {
        Self {
            controller,
            half_period_ms,
            selected: None,
            pending: false,
            elapsed: 0,
            step: 0,
            current: 0,
        }
    }

    /// `None` stops sending without touching the plugin's current value.
    pub fn select(&mut self, selected: Option<SequenceModulation>) {
        if self.selected != selected {
            self.selected = selected;
            self.restart();
        }
    }

    fn set_controller(&mut self, controller: u8) {
        if self.controller != controller {
            self.controller = controller;
            self.restart();
        }
    }

    fn restart(&mut self) {
        self.pending = self.selected.is_some();
        self.elapsed = 0;
        self.step = 0;
    }

    fn next(&self, rate: u32, playing: bool) -> Option<usize> {
        if self.pending {
            Some(0)
        } else if playing && self.selected == Some(SequenceModulation::Sweep) {
            let at = (self.step * self.half_period_ms * u64::from(rate)).div_ceil(127 * 1000);
            Some(at.saturating_sub(self.elapsed) as usize)
        } else {
            None
        }
    }

    fn value(&self) -> u8 {
        match self.selected {
            Some(SequenceModulation::Fixed(value)) => value,
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

    fn sent(&mut self) {
        self.current = self.value();
        self.pending = false;
        self.step += 1;
    }

    fn advance(&mut self, frames: usize, playing: bool) {
        if playing && self.selected == Some(SequenceModulation::Sweep) {
            self.elapsed += frames as u64;
        }
    }
}

/// The instrument's macro count (Surge XT: Macro 1-8), one lane each.
pub(crate) const MACROS: usize = 8;

/// Per macro: (controller, choice); `None` sends nothing.
pub(crate) type MacroLanes = [Option<(u8, SequenceModulation)>; MACROS];

const CC1_HALF_PERIOD_MS: u64 = 2000;

/// Each a different prime times 100 ms, none a multiple of the CC1 period, so
/// no two sweeps line up and the macros keep moving against each other.
pub(crate) const MACRO_HALF_PERIOD_MS: [u64; MACROS] =
    [2900, 3700, 4300, 5300, 6100, 7100, 7900, 8900];

/// CC1 and the patch's macro CCs (ADR 0036) on one sample timeline.
pub(crate) struct Lanes {
    pub cc1: Modulation,
    macros: [Modulation; MACROS],
}

impl Default for Lanes {
    fn default() -> Self {
        Self {
            cc1: Modulation::new(1, CC1_HALF_PERIOD_MS),
            macros: MACRO_HALF_PERIOD_MS.map(|ms| Modulation::new(0, ms)),
        }
    }
}

impl Lanes {
    pub fn select_macros(&mut self, lanes: &MacroLanes) {
        for (lane, selected) in self.macros.iter_mut().zip(lanes) {
            match *selected {
                Some((controller, selected)) => {
                    lane.set_controller(controller);
                    lane.select(Some(selected));
                }
                None => lane.select(None),
            }
        }
    }

    fn all(&mut self) -> impl Iterator<Item = &mut Modulation> {
        std::iter::once(&mut self.cc1).chain(&mut self.macros)
    }

    pub fn restart(&mut self) {
        self.all().for_each(Modulation::restart);
    }

    /// Samples from the reference point to the earliest controller event.
    pub fn next(&self, rate: u32, playing: bool) -> Option<usize> {
        std::iter::once(&self.cc1)
            .chain(&self.macros)
            .filter_map(|lane| lane.next(rate, playing))
            .min()
    }

    /// Pushes every controller event due at `at`; false when `out` is full.
    pub fn push_due(&mut self, at: usize, rate: u32, playing: bool, out: &mut EventBuf) -> bool {
        for lane in self.all() {
            if lane.next(rate, playing) == Some(at) {
                if !out.push(cc(lane.controller, lane.value())) {
                    return false;
                }
                lane.sent();
            }
        }
        true
    }

    pub fn advance(&mut self, frames: usize, playing: bool) {
        self.all().for_each(|lane| lane.advance(frames, playing));
    }

    /// Last delivered values: (CC1, each macro CC).
    pub fn values(&self) -> (u8, [u8; MACROS]) {
        (
            self.cc1.current,
            self.macros.each_ref().map(|lane| lane.current),
        )
    }
}
