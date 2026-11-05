//! Kit auditions timed entirely in samples with fixed callback storage.
use crate::{
    preview::Notes,
    seq::{jr_timestamp, note_off, note_on, EventBuf},
};

const VELOCITIES: [u8; 8] = [40, 64, 80, 100, 127, 100, 80, 64];

pub(crate) struct Player {
    pending: Option<Notes>,
    requested: Notes,
    sounding: [u64; 128],
    at: u64,
    latest: Option<u8>,
    velocity: u8,
    comparison_step: usize,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            pending: None,
            requested: Notes::default(),
            sounding: [0; 128],
            at: 0,
            latest: None,
            velocity: 0,
            comparison_step: 0,
        }
    }
}

impl Player {
    pub fn replace(&mut self, notes: Notes) {
        self.pending = Some(notes);
    }
    pub fn active(&self) -> bool {
        self.pending.is_some()
            || self.requested.pitches.iter().any(|&p| p != 0)
            || self.sounding.iter().any(|&p| p != 0)
    }
    pub fn current_note(&self) -> Option<u8> {
        self.latest
            .filter(|&note| self.sounding[note as usize] != 0)
    }
    pub fn current_velocity(&self) -> u8 {
        self.current_note().map_or(0, |_| self.velocity)
    }
    pub fn render(&mut self, rate: u32, frames: usize, out: &mut EventBuf) {
        if let Some(notes) = self.pending {
            for (note, until) in self.sounding.iter_mut().enumerate() {
                if *until != 0 {
                    if !out.push(note_off(0, note as u8)) {
                        return;
                    }
                    *until = 0;
                }
            }
            self.latest = None;
            self.requested = notes;
            self.comparison_step = 0;
            self.at = 0;
            self.pending = None;
        }
        let end = self.at + frames as u64;
        let mut cursor = self.at;
        loop {
            let on = self
                .requested
                .pitches
                .iter()
                .filter(|&&n| n != 0)
                .map(|n| (n >> 8) - 1)
                .min();
            let off = self.sounding.iter().copied().filter(|&n| n != 0).min();
            let next = on.into_iter().chain(off).min();
            let Some(next) = next.filter(|&n| n < end) else {
                break;
            };
            let next = next.max(cursor);
            // Reserve note-offs for retriggers too, before changing any state.
            let count = self
                .sounding
                .iter()
                .filter(|&&n| n != 0 && n <= next)
                .count()
                + self
                    .requested
                    .pitches
                    .iter()
                    .filter(|&&n| n != 0 && (n >> 8) - 1 <= next)
                    .count()
                + self
                    .requested
                    .pitches
                    .iter()
                    .enumerate()
                    .filter(|&(note, &n)| {
                        n != 0 && (n >> 8) - 1 <= next && self.sounding[note] > next
                    })
                    .count();
            if out.remaining() < count + usize::from(next > cursor) {
                break;
            }
            if next > cursor {
                out.push(jr_timestamp((next - cursor) as usize));
            }
            cursor = next;
            for (note, until) in self.sounding.iter_mut().enumerate() {
                if *until != 0 && *until <= next {
                    out.push(note_off(0, note as u8));
                    *until = 0;
                }
            }
            for (note, value) in self.requested.pitches.iter_mut().enumerate() {
                if *value != 0 && (*value >> 8) - 1 <= next {
                    if self.sounding[note] != 0 {
                        out.push(note_off(0, note as u8));
                    }
                    let comparing = *value as u8 & 128 != 0;
                    let velocity = if comparing {
                        VELOCITIES[self.comparison_step]
                    } else {
                        *value as u8 & 127
                    };
                    out.push(note_on(0, note as u8, velocity));
                    self.latest = Some(note as u8);
                    self.velocity = velocity;
                    self.sounding[note] = next + 2 * u64::from(rate);
                    *value = if comparing {
                        self.comparison_step = (self.comparison_step + 1) % VELOCITIES.len();
                        ((next + u64::from(rate) / 4 + 1) << 8) | 128
                    } else {
                        0
                    };
                }
            }
        }
        self.at = end;
    }
}

/// Mailbox payload: start sample + 1 above the low byte; low 7 bits are velocity.
/// Bit 7 selects the repeating velocity pattern; its step lives in the player.
pub(crate) fn program(notes: &[u8], rate: u32) -> Notes {
    let mut unique = [false; 128];
    for &note in notes {
        if note < 128 {
            unique[note as usize] = true;
        }
    }
    let mut result = Notes::default();
    let mut index = 0;
    for (note, &present) in unique.iter().enumerate() {
        if present {
            result.pitches[note] = ((index * u64::from(rate) / 4 + 1) << 8) | 127;
            index += 1;
        }
    }
    result
}

pub(crate) fn comparison(note: u8) -> Notes {
    let mut result = Notes::default();
    if note < 128 {
        result.pitches[note as usize] = (1 << 8) | 128;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ascending_once_quarter_second_and_two_second_gate_at_multiple_rates() {
        for rate in [44100, 48000] {
            let mut player = Player::default();
            player.replace(program(&[127, 0, 64, 0], rate));
            let mut out = EventBuf::new();
            let mut events = Vec::new();
            for block in 0..(rate as usize * 4).div_ceil(137) {
                out.clear();
                player.render(rate, 137, &mut out);
                let mut at = block * 137;
                for &word in out.as_slice() {
                    if word >> 20 == 2 {
                        at += (word & 0xffff) as usize;
                    } else {
                        events.push((at, word));
                    }
                }
            }
            let quarter = rate as usize / 4;
            assert_eq!(
                events,
                vec![
                    (0, note_on(0, 0, 127)),
                    (quarter, note_on(0, 64, 127)),
                    (rate as usize / 2, note_on(0, 127, 127)),
                    (rate as usize * 2, note_off(0, 0)),
                    (quarter + rate as usize * 2, note_off(0, 64)),
                    (rate as usize * 5 / 2, note_off(0, 127))
                ]
            );
            assert!(!player.active());
        }
    }
    #[test]
    fn live_note_tracks_actual_events_including_zero_and_127_and_clears_after_gate() {
        let mut player = Player::default();
        let mut out = EventBuf::new();
        player.replace(program(&[0, 127], 48000));
        assert_eq!(player.current_note(), None);
        player.render(48000, 12000, &mut out);
        assert_eq!(player.current_note(), Some(0));
        out.clear();
        player.render(48000, 1, &mut out);
        assert_eq!(player.current_note(), Some(127));
        out.clear();
        player.render(48000, 96000, &mut out);
        assert_eq!(player.current_note(), None);
        player.replace(program(&[70], 48000));
        out.clear();
        player.render(48000, 64, &mut out);
        assert_eq!(player.current_note(), Some(70));
        player.replace(Notes::default());
        out.clear();
        player.render(48000, 64, &mut out);
        assert_eq!(player.current_note(), None);
    }

    #[test]
    fn replacement_and_stop_release_old_notes_and_drop_future_hits() {
        let mut player = Player::default();
        let mut out = EventBuf::new();
        player.replace(program(&[0, 127], 48000));
        player.render(48000, 64, &mut out);
        out.clear();
        player.replace(program(&[64], 48000));
        player.render(48000, 64, &mut out);
        assert_eq!(out.as_slice(), &[note_off(0, 0), note_on(0, 64, 127)]);
        out.clear();
        player.replace(Notes::default());
        player.render(48000, 48000, &mut out);
        assert_eq!(out.as_slice(), &[note_off(0, 64)]);
        assert!(!player.active());
    }

    #[test]
    fn comparison_loops_without_repeating_endpoints_at_quarter_second_boundaries() {
        for rate in [44100, 48000] {
            for frames in [137, 1024] {
                let mut player = Player::default();
                player.replace(comparison(70));
                let mut out = EventBuf::new();
                let mut hits = Vec::new();
                let mut sounding = [false; 128];
                let duration = rate as usize * 9 / 2;
                for start in (0..duration).step_by(frames) {
                    out.clear();
                    player.render(rate, frames.min(duration - start), &mut out);
                    let mut at = start;
                    for &word in out.as_slice() {
                        if word >> 20 == 2 {
                            at += (word & 0xffff) as usize;
                        } else {
                            let note = (word >> 8 & 127) as usize;
                            if word >> 20 & 15 == 9 {
                                assert!(!sounding[note], "retrigger must release the previous hit");
                                sounding[note] = true;
                                hits.push((at, note, word & 127));
                            } else if word >> 20 & 15 == 8 {
                                assert!(sounding[note]);
                                sounding[note] = false;
                            }
                        }
                    }
                }
                assert_eq!(
                    hits,
                    [40, 64, 80, 100, 127, 100, 80, 64]
                        .into_iter()
                        .cycle()
                        .take(18)
                        .enumerate()
                        .map(|(index, velocity)| (index * rate as usize / 4, 70, velocity))
                        .collect::<Vec<_>>()
                );
                assert!(player.active());
                assert_eq!(player.current_note(), Some(70));
                assert_eq!(player.current_velocity(), 64);
            }
        }
    }

    #[test]
    fn stopping_comparison_cancels_remaining_hits() {
        let mut player = Player::default();
        let mut out = EventBuf::new();
        player.replace(comparison(60));
        player.render(48000, 64, &mut out);
        assert_eq!(player.current_velocity(), 40);
        out.clear();
        player.replace(Notes::default());
        player.render(48000, 48000 * 5, &mut out);
        assert_eq!(out.as_slice(), &[note_off(0, 60)]);
        assert!(!player.active());
        assert_eq!(player.current_note(), None);
        assert_eq!(player.current_velocity(), 0);
    }

    #[test]
    fn selecting_another_note_or_normal_preview_replaces_the_loop() {
        let mut player = Player::default();
        let mut out = EventBuf::new();
        player.replace(comparison(69));
        player.render(48000, 12001, &mut out);
        assert_eq!(player.current_velocity(), 64);
        out.clear();
        player.replace(comparison(70));
        player.render(48000, 1, &mut out);
        assert_eq!(out.as_slice(), &[note_off(0, 69), note_on(0, 70, 40)]);
        out.clear();
        player.replace(program(&[71], 48000));
        player.render(48000, 1, &mut out);
        assert_eq!(out.as_slice(), &[note_off(0, 70), note_on(0, 71, 127)]);
    }
}
