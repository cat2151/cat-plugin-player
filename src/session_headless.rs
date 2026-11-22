//! Explicit native-test sink: retains the transaction's processor, never opens a device.
use crate::{config::PluginKey, ffi, macro_parameters, seq, SequencePattern, SequenceVelocity};
use std::{cell::RefCell, time::Instant};

#[derive(Default)]
struct Sink {
    enabled: bool,
    processor: Option<ffi::Processor>,
    bindings: macro_parameters::Bindings,
    sequencer: Option<seq::Sequencer>,
    spans: Vec<(&'static str, f64)>,
}
thread_local! { static SINK: RefCell<Sink> = RefCell::new(Sink::default()); }

pub(crate) fn enabled() -> bool {
    SINK.with_borrow(|s| s.enabled)
}
pub(crate) fn enable() {
    SINK.with_borrow_mut(|s| s.enabled = true);
}
pub(crate) struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        disable();
    }
}
pub(crate) fn clear() {
    SINK.with_borrow_mut(|s| {
        if let Some(processor) = &s.processor {
            // Match Voice's explicit release of held notes before route destruction.
            let mut audio = unsafe { processor.audio_ref() };
            for channel in 0..16 {
                let offs: [u32; 128] =
                    std::array::from_fn(|note| seq::note_off(channel, note as u8));
                assert!(audio.process(&offs, &mut [0.0; 2], 2));
            }
        }
        s.processor = None;
        s.sequencer = None;
    });
}
pub(crate) fn disable() {
    clear();
    SINK.with_borrow_mut(|s| s.enabled = false);
}
pub(crate) fn install(
    processor: ffi::Processor,
    bindings: macro_parameters::Bindings,
    plugin: &PluginKey,
    rate: u32,
) {
    let mut sequencer = seq::Sequencer::new(rate);
    sequencer.delay_start(crate::plugin_specific::automatic_note_start_delay_at_rate(
        plugin, rate,
    ));
    SINK.with_borrow_mut(|s| {
        s.processor = Some(processor);
        s.bindings = bindings;
        s.sequencer = Some(sequencer);
    });
}
pub(crate) fn render() -> Vec<f32> {
    SINK.with_borrow_mut(|s| {
        let mut events = seq::EventBuf::new();
        s.sequencer.as_mut().unwrap().render(
            SequencePattern::Steps,
            SequenceVelocity::V100,
            1024,
            &mut events,
        );
        let mut audio = unsafe { s.processor.as_ref().unwrap().audio_ref() };
        let levels = s.sequencer.as_ref().unwrap().macro_parameters();
        macro_parameters::send(
            &mut audio,
            &s.bindings,
            levels,
            &mut [Default::default(); 8],
        )
        .unwrap();
        let mut output = vec![0.0; 2048];
        assert!(audio.process(events.as_slice(), &mut output, 2));
        output
    })
}
pub(crate) struct Span(Option<(&'static str, Instant)>);
pub(crate) fn span(name: &'static str) -> Span {
    Span(enabled().then(|| (name, Instant::now())))
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some((name, start)) = self.0 {
            SINK.with_borrow_mut(|s| s.spans.push((name, start.elapsed().as_secs_f64() * 1000.0)));
        }
    }
}
pub(crate) fn spans() -> Vec<(&'static str, f64)> {
    SINK.with_borrow_mut(|s| std::mem::take(&mut s.spans))
}

static PREPARATIONS: std::sync::Mutex<Vec<f64>> = std::sync::Mutex::new(Vec::new());
pub(crate) struct PreparationSpan(Option<Instant>);
pub(crate) fn preparation_span() -> PreparationSpan {
    PreparationSpan(std::env::var_os("PATCH_LATENCY_JSON").map(|_| Instant::now()))
}
impl Drop for PreparationSpan {
    fn drop(&mut self) {
        if let Some(start) = self.0 {
            PREPARATIONS
                .lock()
                .unwrap()
                .push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
}
pub(crate) fn preparations() -> Vec<f64> {
    std::mem::take(&mut *PREPARATIONS.lock().unwrap())
}
