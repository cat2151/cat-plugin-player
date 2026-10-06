//! Opt-in startup measurements. The audio callback only publishes timestamps;
//! file I/O happens later on the UI thread. Times start at Rust main(), not at
//! process creation, and rendered samples do not measure speaker latency.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

#[derive(Clone, Copy)]
pub enum Stage {
    Main,
    AppCreation,
    HostReady,
    DeviceReady,
    HistoryRead,
    ScanRequested,
    ScanCallback,
    CatalogReady,
    LoadRequested,
    LoadReturned,
    InstanceCallback,
    InstanceHandled,
    ProcessorReady,
    StreamBuildRequested,
    StreamBuilt,
    PlayReturned,
    FirstAudioCallback,
    FirstSignalRendered,
    GuiRequested,
}

const NAMES: [&str; 19] = [
    "main",
    "app_creation",
    "host_ready",
    "device_query_finished",
    "history_read",
    "scan_requested",
    "scan_callback",
    "catalog_ready",
    "load_requested",
    "load_call_returned",
    "instance_callback",
    "instance_handled",
    "processor_ready",
    "stream_build_requested",
    "stream_built",
    "play_returned",
    "first_audio_callback",
    "first_signal_rendered",
    "gui_requested",
];

struct Trace {
    start: Instant,
    path: PathBuf,
    stamps: [AtomicU64; NAMES.len()],
    written: AtomicBool,
}

static TRACE: OnceLock<Trace> = OnceLock::new();

pub fn init(start: Instant) {
    if let Some(path) = std::env::var_os("CAT_STARTUP_TRACE") {
        let _ = TRACE.set(Trace {
            start,
            path: path.into(),
            stamps: std::array::from_fn(|_| AtomicU64::new(0)),
            written: AtomicBool::new(false),
        });
        // Store microseconds plus one, reserving zero for missing measurements.
        TRACE.get().unwrap().stamps[Stage::Main as usize].store(1, Ordering::Relaxed);
    }
}

pub fn enabled() -> bool {
    TRACE.get().is_some()
}

pub fn mark(stage: Stage) {
    if let Some(trace) = TRACE.get() {
        let stamp = &trace.stamps[stage as usize];
        if stamp.load(Ordering::Relaxed) == 0 {
            let micros = trace.start.elapsed().as_micros() as u64 + 1;
            let _ = stamp.compare_exchange(0, micros, Ordering::Relaxed, Ordering::Relaxed);
        }
    }
}

/// Called from the UI thread; also saves incomplete traces after ten seconds.
pub fn flush_if_ready() {
    let Some(trace) = TRACE.get() else { return };
    if trace.written.load(Ordering::Relaxed) {
        return;
    }
    if (trace.stamps[Stage::FirstSignalRendered as usize].load(Ordering::Relaxed) == 0
        || trace.stamps[Stage::CatalogReady as usize].load(Ordering::Relaxed) == 0)
        && trace.start.elapsed().as_secs() < 10
    {
        return;
    }
    flush();
}

pub fn flush() {
    let Some(trace) = TRACE.get() else { return };
    if trace.written.swap(true, Ordering::Relaxed) {
        return;
    }
    let mut rows: Vec<_> = NAMES
        .iter()
        .zip(&trace.stamps)
        .filter_map(|(name, stamp)| {
            let micros = stamp.load(Ordering::Relaxed);
            (micros != 0).then(|| (micros - 1, *name))
        })
        .collect();
    rows.sort_by_key(|row| row.0);
    let mut report = String::from(
        "# Times from Rust main(); excludes OS loader and physical output latency.\n\
         # First signal: rendered sample magnitude > 0.000001 (-120 dBFS).\n\
         elapsed_ms,stage\n",
    );
    for (micros, name) in rows {
        report.push_str(&format!("{:.3},{name}\n", micros as f64 / 1000.0));
    }
    for (name, stamp) in NAMES.iter().zip(&trace.stamps) {
        if stamp.load(Ordering::Relaxed) == 0 {
            report.push_str(&format!("# missing: {name}\n"));
        }
    }
    if let Err(error) = std::fs::write(&trace.path, report) {
        eprintln!(
            "could not write startup trace {}: {error}",
            trace.path.display()
        );
    }
}
