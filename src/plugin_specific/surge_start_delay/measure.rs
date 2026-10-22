use super::*;
use clap::{Args, Subcommand};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Instant,
};
mod observation;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Measure Surge XT CLAP catalog presets without opening GUI/audio devices.
    MeasureSurgeStartDelay(Options),
}

#[derive(Debug, Args)]
pub(crate) struct Options {
    #[arg(long)]
    output: PathBuf,
    /// Reclassify a previous complete table without rendering again.
    #[arg(long, conflicts_with_all = ["candidate_wait_ms", "reference_wait_ms", "onset_ratio", "limit", "jobs", "timeout_seconds", "work_dir"])]
    reclassify: Option<PathBuf>,
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(0..=2000))]
    candidate_wait_ms: u32,
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(0..=2000))]
    reference_wait_ms: u32,
    #[arg(long, default_value_t = 1.0, value_parser = nonnegative)]
    max_onset_delta_ms: f64,
    #[arg(long, default_value_t = 0.01, value_parser = ratio)]
    onset_ratio: f64,
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u16).range(1..=16))]
    jobs: u16,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=600))]
    timeout_seconds: u64,
    /// Limit the catalog prefix for a smoke run; omitted means every preset.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    limit: Option<u32>,
    /// Parent for a new run directory containing states, WAVs and subprocess logs.
    #[arg(long)]
    work_dir: Option<PathBuf>,
}

fn nonnegative(s: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or("Expected finite nonnegative value".into())
}
fn ratio(s: &str) -> Result<f64, String> {
    nonnegative(s).and_then(|v| {
        if v > 0.0 && v <= 1.0 {
            Ok(v)
        } else {
            Err("Expected ratio in (0, 1]".into())
        }
    })
}

pub(crate) fn run(command: &Command) -> Result<(), String> {
    let Command::MeasureSurgeStartDelay(args) = command;
    let start = Instant::now();
    let mut table = if let Some(input) = &args.reclassify {
        Table::parse(&std::fs::read(input).map_err(|e| e.to_string())?)?
    } else {
        survey(args)?
    };
    table.select(args.max_onset_delta_ms)?;
    let bytes = serde_json::to_vec_pretty(&table).map_err(|e| e.to_string())?;
    crate::state_store::atomic_write(&args.output, &bytes)?;
    println!(
        "{}",
        serde_json::json!({"output":args.output,"total":table.entries.len(),
        "zero_wait_allowed":table.entries.values().filter(|e|e.zero_wait_allowed).count(),
        "silent":table.entries.values().filter(|e|e.outcome==Outcome::Silent).count(),
        "errors":table.entries.values().filter(|e|e.outcome==Outcome::Error).count(),
        "seconds":start.elapsed().as_secs_f64()})
    );
    Ok(())
}

fn survey(args: &Options) -> Result<Table, String> {
    let mut catalog: Vec<_> = crate::patch_catalog::load()?
        .into_iter()
        .filter(|c| {
            c.format == "CLAP"
                && c.plugin_id == ID
                && (c.display.starts_with("patches_factory/")
                    || c.display.starts_with("patches_3rdparty/"))
        })
        .collect();
    catalog.sort_by(|a, b| a.display.cmp(&b.display));
    if let Some(limit) = args.limit {
        catalog.truncate(limit as usize);
    }
    let first = catalog.first().ok_or("No Surge XT CLAP catalog presets")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let plugin_sha256 = hash(&std::fs::read(&first.bundle_path).map_err(|e| e.to_string())?);
    let renderer_sha256 = hash(&std::fs::read(&exe).map_err(|e| e.to_string())?);
    let run = format!(
        "run-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    );
    let parent = match &args.work_dir {
        Some(path) => path.clone(),
        None => crate::native_library::cache_root()?.join("cache/surge-start-delay-measurements"),
    };
    let work = parent.join(run);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    eprintln!(
        "Measuring {} presets; artifacts: {}",
        catalog.len(),
        work.display()
    );
    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel();
    let entries = std::thread::scope(|scope| {
        for _ in 0..args.jobs {
            let (tx, catalog, exe, work, next) = (tx.clone(), &catalog, &exe, &work, &next);
            scope.spawn(move || {
                while let Some(c) = catalog.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let entry = observation::measure(args, c, exe, work);
                    if tx.send((c.display.clone(), entry)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        let mut entries = BTreeMap::new();
        for (path, entry) in rx {
            entries.insert(path, entry);
            if entries.len() % 50 == 0 || entries.len() == catalog.len() {
                eprintln!("{}/{} measured", entries.len(), catalog.len());
            }
        }
        entries
    });
    if entries.len() != catalog.len() {
        return Err("Incomplete survey; previous output retained".into());
    }
    Ok(Table {
        format_version: 1,
        plugin_format: "CLAP".into(),
        plugin_id: ID.into(),
        measurement: Measurement {
            candidate_wait_ms: args.candidate_wait_ms,
            reference_wait_ms: args.reference_wait_ms,
            capture_ms: 200,
            sample_rate: 48000,
            note: 48,
            velocity: 100,
            cc1: 0,
            onset_ratio: args.onset_ratio,
            onset_window_frames: 48,
            trials_per_wait: 1,
            plugin_sha256,
            renderer_sha256,
        },
        max_onset_delta_ms: args.max_onset_delta_ms,
        entries,
    })
}
