use crate::{ffi, seq, sequence_modulation::SequenceModulation, SequencePattern, SequenceVelocity};
use std::{
    ffi::{c_char, c_void, CStr},
    time::{Duration, Instant},
};

pub(super) const RATE: u32 = 48_000;
pub(super) const BLOCK: usize = 256;
const ATTACK: usize = RATE as usize / 5;

pub(super) unsafe extern "C" fn wake(_: *mut c_void) {}

unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    let result = &mut *(user as *mut Option<Result<i32, String>>);
    *result = Some(if !error.is_null() {
        Err(CStr::from_ptr(error).to_string_lossy().into_owned())
    } else if id < 0 {
        Err("creation failed".into())
    } else {
        Ok(id)
    });
}

pub(super) struct Instance<'a> {
    host: &'a ffi::Host,
    pub id: i32,
}

impl Drop for Instance<'_> {
    fn drop(&mut self) {
        ffi::trace::mark("attack instance destroy begin");
        self.host.destroy_instance(self.id);
        ffi::trace::mark("attack instance destroy end");
    }
}

impl<'a> Instance<'a> {
    pub fn new(host: &'a ffi::Host, index: i32, state: &[u8]) -> Self {
        Self::try_new(host, index, state).unwrap()
    }

    pub fn try_new(host: &'a ffi::Host, index: i32, state: &[u8]) -> Result<Self, String> {
        let mut result: Option<Result<i32, String>> = None;
        ffi::trace::mark("attack instance create begin");
        unsafe {
            host.create_instance(
                index,
                RATE,
                BLOCK as u32,
                created,
                &mut result as *mut _ as *mut c_void,
            )
        };
        let deadline = Instant::now() + Duration::from_secs(30);
        while result.is_none() {
            assert!(Instant::now() < deadline, "creation timed out");
            host.pump_startup();
            std::thread::sleep(Duration::from_millis(1));
        }
        let instance = Self {
            host,
            id: result.unwrap()?,
        };
        ffi::trace::mark("attack instance create end; state load begin");
        host.load_state(instance.id, state)?;
        ffi::trace::mark("attack state load end");
        Ok(instance)
    }
}

pub(super) struct Player<'a> {
    host: &'a ffi::Host,
    audio: ffi::ProcessorRef,
    started: Instant,
    frames: usize,
}

impl<'a> Player<'a> {
    pub fn new(host: &'a ffi::Host, processor: &'a ffi::Processor) -> Self {
        Self {
            host,
            audio: unsafe { processor.audio_ref() },
            started: Instant::now(),
            frames: 0,
        }
    }

    fn block(&mut self, events: &[u32]) -> [f32; BLOCK * 2] {
        let mut output = [0.0; BLOCK * 2];
        assert!(self.audio.process(events, &mut output, 2));
        self.frames += BLOCK;
        self.host.pump_startup();
        let due = self.started + Duration::from_secs_f64(self.frames as f64 / f64::from(RATE));
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        output
    }

    pub fn attack(&mut self, delay_ms: u64) -> Vec<f32> {
        let delay = delay_ms as usize * 48;
        let mut sequencer = seq::Sequencer::new(RATE);
        sequencer.set_phrase(None);
        sequencer.delay_start(Duration::from_millis(delay_ms));
        let mut samples = Vec::new();
        let mut events = seq::EventBuf::new();
        let mut note_ons = Vec::new();
        for at in (0..delay + ATTACK).step_by(BLOCK) {
            events.clear();
            sequencer.set_modulation(Some(SequenceModulation::Fixed(0)));
            sequencer.render(
                SequencePattern::Steps,
                SequenceVelocity::V100,
                BLOCK,
                &mut events,
            );
            let mut offset = at;
            for &word in events.as_slice() {
                if word & 0xffff_0000 == 0x0020_0000 {
                    offset += (word & 0xffff) as usize;
                } else if word & 0xfff0_0000 == 0x2090_0000 {
                    note_ons.push((offset, word));
                }
            }
            samples.extend(self.block(events.as_slice()));
        }
        assert_eq!(note_ons, [(delay, seq::note_on(0, seq::NOTES[0], 100))]);
        events.clear();
        sequencer.render(
            SequencePattern::Off,
            SequenceVelocity::V100,
            BLOCK,
            &mut events,
        );
        self.block(events.as_slice());
        samples[delay * 2..(delay + ATTACK) * 2].to_vec()
    }

    pub fn settle(&mut self) {
        let _ = self.settle_with_tail();
    }

    pub fn settle_with_tail(&mut self) -> Vec<f32> {
        let mut samples = Vec::new();
        for _ in 0..(RATE as usize).div_ceil(BLOCK) {
            samples.extend(self.block(&[]));
        }
        samples[samples.len() - ATTACK * 2..].to_vec()
    }
}
