//! Routed macros of the instruments whose patches save which macros they use.
#[path = "instrument_macros/floe.rs"]
pub mod floe;
#[path = "instrument_macros/six_sines.rs"]
pub mod six_sines;
#[cfg(test)]
#[path = "instrument_macros/tests.rs"]
mod tests;
#[path = "instrument_macros/vaporizer2.rs"]
mod vaporizer2;

/// One routed macro; `position` is the saved 0-1 operating position.
pub struct Macro {
    pub number: usize,
    pub position: Option<f64>,
    pub bipolar: bool,
    pub name: Option<String>,
    /// CCs the saved state maps onto the macro; only Floe saves them in its state.
    pub ccs: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instrument {
    SurgeXt,
    Vaporizer2,
    SixSines,
    Floe,
}
impl Instrument {
    pub fn of(format: &str, id: &str) -> Option<Self> {
        if crate::surge_macros::supports(format, id) {
            return Some(Self::SurgeXt);
        }
        if format != "CLAP" {
            return None;
        }
        [
            (vaporizer2::ID, Self::Vaporizer2),
            (six_sines::ID, Self::SixSines),
            (floe::ID, Self::Floe),
        ]
        .into_iter()
        .find_map(|(clap, instrument)| (clap == id).then_some(instrument))
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::SurgeXt => "Surge XT",
            Self::Vaporizer2 => "Vaporizer2",
            Self::SixSines => "Six Sines",
            Self::Floe => "Floe",
        }
    }
    /// None when the state cannot be inspected.
    pub fn routed(self, format: &str, state: &[u8]) -> Option<Vec<Macro>> {
        match self {
            Self::SurgeXt => {
                let c = crate::surge_macros::parse(state, format == "VST3")?;
                Some(
                    (0..crate::surge_macros::MACROS)
                        .filter(|&i| c.routed[i])
                        .map(|i| Macro {
                            number: i + 1,
                            position: c.positions[i],
                            bipolar: c.bipolar[i],
                            name: c.names[i].clone(),
                            ccs: Vec::new(),
                        })
                        .collect(),
                )
            }
            Self::Vaporizer2 => vaporizer2::parse(state),
            Self::SixSines => six_sines::parse(state),
            Self::Floe => floe::routed(state),
        }
    }
    /// The routed macros a parameter may move; a macro the state maps CC1 onto already follows
    /// the mod wheel, and a second writer would fight it.
    pub fn movable(self, format: &str, state: &[u8]) -> Option<Vec<Macro>> {
        let mut macros = self.routed(format, state)?;
        macros.retain(|m| !m.ccs.contains(&1));
        Some(macros)
    }
    /// The host range of the macro parameters.
    pub fn range(self) -> (f64, f64) {
        match self {
            Self::SixSines => (-1.0, 1.0),
            _ => (0.0, 1.0),
        }
    }
    /// Whether a host parameter name carries macro `number`.
    pub fn parameter(self, number: usize, name: &str) -> bool {
        match self {
            // Surge's own adapter exposes M1: <patch label> through M8: <patch label>.
            Self::SurgeXt => name.starts_with(&format!("M{number}:")),
            Self::Vaporizer2 => name == format!("Custom modulator {number}"),
            Self::SixSines => name == format!("Macro {number} Level"),
            Self::Floe => name == format!("Macro {number}"),
        }
    }
}
