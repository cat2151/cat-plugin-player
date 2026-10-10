//! UI-side cursor extraction through the same cmrt converter as confirmed playback.
use crate::preview::Notes;

#[derive(Clone, Default)]
pub(crate) struct CursorPreview {
    cursor: Option<usize>,
    unit: Option<Box<(std::ops::Range<usize>, Notes)>>,
}

impl CursorPreview {
    pub fn refresh(
        &mut self,
        input: &str,
        cursor: usize,
        rate: u32,
        changed: bool,
    ) -> Option<Option<Notes>> {
        if !changed && self.cursor == Some(cursor) {
            return None;
        }
        self.cursor = Some(cursor);
        let unit = unit_at_cursor(input, cursor, rate).map(Box::new);
        if unit == self.unit {
            return None;
        }
        self.unit = unit;
        Some(self.unit.as_ref().map(|unit| unit.1))
    }
}

#[cfg(test)]
pub(crate) fn at_cursor(input: &str, cursor_chars: usize, rate: u32) -> Option<Notes> {
    unit_at_cursor(input, cursor_chars, rate).map(|(_, notes)| notes)
}

fn unit_at_cursor(
    input: &str,
    cursor_chars: usize,
    rate: u32,
) -> Option<(std::ops::Range<usize>, Notes)> {
    let byte = input
        .char_indices()
        .nth(cursor_chars)
        .map_or(input.len(), |(i, _)| i);
    let span = cmrt_chord::cursor_sounding_unit(input, byte)?;
    let performance = cmrt_chord::timed_performance(&input[..span.end]).ok()?;
    // Prefixes keep multiline and track directives in the exact confirmed context.
    // Other tracks can end later than this unit: remove previously present note ons
    // instead of simply selecting the globally last onset in the merged timeline.
    let mut prior = cmrt_chord::timed_performance(&input[..span.start])
        .ok()
        .map(|p| {
            p.events
                .into_iter()
                .filter(|e| e.message[0] == 0x90)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let added = performance
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.message[0] == 0x90)
        .filter_map(|(i, e)| {
            if !performance.from_chord {
                if let Some(index) = prior
                    .iter()
                    .position(|old| old.seconds == e.seconds && old.message == e.message)
                {
                    prior.remove(index);
                    return None;
                }
            }
            Some(i)
        })
        .collect::<Vec<_>>();
    let onset = *added.last()?;
    let at = performance.events[onset].seconds;
    let mut notes = Notes::default();
    for index in added
        .into_iter()
        .filter(|&i| performance.events[i].seconds == at)
    {
        let event = &performance.events[index];
        let pitch = event.message[1] as usize;
        let end = performance.events[index + 1..]
            .iter()
            .find(|off| off.message[0] == 0x80 && off.message[1] == event.message[1])?;
        let gate = ((end.seconds - at) * f64::from(rate)).round().max(1.0) as u64;
        notes.pitches[pitch] = (gate.min(Notes::MAX_GATE) << 8) | u64::from(event.message[2]);
    }
    Some((span, notes))
}

#[cfg(test)]
#[path = "mml_preview_tests.rs"]
mod tests;
