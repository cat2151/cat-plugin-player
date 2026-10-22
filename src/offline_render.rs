//! Fresh-process, unpaced instrument rendering. Never opens the app or an audio device.
use crate::{config::PluginKey, ffi, seq, SequenceModulation, SequencePattern, SequenceVelocity};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    ffi::{c_char, c_void},
    path::PathBuf,
    time::{Duration, Instant},
};

mod output;
#[cfg(test)]
mod tests;

const RATE: u32 = 48_000;
const BLOCK: usize = 256;
const CAPTURE: usize = 9_600;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Render note48/velocity100/CC1=0 to a new float WAV and .json sidecar, then exit.
    #[arg(long, requires_all = ["plugin_format", "plugin_id", "plugin_path", "state_file", "wait_ms"],
        conflicts_with_all = ["restore_state", "patch_roots", "resident", "select_smf"])]
    pub render_wav: Option<PathBuf>,
    #[arg(long, requires = "render_wav", value_parser = ["CLAP", "VST3"])]
    pub plugin_format: Option<String>,
    #[arg(long, requires = "render_wav")]
    pub plugin_id: Option<String>,
    #[arg(long, requires = "render_wav")]
    pub plugin_path: Option<PathBuf>,
    #[arg(long, requires = "render_wav")]
    pub state_file: Option<PathBuf>,
    /// Empty-event sample time before note-on, processed without clock synchronization.
    #[arg(long, requires = "render_wav")]
    pub wait_ms: Option<u32>,
}

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    let result = &mut *(user as *mut Option<Result<i32, String>>);
    *result = Some(match ffi::error_string(error) {
        Some(error) => Err(error),
        None if id < 0 => Err("Instance creation failed".into()),
        None => Ok(id),
    });
}

struct Instance<'a>(&'a ffi::Host, i32);
impl Drop for Instance<'_> {
    fn drop(&mut self) {
        self.0.destroy_instance(self.1);
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Capture {
    pcm: Vec<f32>,
    processed: usize,
    empty_ms: f64,
    sounding_ms: f64,
}

// A single processing callback allows timing/event-position tests without native plugins.
fn capture(
    wait_ms: u32,
    mut process: impl FnMut(&[u32], &mut [f32]) -> Result<(), String>,
) -> Result<Capture, String> {
    let delay = usize::try_from(u64::from(wait_ms) * 48).map_err(|_| "Wait too large")?;
    let end = delay.checked_add(CAPTURE).ok_or("Wait too large")?;
    let mut sequencer = seq::Sequencer::new(RATE);
    sequencer.set_phrase(None);
    sequencer.delay_start(Duration::from_millis(u64::from(wait_ms)));
    sequencer.set_modulation(Some(SequenceModulation::Fixed(0)));
    let mut events = seq::EventBuf::new();
    let mut pcm = Vec::with_capacity(CAPTURE * 2);
    let mut note_ons = Vec::new();
    let mut empty_ms = 0.0;
    let mut sounding_ms = 0.0;
    let mut block = [0.0; BLOCK * 2];
    for at in (0..end).step_by(BLOCK) {
        events.clear();
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
            } else if at + BLOCK <= delay {
                return Err("Unexpected event in empty wait".into());
            }
        }
        block.fill(0.0);
        let started = Instant::now();
        process(events.as_slice(), &mut block)?;
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        if at + BLOCK <= delay {
            empty_ms += elapsed;
        } else {
            sounding_ms += elapsed;
        }
        let first = delay.saturating_sub(at).min(BLOCK);
        let last = (end - at).min(BLOCK);
        if first < last {
            pcm.extend_from_slice(&block[first * 2..last * 2]);
        }
    }
    if note_ons != [(delay, seq::note_on(0, 48, 100))] || pcm.len() != CAPTURE * 2 {
        return Err(format!(
            "Invalid note position or capture length: {note_ons:?}/{}",
            pcm.len()
        ));
    }
    if pcm.iter().any(|s| !s.is_finite()) {
        return Err("Non-finite PCM".into());
    }
    Ok(Capture {
        pcm,
        processed: end.div_ceil(BLOCK) * BLOCK,
        empty_ms,
        sounding_ms,
    })
}

pub fn run(args: &Args) -> Result<(), String> {
    let started = Instant::now();
    let path = args.render_wav.as_ref().ok_or("Missing WAV path")?;
    let mut output = output::Output::new(path)?;
    let result = render(args, started);
    match result {
        Ok((capture, mut report)) => {
            let save = Instant::now();
            output.wav(&capture.pcm)?;
            report["timings_ms"]["wav_save"] = json!(save.elapsed().as_secs_f64() * 1000.0);
            report["timings_ms"]["total_before_report"] =
                json!(started.elapsed().as_secs_f64() * 1000.0);
            output.report(&report)?;
            if report["silent"] == true {
                Err("Rendered window is silent; see JSON (status=silent)".into())
            } else {
                Ok(())
            }
        }
        Err(error) => {
            output.report(
                &json!({"status":"error", "error": error, "wait_ms":args.wait_ms,
                "total_ms": started.elapsed().as_secs_f64()*1000.0}),
            )?;
            Err(error)
        }
    }
}

fn render(args: &Args, started: Instant) -> Result<(Capture, serde_json::Value), String> {
    let state_path = args.state_file.as_ref().ok_or("Missing state")?;
    let state = std::fs::read(state_path).map_err(|e| e.to_string())?;
    if state.is_empty() || state.len() > 16 * 1024 * 1024 {
        return Err("Invalid state size".into());
    }
    let wait = args.wait_ms.ok_or("Missing wait")?;
    let key = PluginKey {
        format: args.plugin_format.clone().ok_or("Missing format")?,
        id: args.plugin_id.clone().ok_or("Missing plugin id")?,
        bundle_path: args
            .plugin_path
            .as_ref()
            .ok_or("Missing plugin path")?
            .to_string_lossy()
            .into(),
        name: String::new(),
        vendor: String::new(),
    };
    let native = Instant::now();
    let _library = crate::native_library::load().map_err(|e| e.to_string())?;
    let native_ms = native.elapsed().as_secs_f64() * 1000.0;
    // Context outlives host, including cancellation/destruction on timeout.
    let mut result: Box<Option<Result<i32, String>>> = Box::new(None);
    let host_start = Instant::now();
    let host =
        unsafe { ffi::Host::new(wake, std::ptr::null_mut()) }.ok_or("Host creation failed")?;
    let plugin = host
        .restore_plugin(&key)
        .ok_or("Plugin registration failed")?;
    let host_ms = host_start.elapsed().as_secs_f64() * 1000.0;
    let creation = Instant::now();
    unsafe {
        host.create_instance(
            plugin.index,
            RATE,
            BLOCK as u32,
            created,
            &mut *result as *mut _ as *mut c_void,
        )
    };
    let mut pumps = 0;
    let mut callback_wait = Duration::ZERO;
    while result.is_none() {
        if creation.elapsed() > Duration::from_secs(30) {
            return Err("Instance creation timed out".into());
        }
        host.pump_startup();
        pumps += 1;
        if result.is_none() {
            let pause = Instant::now();
            std::thread::sleep(Duration::from_millis(1));
            callback_wait += pause.elapsed();
        }
    }
    let instance = Instance(&host, result.take().unwrap()?);
    let creation_ms = creation.elapsed().as_secs_f64() * 1000.0;
    let load = Instant::now();
    host.load_state(instance.1, &state)?;
    let load_ms = load.elapsed().as_secs_f64() * 1000.0;
    let chain_start = Instant::now();
    let processor = host
        .create_chain(instance.1, &[], false, RATE, BLOCK as u32)
        .ok_or("Processor creation failed")?;
    let chain_ms = chain_start.elapsed().as_secs_f64() * 1000.0;
    let mut audio = unsafe { processor.audio_ref() };
    let capture = capture(wait, |events, out| {
        if !audio.process(events, out, 2) {
            return Err("Audio processing failed".into());
        }
        host.pump_startup();
        Ok(())
    })?;
    drop(processor);
    drop(instance);
    drop(host);
    let bytes: Vec<u8> = capture.pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
    let silent = capture.pcm.iter().all(|s| *s == 0.0);
    let report = json!({
        "status": if silent {"silent"} else {"ok"}, "silent": silent,
        "silence_definition": "all decoded samples equal zero", "state_sha256":digest(&state),
        "state_file":state_path, "plugin":key, "wait_ms":wait,
        "sample_rate":RATE, "block_frames":BLOCK, "channels":2,
        "note":48, "velocity":100, "cc1":0, "note_on_frame":u64::from(wait)*48,
        "capture_frames":CAPTURE, "processed_frames":capture.processed,
        "pcm_sha256":digest(&bytes), "callback_pumps":pumps,
        "timings_ms": {"native_load":native_ms, "host_and_registration":host_ms,
            "instance_creation":creation_ms, "callback_wait":callback_wait.as_secs_f64()*1000.0,
            "state_load":load_ms, "chain_creation":chain_ms, "empty_blocks":capture.empty_ms,
            "note_blocks_including_boundary":capture.sounding_ms,
            "render_and_teardown":started.elapsed().as_secs_f64()*1000.0}
    });
    Ok((capture, report))
}
