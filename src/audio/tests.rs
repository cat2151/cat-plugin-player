use super::*;

#[test]
fn rebuilding_stream_releases_notes_without_requiring_cc_support() {
    let events = shutdown_events();
    for note in crate::seq::GUITAR_NOTES.into_iter().chain([60]) {
        assert!(events[..128].contains(&note_off(0, note)));
    }
    assert_eq!(events[2048], 0x20B0_7800);
    assert_eq!(events[2063], 0x20BF_7800);
}

#[test]
fn macro_targets_survive_the_atomic_word() {
    for target in [
        MacroTarget::Cc(0),
        MacroTarget::Cc(127),
        MacroTarget::Parameter,
    ] {
        for choice in SequenceModulation::CHOICES.into_iter().flatten() {
            let lane = Some((target, choice));
            assert_eq!(decode_macro(encode_macro(lane)), lane);
        }
    }
    assert_eq!(decode_macro(encode_macro(None)), None);
}
