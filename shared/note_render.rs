//! Bounded MIDI note batches for an app-owned track scheduler.
use crate::host::ProcessorRef;

pub struct Render {
    processor: ProcessorRef,
    held: [bool; 128],
    remaining: [u64; 128],
}

impl Render {
    #[allow(dead_code)] // Source-tick collisions in editor automation.
    pub fn process_ordered(
        &mut self,
        events: &[crate::host::parameters::AutomationEvent],
        durations: &[u64; 128],
        out: &mut [f32],
        channels: usize,
    ) -> Result<(), i32> {
        use crate::host::parameters::MAX_AUTOMATION_EVENTS;
        if channels == 0 || events.len() > MAX_AUTOMATION_EVENTS {
            return Err(-7);
        }
        let mut words = [0u32; MAX_AUTOMATION_EVENTS];
        let mut count = 0;
        let mut parameters = false;
        for event in events {
            if event.kind == 0 {
                parameters = true;
                continue;
            }
            let pitch = ((event.midi >> 8) & 127) as usize;
            let on = (event.midi >> 20) & 15 == 9 && event.midi & 127 != 0;
            self.held[pitch] = on;
            self.remaining[pitch] = if on { durations[pitch] } else { 0 };
            words[count] = event.midi;
            count += 1;
        }
        let frames = (out.len() / channels) as u64;
        for pitch in 0..128 {
            if self.held[pitch] && self.remaining[pitch] != u64::MAX {
                self.remaining[pitch] = self.remaining[pitch].saturating_sub(frames);
            }
        }
        if parameters {
            self.processor.process_automation(events, out, channels)
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
        let mut events = [(0usize, 0u32); 384];
        let mut count = 0;
        let mut push = |time, word| {
            events[count] = (time, word);
            count += 1;
        };
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
        let mut words = [0u32; 768];
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
