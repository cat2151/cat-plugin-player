//! Bounded MIDI note batches for an app-owned track scheduler.
use crate::host::ProcessorRef;

pub struct Render {
    processor: ProcessorRef,
    held: [bool; 128],
    remaining: [u64; 128],
    scheduled: [[u16; 128]; 16],
    reset_events: bool,
    ordered_controls: bool,
}

impl Render {
    /// Opt in only after native ordered-event preparation with stopped renderers.
    pub fn set_ordered_controls(&mut self, enabled: bool) {
        self.ordered_controls = enabled;
    }
    /// Queue note-offs before the next real audio block, without advancing time.
    pub fn reset_events(&mut self) {
        self.reset_events = true;
    }

    fn scheduled_offs(&mut self, mut push: impl FnMut(u32) -> bool) -> bool {
        for channel in 0..16 {
            for pitch in 0..128 {
                while self.scheduled[channel][pitch] > 0 {
                    if !push(0x2080_0000 | (channel as u32) << 16 | (pitch as u32) << 8) {
                        return false;
                    }
                    self.scheduled[channel][pitch] -= 1;
                }
            }
        }
        true
    }
    #[allow(dead_code)] // Editor sequence playback uses explicit source events.
    pub fn process_events(
        &mut self,
        events: &[crate::host::parameters::AutomationEvent],
        out: &mut [f32],
        channels: usize,
    ) -> Result<(), i32> {
        use crate::host::parameters::MAX_AUTOMATION_EVENTS;
        if channels == 0 || events.len() > MAX_AUTOMATION_EVENTS {
            return Err(-7);
        }
        let pending = if self.reset_events {
            self.scheduled
                .iter()
                .flatten()
                .map(|&count| usize::from(count))
                .sum::<usize>()
                + self.held.iter().filter(|&&held| held).count()
        } else {
            0
        };
        if pending + events.len() > MAX_AUTOMATION_EVENTS {
            return Err(-7);
        }
        let mut words = [0u32; MAX_AUTOMATION_EVENTS];
        let mut count = 0;
        let mut parameters = false;
        if self.reset_events {
            if !self.scheduled_offs(|word| {
                if count == words.len() {
                    return false;
                }
                words[count] = word;
                count += 1;
                true
            }) {
                return Err(-7);
            }
            for pitch in 0..128 {
                if self.held[pitch] {
                    if count == words.len() {
                        return Err(-7);
                    }
                    words[count] = 0x2080_0000 | (pitch as u32) << 8;
                    count += 1;
                    self.held[pitch] = false;
                    self.remaining[pitch] = 0;
                }
            }
            self.reset_events = false;
        }
        let prefix = count;
        for event in events {
            if event.kind == 0 {
                parameters = true;
                continue;
            }
            let pitch = ((event.midi >> 8) & 127) as usize;
            let channel = ((event.midi >> 16) & 15) as usize;
            let on = (event.midi >> 20) & 15 == 9 && event.midi & 127 != 0;
            parameters |= self.ordered_controls && matches!((event.midi >> 20) & 15, 11 | 14);
            if matches!((event.midi >> 20) & 15, 8 | 9) {
                let held = &mut self.scheduled[channel][pitch];
                *held = if on {
                    held.checked_add(1).ok_or(-7)?
                } else {
                    held.saturating_sub(1)
                };
            }
            if count == words.len() {
                return Err(-7);
            }
            words[count] = event.midi;
            count += 1;
        }
        if parameters {
            if prefix == 0 {
                self.processor.process_automation(events, out, channels)
            } else {
                if prefix + events.len() > MAX_AUTOMATION_EVENTS {
                    return Err(-7);
                }
                let mut combined =
                    [crate::host::parameters::AutomationEvent::note(0); MAX_AUTOMATION_EVENTS];
                for (target, word) in combined.iter_mut().zip(&words[..prefix]) {
                    *target = crate::host::parameters::AutomationEvent::note(*word);
                }
                combined[prefix..prefix + events.len()].copy_from_slice(events);
                self.processor
                    .process_automation(&combined[..prefix + events.len()], out, channels)
            }
        } else if self.processor.process(&words[..count], out, channels) {
            Ok(())
        } else {
            Err(-1)
        }
    }
    #[allow(dead_code)] // Editor automation uses the shared note renderer.
    pub fn parameters(
        &mut self,
        values: &[crate::host::parameters::ParameterValue],
    ) -> Result<(), i32> {
        self.processor.parameters(values)
    }
    pub fn new(processor: ProcessorRef) -> Self {
        Self {
            processor,
            held: [false; 128],
            remaining: [0; 128],
            scheduled: [[0; 128]; 16],
            reset_events: false,
            ordered_controls: false,
        }
    }

    pub fn process(
        &mut self,
        velocities: &[u8; 128],
        durations: &[u64; 128],
        changed: i32,
        out: &mut [f32],
        channels: usize,
    ) -> bool {
        if channels == 0 || out.len() / channels > 1024 {
            return false;
        }
        let frames = out.len() / channels;
        let mut events = [(0usize, 0u32); 4096];
        let mut count = 0;
        if changed == 1 || changed == 3 || self.reset_events {
            if !self.scheduled_offs(|word| {
                if count == events.len() - 384 {
                    return false;
                }
                events[count] = (0, word);
                count += 1;
                true
            }) {
                return false;
            }
            self.reset_events = false;
        }
        let mut push = |time, word| {
            events[count] = (time, word);
            count += 1;
        };
        // End old voices before starting any replacement, independent of pitch.
        for (pitch, &velocity) in velocities.iter().enumerate() {
            if self.held[pitch]
                && ((changed != 0 && !(changed == 2 && velocity == 0))
                    || self.remaining[pitch] == 0)
            {
                push(0, 0x2080_0000 | (pitch as u32) << 8);
                self.held[pitch] = false;
            }
        }
        for pitch in 0..128 {
            if changed != 0 && !(changed == 2 && velocities[pitch] == 0) {
                if self.held[pitch] {
                    push(0, 0x2080_0000 | (pitch as u32) << 8);
                }
                self.held[pitch] = changed != 3 && velocities[pitch] != 0;
                self.remaining[pitch] = durations[pitch];
                if self.held[pitch] {
                    push(
                        0,
                        0x2090_0000 | (pitch as u32) << 8 | u32::from(velocities[pitch]),
                    );
                }
            }
            if self.held[pitch] {
                if self.remaining[pitch] < frames as u64 {
                    push(
                        self.remaining[pitch] as usize,
                        0x2080_0000 | (pitch as u32) << 8,
                    );
                    self.held[pitch] = false;
                } else if self.remaining[pitch] != u64::MAX {
                    self.remaining[pitch] -= frames as u64;
                }
            }
        }
        // Stable insertion sort preserves off/on ordering at the same frame.
        for i in 1..count {
            let e = events[i];
            let mut j = i;
            while j > 0 && events[j - 1].0 > e.0 {
                events[j] = events[j - 1];
                j -= 1;
            }
            events[j] = e;
        }
        let mut words = [0u32; 8192];
        let mut len = 0;
        let mut previous = 0;
        for &(time, event) in &events[..count] {
            if time != previous {
                words[len] = 0x0020_0000 | (time - previous) as u32;
                len += 1;
            }
            words[len] = event;
            len += 1;
            previous = time;
        }
        self.processor.process(&words[..len], out, channels)
    }
}
