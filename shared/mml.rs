//! Shared MML/chord conversion, cursor audition, and transient input UI.
#[path = "mml_preview.rs"]
mod preview;
pub use preview::CursorPreview;
#[path = "mml_widget.rs"]
mod widget;
pub use widget::{editor, Action};

/// Resolve chord notation with the same rules as the player, then generate SMF.
pub fn to_smf(input: &str) -> Result<(Vec<u8>, bool), String> {
    if input.trim().is_empty() {
        return Err("MMLを入力してください".into());
    }
    let resolved = cmrt_chord::resolve_chord_or_mml(input);
    let bytes = mmlabc_to_smf::mml_to_smf_bytes(&resolved.mml)
        .map_err(|error| format!("MML変換に失敗しました: {error}"))?;
    Ok((bytes, resolved.from_chord))
}
pub fn performance(input: &str) -> Result<cmrt_chord::TimedPerformance, String> {
    let (bytes, from_chord) = to_smf(input)?;
    let mut performance = cmrt_chord::timed_smf_notes(&bytes)?;
    performance.from_chord = from_chord;
    Ok(performance)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Notes {
    // High bits: gate in samples; low byte: velocity (zero means absent).
    pub pitches: [u64; 128],
}
impl Notes {
    pub const MAX_GATE: u64 = u64::MAX >> 8;
}
impl Default for Notes {
    fn default() -> Self {
        Self { pitches: [0; 128] }
    }
}
#[derive(Clone, Debug)]
pub struct Phrase {
    pub events: Vec<(u64, [u8; 3])>,
    pub duration: u64,
    pub notes: usize,
}
impl Phrase {
    /// Conversion and allocation happen before building the audio callback.
    pub fn parse(input: &str, rate: u32) -> Result<Self, String> {
        let performance = performance(input)?;
        let duration = (performance.duration_seconds * f64::from(rate)).round() as u64;
        if duration == 0 {
            return Err("Phrase is shorter than one sample".into());
        }
        let events: Vec<_> = performance
            .events
            .into_iter()
            .map(|e| ((e.seconds * f64::from(rate)).round() as u64, e.message))
            .collect();
        let notes = events.iter().filter(|(_, m)| m[0] == 0x90).count();
        Ok(Self {
            events,
            duration,
            notes,
        })
    }
}
