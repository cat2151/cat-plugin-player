use super::*;

#[test]
fn cursor_changes_preview_units_without_retriggering_redraws_or_same_unit_moves() {
    let mut preview = CursorPreview::default();
    let first = preview.refresh("C7 F G", 1, 48000, false).unwrap().unwrap();
    assert!(preview.refresh("C7 F G", 2, 48000, false).is_none());
    assert!(preview.refresh("C7 F G", 2, 48000, false).is_none());
    let next = preview.refresh("C7 F G", 4, 48000, false).unwrap().unwrap();
    assert_ne!(first, next);
    assert_eq!(
        preview.refresh("C7 F G", 1, 48000, false),
        Some(Some(first))
    );
    // Identical pitches at distinct positions are still separate sounding units.
    assert!(preview.refresh("c c", 1, 48000, true).unwrap().is_some());
    assert!(preview.refresh("c c", 3, 48000, false).unwrap().is_some());
    assert_eq!(preview.refresh("c r", 3, 48000, true), Some(None));
    assert!(preview.refresh("c r", 3, 48000, false).is_none());
    assert!(preview.refresh("c r", 1, 48000, false).unwrap().is_some());
    assert!(preview.refresh("d r", 1, 48000, true).unwrap().is_some());
}
fn pitches(notes: Notes) -> Vec<usize> {
    notes
        .pitches
        .iter()
        .enumerate()
        .filter(|(_, n)| **n != 0)
        .map(|(p, _)| p)
        .collect()
}
#[test]
fn context_length_accidental_and_mid_phrase_edits_match_confirmed_units() {
    for text in ["o5 l8 c", "o5 l8 c+", "o5 l8 'ceg'", "o5 l8 c4."] {
        let preview = at_cursor(text, text.chars().count(), 48000).unwrap();
        let phrase = crate::timed_sequence::Phrase::parse(text, 48000).unwrap();
        let expected = phrase
            .events
            .iter()
            .filter(|(_, m)| m[0] == 0x90)
            .map(|(_, m)| m[1] as usize)
            .collect::<Vec<_>>();
        assert_eq!(pitches(preview), expected, "{text}");
        for &(on, m) in phrase.events.iter().filter(|(_, m)| m[0] == 0x90) {
            let off = phrase
                .events
                .iter()
                .find(|(_, off)| off[0] == 0x80 && off[1] == m[1])
                .unwrap()
                .0;
            assert_eq!(preview.pitches[m[1] as usize] >> 8, off - on);
        }
    }
    assert_eq!(pitches(at_cursor("o5 l8 c+4 d", 8, 48000).unwrap()), [61]);
    assert!(at_cursor("o5 l8 cr", 8, 48000).is_none());
    assert!(at_cursor("o5 l8 c o6", 10, 48000).is_none());
    assert!(at_cursor("", 0, 48000).is_none());
}
#[test]
fn chord_input_auditions_only_the_cursor_chord() {
    let text = "C F G";
    let phrase = crate::timed_sequence::Phrase::parse(text, 48000).unwrap();
    let onsets = phrase
        .events
        .iter()
        .filter(|(_, m)| m[0] == 0x90)
        .map(|(at, _)| *at)
        .collect::<std::collections::BTreeSet<_>>();
    for (cursor, at) in [1, 3, 5].into_iter().zip(onsets) {
        let expected = phrase
            .events
            .iter()
            .filter(|(t, m)| *t == at && m[0] == 0x90)
            .map(|(_, m)| m[1] as usize)
            .collect::<Vec<_>>();
        assert_eq!(pitches(at_cursor(text, cursor, 48000).unwrap()), expected);
    }
}
#[test]
fn multiline_prefix_carries_confirmed_octave_and_length() {
    let text = "o5 l8 c\nd";
    let notes = at_cursor(text, text.chars().count(), 48000).unwrap();
    let phrase = crate::timed_sequence::Phrase::parse(text, 48000).unwrap();
    let last = phrase
        .events
        .iter()
        .rev()
        .find(|(_, m)| m[0] == 0x90)
        .unwrap();
    assert_eq!(pitches(notes), [last.1[1] as usize]);
}

#[test]
fn cursor_track_is_selected_even_when_another_track_ends_later() {
    let text = "o3 c1 d1; o5 l8 e";
    let notes = at_cursor(text, text.chars().count(), 48000).unwrap();
    let phrase = crate::timed_sequence::Phrase::parse(text, 48000).unwrap();
    let pitch = 64;
    assert_eq!(pitches(notes), [pitch]);
    let on = phrase
        .events
        .iter()
        .find(|(_, m)| m[0] == 0x90 && m[1] as usize == pitch)
        .unwrap()
        .0;
    let off = phrase
        .events
        .iter()
        .find(|(_, m)| m[0] == 0x80 && m[1] as usize == pitch)
        .unwrap()
        .0;
    assert_eq!(notes.pitches[pitch] >> 8, off - on);
}
