//! Opt-in UI parameter discovery and allocation-free resolved audio batches.
use super::{Host, Processor, ProcessorRef, UhHost, UhProcessor};
use std::ffi::{c_char, CStr};

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub stable_id: String,
    pub name: String,
    pub path: String,
    pub minimum: f64,
    pub maximum: f64,
    pub current: f64,
    pub normalized: bool,
    pub eligible: bool,
    /// Instance-local; never serialize this token or its index.
    pub binding: ParameterValue,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParameterValue {
    generation: u64,
    revision: u64,
    index: u32,
    value: f64,
}
impl ParameterValue {
    pub fn value(self) -> f64 {
        self.value
    }
    pub fn with_value(self, value: f64) -> Self {
        Self { value, ..self }
    }
}

pub const MAX_AUTOMATION_EVENTS: usize = 512;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AutomationEvent {
    pub parameter: ParameterValue,
    pub midi: u32,
    pub kind: u32,
}
impl AutomationEvent {
    pub fn parameter(value: ParameterValue) -> Self {
        Self {
            parameter: value,
            midi: 0,
            kind: 0,
        }
    }
    pub fn note(midi: u32) -> Self {
        Self {
            parameter: Default::default(),
            midi,
            kind: 1,
        }
    }
}

#[repr(C)]
struct Info {
    generation: u64,
    revision: u64,
    index: u32,
    eligible: u32,
    normalized: u32,
    minimum: f64,
    maximum: f64,
    current: f64,
    stable_id: [c_char; 256],
    name: [c_char; 1024],
    path: [c_char; 2048],
}
unsafe extern "C" {
    fn uh_parameter_count(host: *mut UhHost, id: i32) -> i32;
    fn uh_parameter_info(host: *mut UhHost, id: i32, index: u32, info: *mut Info) -> i32;
    fn uh_parameter_text(
        host: *mut UhHost,
        id: i32,
        value: *const ParameterValue,
        text: *mut c_char,
        size: i32,
    ) -> i32;
    fn uh_processor_prepare_parameters(processor: *mut UhProcessor) -> i32;
    fn uh_processor_parameter_batch(
        processor: *mut UhProcessor,
        values: *const ParameterValue,
        count: u32,
    ) -> i32;
    fn uh_processor_process_automation(
        processor: *mut UhProcessor,
        events: *const AutomationEvent,
        count: u32,
        out: *mut f32,
        channels: i32,
        frames: i32,
    ) -> i32;
}
impl Host {
    pub fn parameters(&self, id: i32) -> Result<Vec<Parameter>, String> {
        let count = unsafe { uh_parameter_count(self.raw, id) };
        if count < 0 {
            return Err(format!("Parameter discovery failed ({count})"));
        }
        let mut parameters = Vec::new();
        for index in 0..count as u32 {
            let mut info = std::mem::MaybeUninit::<Info>::uninit();
            let code = unsafe { uh_parameter_info(self.raw, id, index, info.as_mut_ptr()) };
            if code != 0 {
                return Err(format!("Parameter {index} discovery failed ({code})"));
            }
            let info = unsafe { info.assume_init() };
            let string = |bytes: &[c_char]| {
                // C ABI guarantees NUL termination on success.
                unsafe { CStr::from_ptr(bytes.as_ptr()) }
                    .to_string_lossy()
                    .into_owned()
            };
            let binding = ParameterValue {
                generation: info.generation,
                revision: info.revision,
                index: info.index,
                value: info.current,
            };
            if parameters.first().is_some_and(|p: &Parameter| {
                p.binding.generation != binding.generation || p.binding.revision != binding.revision
            }) {
                return Err("Parameter list changed during discovery".into());
            }
            parameters.push(Parameter {
                stable_id: string(&info.stable_id),
                name: string(&info.name),
                path: string(&info.path),
                minimum: info.minimum,
                maximum: info.maximum,
                current: info.current,
                eligible: info.eligible != 0,
                normalized: info.normalized != 0,
                binding,
            });
        }
        Ok(parameters)
    }

    /// Plugin formatting is UI-only. Empty means use the native numeric value.
    pub fn parameter_text(&self, id: i32, value: ParameterValue) -> Result<String, String> {
        let mut text = [0; 2048];
        let code = unsafe {
            uh_parameter_text(self.raw, id, &value, text.as_mut_ptr(), text.len() as i32)
        };
        if code != 0 {
            return Err(format!("Parameter display failed ({code})"));
        }
        Ok(unsafe { CStr::from_ptr(text.as_ptr()) }
            .to_string_lossy()
            .into_owned())
    }
}
impl Processor {
    /// Call on the UI thread with every audio renderer stopped.
    pub fn prepare_parameters(&self) -> Result<(), String> {
        let code = unsafe { uh_processor_prepare_parameters(self.raw) };
        if code == 0 {
            Ok(())
        } else {
            Err(format!("Parameter preparation failed ({code})"))
        }
    }
}
impl ProcessorRef {
    pub fn process_automation(
        &mut self,
        events: &[AutomationEvent],
        out: &mut [f32],
        channels: usize,
    ) -> Result<(), i32> {
        if events.len() > MAX_AUTOMATION_EVENTS
            || channels == 0
            || !out.len().is_multiple_of(channels)
        {
            out.fill(0.0);
            return Err(-7);
        }
        let code = unsafe {
            uh_processor_process_automation(
                self.raw,
                events.as_ptr(),
                events.len() as u32,
                out.as_mut_ptr(),
                channels as i32,
                (out.len() / channels) as i32,
            )
        };
        if code == 0 {
            Ok(())
        } else {
            out.fill(0.0);
            Err(code)
        }
    }
    /// No allocation, discovery or formatting. Must immediately precede process.
    pub fn parameters(&mut self, values: &[ParameterValue]) -> Result<(), i32> {
        if values.len() > 256 {
            return Err(-7);
        }
        let code =
            unsafe { uh_processor_parameter_batch(self.raw, values.as_ptr(), values.len() as u32) };
        if code == 0 {
            Ok(())
        } else {
            Err(code)
        }
    }
}
