use super::*;
use crate::{seq::Sequencer, SequencePattern, SequenceVelocity};
fn notes(pitch: usize, gate: u64) -> Notes {
    let mut notes = Notes::default();
    notes.pitches[pitch] = (gate << 8) | 96;
    notes
}
fn render(seq: &mut Sequencer, pattern: SequencePattern, frames: usize) -> Vec<u32> {
    let mut out = EventBuf::new();
    seq.render(pattern, SequenceVelocity::V100, frames, &mut out);
    out.as_slice().to_vec()
}
#[test]
fn latest_mailbox_request_replaces_fast_input_without_replaying_older_requests() {
    let mailbox = Mailbox::default();
    let mut seen = 0;
    assert!(mailbox.poll(&mut seen).is_none());
    for pitch in [60, 62, 64] {
        mailbox.publish(Some(notes(pitch, 10)));
    }
    assert_eq!(mailbox.poll(&mut seen), Some(notes(64, 10)));
    assert!(mailbox.poll(&mut seen).is_none());
    mailbox.publish(None);
    assert_eq!(mailbox.poll(&mut seen), Some(Notes::default()));
}
#[test]
fn gate_and_replacement_release_notes_across_blocks_and_stop_stays_stopped() {
    let mut seq = Sequencer::new(48000);
    seq.preview(notes(60, 100));
    assert_eq!(
        render(&mut seq, SequencePattern::Off, 64),
        [note_on(0, 60, 96)]
    );
    assert_eq!(
        render(&mut seq, SequencePattern::Off, 64),
        [jr_timestamp(36), note_off(0, 60)]
    );
    assert!(render(&mut seq, SequencePattern::Off, 64)
        .iter()
        .all(|w| w & 0xfff0_0000 != 0x2090_0000));
    seq.preview(notes(60, 100));
    render(&mut seq, SequencePattern::Off, 1);
    seq.preview(notes(62, 100));
    assert_eq!(
        render(&mut seq, SequencePattern::Off, 1),
        [note_off(0, 60), note_on(0, 62, 96)]
    );
    seq.preview(Notes::default());
    assert_eq!(render(&mut seq, SequencePattern::Off, 1), [note_off(0, 62)]);
}
#[test]
fn preview_releases_normal_loop_suppresses_it_and_then_resumes_transport() {
    for pattern in [SequencePattern::Steps, SequencePattern::Custom] {
        let mut seq = Sequencer::new(48000);
        seq.set_phrase(Some(std::sync::Arc::new(
            crate::timed_sequence::Phrase::parse("c", 48000).unwrap(),
        )));
        let start = render(&mut seq, pattern, 1);
        let pitch = ((start
            .iter()
            .find(|&&w| w & 0xfff0_0000 == 0x2090_0000)
            .unwrap()
            >> 8)
            & 127) as u8;
        seq.preview(notes(80, 10));
        let events = render(&mut seq, pattern, 1);
        assert!(events.contains(&note_off(0, pitch)));
        assert!(events.contains(&note_on(0, 80, 96)));
        assert!(render(&mut seq, pattern, 5).is_empty());
        assert_eq!(
            render(&mut seq, pattern, 5),
            [jr_timestamp(4), note_off(0, 80)]
        );
        assert!(render(&mut seq, pattern, 1)
            .iter()
            .any(|w| w & 0xfff0_0000 == 0x2090_0000));
    }
}
#[test]
fn saturated_buffer_preserves_pending_releases_and_replacement() {
    let mut player = Player::default();
    let mut out = EventBuf::new();
    player.replace(notes(60, 100));
    player.render(1, &mut out);
    out.clear();
    for _ in 0..256 {
        out.push(0);
    }
    player.replace(notes(62, 100));
    player.render(1, &mut out);
    assert!(player.active());
    out.clear();
    player.render(1, &mut out);
    assert_eq!(out.as_slice(), [note_off(0, 60), note_on(0, 62, 96)]);
}

#[test]
fn concurrent_mailbox_snapshots_never_mix_chord_requests() {
    let mailbox = std::sync::Arc::new(Mailbox::default());
    let writer = mailbox.clone();
    let handle = std::thread::spawn(move || {
        for i in 0..2000 {
            let mut chord = notes(60, 100 + i);
            chord.pitches[64] = ((100 + i) << 8) | 96;
            writer.publish(Some(chord));
        }
    });
    let mut seen = 0;
    for _ in 0..5000 {
        if let Some(notes) = mailbox.poll(&mut seen) {
            assert_eq!(notes.pitches[60], notes.pitches[64]);
        }
    }
    handle.join().unwrap();
    if let Some(notes) = mailbox.poll(&mut seen) {
        assert_eq!(notes.pitches[60], notes.pitches[64]);
    }
}
