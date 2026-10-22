use crate::{config, ffi};
use serde::Serialize;
use std::collections::BTreeMap;

#[path = "surge_xt_attack_corpus.rs"]
mod corpus;
#[path = "surge_xt_attack_envelope.rs"]
mod envelope;
#[path = "surge_xt_attack_pilot.rs"]
mod pilot;
#[path = "surge_xt_attack_render.rs"]
mod render;
#[path = "surge_xt_attack_repeats.rs"]
mod repeats;
#[path = "surge_xt_attack_survey.rs"]
mod survey;

const DELAYS: [u64; 7] = [0, 10, 25, 50, 100, 150, 200];

#[derive(Serialize)]
struct Trial {
    first_rms: Vec<f64>,
    reference_rms: Vec<f64>,
    comparison: envelope::Comparison,
    settled_repeat: Option<envelope::Comparison>,
    settled_repeat_rms: Option<Vec<f64>>,
}

#[derive(Serialize)]
struct Candidate {
    delay_ms: u64,
    trials: Vec<Trial>,
}

#[derive(Serialize)]
struct Row {
    patch: String,
    format: String,
    state_sha256: String,
    minimum_ms: Option<u64>,
    candidates: Vec<Candidate>,
}

fn measure(host: &ffi::Host, index: i32, state: &[u8], delay: u64) -> Trial {
    let instance = render::Instance::new(host, index, state);
    let processor = host
        .create_chain(instance.id, &[], false, render::RATE, render::BLOCK as u32)
        .unwrap();
    let mut player = render::Player::new(host, &processor);
    let first_rms = envelope::rms(&player.attack(delay));
    player.settle();
    let reference_rms = envelope::rms(&player.attack(0));
    let comparison = envelope::compare(&first_rms, &reference_rms);
    let settled_repeat_rms = if delay == 200 && !comparison.matched {
        player.settle();
        Some(envelope::rms(&player.attack(0)))
    } else {
        None
    };
    let settled_repeat = settled_repeat_rms
        .as_ref()
        .map(|rms| envelope::compare(&reference_rms, rms));
    Trial {
        first_rms,
        reference_rms,
        comparison,
        settled_repeat,
        settled_repeat_rms,
    }
}

fn save(rows: &[Row], inputs: &[serde_json::Value]) {
    if let Some(path) = std::env::var_os("SURGE_ATTACK_REPORT") {
        let report = serde_json::json!({
            "sample_rate": render::RATE, "block_frames": render::BLOCK,
            "seed": corpus::SEED, "note": 48, "velocity": 100, "cc1": 0,
            "window_ms": 5, "attack_ms": 200, "limit_db": 1,
            "inputs": inputs, "rows": rows,
            "unmeasured": inputs.len() - rows.len(),
        });
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "requires installed Surge XT, user favorites/history and a saved patch catalog; no audio device"]
fn native_surge_attack_settling_measurement() {
    let _dll = crate::native_library::load().unwrap();
    let config = config::path().unwrap();
    let patches = corpus::load(&config);
    let inputs: Vec<_> = patches
        .iter()
        .map(|patch| {
            serde_json::json!({
                "patch": patch.label, "format": patch.plugin.format,
                "state_sha256": corpus::digest(&patch.state),
            })
        })
        .collect();
    let mut rows = Vec::new();
    save(&rows, &inputs);
    eprintln!(
        "attack corpus: {} patches, seed={:#x}; 48kHz/256, note=48 velocity=100 CC1=0",
        patches.len(),
        corpus::SEED
    );
    let host = unsafe { ffi::Host::new(render::wake, std::ptr::null_mut()) }.unwrap();
    for (number, patch) in patches.iter().enumerate() {
        let info = host.restore_plugin(&patch.plugin).unwrap();
        let mut row = Row {
            patch: patch.label.clone(),
            format: patch.plugin.format.clone(),
            state_sha256: corpus::digest(&patch.state),
            minimum_ms: None,
            candidates: Vec::new(),
        };
        for delay_ms in DELAYS {
            let mut candidate = Candidate {
                delay_ms,
                trials: Vec::new(),
            };
            for _ in 0..3 {
                let trial = measure(&host, info.index, &patch.state, delay_ms);
                let matched = trial.comparison.matched;
                candidate.trials.push(trial);
                // A failed trial already rules out 3/3 at this candidate.
                if !matched {
                    break;
                }
            }
            let passed = candidate.trials.len() == 3
                && candidate
                    .trials
                    .iter()
                    .all(|trial| trial.comparison.matched);
            row.candidates.push(candidate);
            if passed {
                row.minimum_ms = Some(delay_ms);
                break;
            }
        }
        eprintln!(
            "attack {}/{} | {} | {} | minimum={:?}",
            number + 1,
            patches.len(),
            row.format,
            row.patch,
            row.minimum_ms
        );
        rows.push(row);
        save(&rows, &inputs);
        if rows.last().unwrap().minimum_ms.is_none() {
            break;
        }
    }
    eprintln!("| format | patch | minimum delay (3/3) |");
    eprintln!("|---|---|---|");
    let mut distribution = BTreeMap::new();
    for row in &rows {
        let minimum = row
            .minimum_ms
            .map_or("NO MATCH <=200ms".into(), |ms| format!("{ms}ms"));
        eprintln!("| {} | {} | {} |", row.format, row.patch, minimum);
        *distribution.entry(minimum).or_insert(0) += 1;
    }
    eprintln!("attack distribution: {distribution:?}");
    eprintln!("unmeasured after stop gate: {}", patches.len() - rows.len());
    assert!(rows.iter().all(|row| row.minimum_ms.is_some()),
        "NEEDS-HUMAN: no 3/3 match at <=200ms; inspect first/reference and settled-repeat comparisons; keep production delay unchanged");
}
