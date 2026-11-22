//! Opt-in UI parameter discovery and allocation-free resolved audio batches.
use super::{Host, Processor, ProcessorRef, UhHost, UhProcessor};
use std::ffi::{c_char, CStr};
#[path = "parameter_domain.rs"]
pub mod domain;

impl Parameter {
    pub fn domain(&self) -> Result<domain::Domain, String> {
        domain::Domain::new(
            if self.normalized {
                domain::ValueSpace::Normalized
            } else {
                domain::ValueSpace::Plain
            },
            self.minimum,
            self.maximum,
        )
    }
    pub fn checked_current(&self) -> Result<domain::ApiValue, String> {
        self.domain()?
            .value(self.current)
            .map_err(|e| format!("{}: {e}", self.name))
    }
}

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

/// Discovery without a current-value read. The binding is an identity token;
/// supply a value with `with_value` before sending it to a processor.
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterMetadata {
    pub stable_id: String,
    pub name: String,
    pub path: String,
    pub minimum: f64,
    pub maximum: f64,
    pub normalized: bool,
    pub eligible: bool,
    pub binding: ParameterValue,
}

impl ParameterMetadata {
    pub fn domain(&self) -> Result<domain::Domain, String> {
        domain::Domain::new(
            if self.normalized {
                domain::ValueSpace::Normalized
            } else {
                domain::ValueSpace::Plain
            },
            self.minimum,
            self.maximum,
        )
    }
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
    fn uh_parameter_metadata(host: *mut UhHost, id: i32, index: u32, info: *mut Info) -> i32;
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
        self.discover_parameters(id, true).map(|infos| {
            infos
                .into_iter()
                .map(|info| {
                    let metadata = info.metadata();
                    Parameter {
                        stable_id: metadata.stable_id,
                        name: metadata.name,
                        path: metadata.path,
                        minimum: metadata.minimum,
                        maximum: metadata.maximum,
                        current: info.current,
                        normalized: metadata.normalized,
                        eligible: metadata.eligible,
                        binding: metadata.binding.with_value(info.current),
                    }
                })
                .collect()
        })
    }

    pub fn parameter_metadata(&self, id: i32) -> Result<Vec<ParameterMetadata>, String> {
        self.discover_parameters(id, false)
            .map(|infos| infos.into_iter().map(|info| info.metadata()).collect())
    }

    fn discover_parameters(&self, id: i32, current: bool) -> Result<Vec<Info>, String> {
        let count = unsafe { uh_parameter_count(self.raw, id) };
        if count < 0 {
            return Err(format!("Parameter discovery failed ({count})"));
        }
        let mut parameters = Vec::new();
        for index in 0..count as u32 {
            let mut info = std::mem::MaybeUninit::<Info>::uninit();
            let code = unsafe {
                if current {
                    uh_parameter_info(self.raw, id, index, info.as_mut_ptr())
                } else {
                    uh_parameter_metadata(self.raw, id, index, info.as_mut_ptr())
                }
            };
            if code != 0 {
                return Err(format!("Parameter {index} discovery failed ({code})"));
            }
            let info = unsafe { info.assume_init() };
            if parameters.first().is_some_and(|p: &Info| {
                p.generation != info.generation || p.revision != info.revision
            }) {
                return Err("Parameter list changed during discovery".into());
            }
            parameters.push(info);
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
impl Info {
    fn metadata(&self) -> ParameterMetadata {
        let string = |bytes: &[c_char]| {
            // C ABI guarantees NUL termination on success.
            unsafe { CStr::from_ptr(bytes.as_ptr()) }
                .to_string_lossy()
                .into_owned()
        };
        ParameterMetadata {
            stable_id: string(&self.stable_id),
            name: string(&self.name),
            path: string(&self.path),
            minimum: self.minimum,
            maximum: self.maximum,
            normalized: self.normalized != 0,
            eligible: self.eligible != 0,
            binding: ParameterValue {
                generation: self.generation,
                revision: self.revision,
                index: self.index,
                value: self.minimum,
            },
        }
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
