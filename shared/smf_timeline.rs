use crate::smf::model::Music;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

pub struct Timeline {
    pub events: Vec<(u64, u32)>,
    pub duration: u64,
}

impl Timeline {
    pub fn new(music: &Music, rate: u32) -> Result<Self, String> {
        if music.tracks.len() != 1 || rate == 0 {
            return Err("Selection input must contain one track".into());
        }
        let mut events = Vec::new();
        let mut tick = 0;
        let mut total = 0u128;
        let mut tempo = 500000u128;
        let mut duration = 0;
        for e in &music.tracks[0].events {
            let delta = e.tick.checked_sub(tick).ok_or("Events out of order")?;
            total += u128::from(delta) * tempo;
            tick = e.tick;
            let sample = if music.division & 0x8000 == 0 {
                let denominator = u128::from(music.division) * 1000000;
                if denominator == 0 {
                    return Err("Invalid division".into());
                }
                (total * u128::from(rate) + denominator / 2) / denominator
            } else {
                let fps = (music.division >> 8) as u8 as i8;
                let (n, d) = if fps == -29 {
                    (30000u128, 1001u128)
                } else if [-24, -25, -30].contains(&fps) {
                    ((-fps) as u128, 1)
                } else {
                    return Err("Invalid SMPTE division".into());
                };
                let denominator = n * u128::from(music.division & 255);
                if denominator == 0 {
                    return Err("Invalid SMPTE division".into());
                }
                (u128::from(tick) * u128::from(rate) * d + denominator / 2) / denominator
            };
            let sample = u64::try_from(sample).map_err(|_| "Timeline overflow")?;
            duration = duration.max(sample);
            if e.status == 0xff && e.meta_type == 0x51 && e.data.len() == 3 {
                tempo = (u128::from(e.data[0]) << 16)
                    | (u128::from(e.data[1]) << 8)
                    | u128::from(e.data[2]);
                if tempo == 0 {
                    return Err("Invalid zero tempo".into());
                }
            } else if (0x80..=0xef).contains(&e.status) {
                let word = 0x2000_0000
                    | (u32::from(e.status) << 16)
                    | (u32::from(*e.data.first().ok_or("Invalid MIDI event")?) << 8)
                    | u32::from(*e.data.get(1).unwrap_or(&0));
                events.push((sample, word));
            }
        }
        if duration == 0 {
            return Err("Selection SMF has no duration".into());
        }
        Ok(Self { events, duration })
    }
}

pub struct LoopPlayer {
    timeline: Arc<Timeline>,
    published: Arc<AtomicU64>,
    pub position: u64,
    index: usize,
    seed: Vec<u32>,
    held: [bool; 2048],
    released: [bool; 2048],
    sustain: [bool; 16],
    delay: u64,
    pub words: Vec<u32>,
}

impl LoopPlayer {
    pub fn new(timeline: Arc<Timeline>, published: Arc<AtomicU64>, delay: u64) -> Self {
        let position = published.load(Ordering::Acquire) % timeline.duration;
        let index = timeline.events.partition_point(|e| e.0 < position);
        let mut controllers = BTreeMap::new();
        let mut notes = [0u8; 2048];
        let mut released = [false; 2048];
        let mut sustain = [false; 16];
        for &(_, word) in &timeline.events[..index] {
            let status = ((word >> 16) & 255) as u8;
            let channel = usize::from(status & 15);
            let key = ((word >> 8) & 127) as usize;
            let value = (word & 127) as u8;
            let slot = channel * 128 + key;
            match status & 0xf0 {
                0x90 if value != 0 => {
                    notes[slot] = value;
                    released[slot] = false;
                }
                0x80 | 0x90 => {
                    released[slot] = true;
                    if !sustain[channel] {
                        notes[slot] = 0;
                    }
                }
                _ => {
                    controllers.insert((status, if status & 0xf0 <= 0xb0 { key } else { 0 }), word);
                    if status & 0xf0 == 0xb0 && key == 64 {
                        sustain[channel] = value >= 64;
                        if !sustain[channel] {
                            for n in channel * 128..(channel + 1) * 128 {
                                if released[n] {
                                    notes[n] = 0;
                                }
                            }
                        }
                    }
                    if status & 0xf0 == 0xb0 && [120, 123].contains(&key) {
                        notes[channel * 128..(channel + 1) * 128].fill(0);
                    }
                }
            }
        }
        let mut seed: Vec<_> = controllers.into_values().collect();
        for (slot, &velocity) in notes.iter().enumerate() {
            if velocity == 0 {
                continue;
            }
            let channel = (slot / 128) as u32;
            let key = (slot % 128) as u32;
            seed.push(0x2090_0000 | channel << 16 | key << 8 | u32::from(velocity));
            if released[slot] {
                seed.push(0x2080_0000 | channel << 16 | key << 8);
            }
        }
        let loops = 1024u64.div_ceil(timeline.duration) as usize + 2;
        let capacity = (timeline.events.len() + 16384) * loops * 2;
        Self {
            timeline,
            published,
            position,
            index,
            seed,
            held: notes.map(|v| v != 0),
            released,
            sustain,
            delay,
            words: Vec::with_capacity(capacity),
        }
    }

    fn track(&mut self, word: u32) {
        let status = ((word >> 16) & 255) as u8;
        let channel = usize::from(status & 15);
        let key = ((word >> 8) & 127) as usize;
        let value = (word & 127) as u8;
        let slot = channel * 128 + key;
        match status & 0xf0 {
            0x90 if value != 0 => {
                self.held[slot] = true;
                self.released[slot] = false;
            }
            0x80 | 0x90 => {
                self.released[slot] = true;
                if !self.sustain[channel] {
                    self.held[slot] = false;
                }
            }
            0xb0 if key == 64 => {
                self.sustain[channel] = value >= 64;
                if !self.sustain[channel] {
                    for n in channel * 128..(channel + 1) * 128 {
                        if self.released[n] {
                            self.held[n] = false;
                        }
                    }
                }
            }
            0xb0 if [120, 123].contains(&key) => {
                self.held[channel * 128..(channel + 1) * 128].fill(false)
            }
            _ => {}
        }
    }

    pub fn render(&mut self, frames: usize) {
        self.words.clear();
        let initial = self.delay.min(frames as u64) as usize;
        self.delay -= initial as u64;
        if initial == frames {
            return;
        }
        let mut elapsed = initial;
        let mut last = 0;
        if initial != 0 {
            self.words.push(0x0020_0000 | initial as u32);
            last = initial;
        }
        self.words.append(&mut self.seed);
        while elapsed < frames {
            let next = self
                .timeline
                .events
                .get(self.index)
                .map_or(self.timeline.duration, |e| e.0)
                .min(self.timeline.duration);
            let distance = next
                .saturating_sub(self.position)
                .min((frames - elapsed) as u64) as usize;
            elapsed += distance;
            self.position += distance as u64;
            if elapsed == frames {
                break;
            }
            if elapsed != last {
                self.words.push(0x0020_0000 | (elapsed - last) as u32);
                last = elapsed;
            }
            if self.position == self.timeline.duration {
                for (slot, held) in self.held.iter_mut().enumerate() {
                    if *held {
                        self.words.push(
                            0x2080_0000 | ((slot / 128) as u32) << 16 | ((slot % 128) as u32) << 8,
                        );
                    }
                    *held = false;
                }
                for channel in 0..16 {
                    self.words.push(0x20b0_4000 | channel << 16);
                }
                self.sustain.fill(false);
                self.released.fill(false);
                self.position = 0;
                self.index = 0;
            } else {
                while let Some(&(at, word)) = self.timeline.events.get(self.index) {
                    if at > self.position {
                        break;
                    }
                    self.track(word);
                    self.words.push(word);
                    self.index += 1;
                }
            }
        }
        self.published.store(self.position, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resumed_chain_restores_remaining_note_and_loops_naturally() {
        let timeline = Arc::new(Timeline {
            events: vec![(0, 0x2093_3c64), (800, 0x2083_3c00)],
            duration: 1000,
        });
        let position = Arc::new(AtomicU64::new(500));
        let mut player = LoopPlayer::new(timeline.clone(), position.clone(), 0);
        player.render(200);
        assert_eq!(player.position, 700);
        assert_eq!(player.words[0], 0x2093_3c64);
        let mut replacement = LoopPlayer::new(timeline, position, 100);
        replacement.render(250);
        assert_eq!(replacement.position, 850);
        assert!(replacement.words.contains(&0x2083_3c00));
        replacement.render(200);
        assert_eq!(replacement.position, 50);
        assert!(replacement.words.contains(&0x2093_3c64));
    }
}
