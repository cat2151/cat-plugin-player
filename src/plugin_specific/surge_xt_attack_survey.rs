use super::{corpus, envelope, render, DELAYS};
use crate::{config, config::PluginKey, ffi};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write, path::Path};

#[path = "surge_xt_attack_statistics.rs"]
mod statistics;
#[path = "surge_xt_attack_survey_tests.rs"]
mod tests;

const SEED: u64 = 0x5355_5247_4553_5552;

#[derive(Clone, Deserialize, Serialize)]
struct Observation {
    group: String,
    repeat: usize,
    order: usize,
    pre_note_frames: usize,
    before_rms: Option<Vec<f64>>,
    attack_rms: Vec<f64>,
    first_signal_frame: Option<usize>,
    energy: f64,
    peak: f64,
}

#[derive(Deserialize, Serialize)]
struct Input {
    patch: String,
    plugin: PluginKey,
    state_sha256: String,
    state_file: String,
}

#[derive(Deserialize, Serialize)]
struct Row {
    patch: String,
    state_sha256: String,
    audio_file: String,
    observations: Vec<Observation>,
    #[serde(default)]
    error: Option<String>,
}

fn write_json(path: &Path, value: &impl Serialize) {
    let temporary = path.with_extension("json.next");
    let mut file = File::create(&temporary).unwrap();
    file.write_all(&serde_json::to_vec_pretty(value).unwrap())
        .unwrap();
    file.sync_all().unwrap();
    drop(file);
    std::fs::rename(temporary, path).unwrap();
}

fn inputs(root: &Path) -> Vec<Input> {
    let path = root.join("inputs.json");
    if path.exists() {
        return serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    }
    let patches = corpus::load(&config::path().unwrap());
    assert_eq!(patches.len(), 175);
    let original: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.parent().unwrap().join("initial-survey-report.json")).unwrap(),
    )
    .unwrap();
    let expected = original["inputs"].as_array().unwrap();
    assert_eq!(expected.len(), patches.len());
    let mut inputs = Vec::new();
    for (number, (patch, expected)) in patches.into_iter().zip(expected).enumerate() {
        let sha = corpus::digest(&patch.state);
        assert_eq!(expected["patch"], patch.label);
        assert_eq!(expected["format"], patch.plugin.format);
        assert_eq!(expected["state_sha256"], sha);
        let state_file = format!("state-{number:03}.bin");
        let state_path = root.join(&state_file);
        if state_path.exists() {
            assert_eq!(std::fs::read(state_path).unwrap(), patch.state);
        } else {
            let mut file = File::create_new(state_path).unwrap();
            file.write_all(&patch.state).unwrap();
            file.sync_all().unwrap();
        }
        inputs.push(Input {
            patch: patch.label,
            plugin: patch.plugin,
            state_sha256: sha,
            state_file,
        });
    }
    write_json(&path, &inputs);
    inputs
}

fn schedule(number: usize) -> Vec<(usize, String)> {
    let mut rng = StdRng::seed_from_u64(SEED + number as u64);
    let mut result = Vec::new();
    for repeat in 1..=30 {
        let mut groups = ["reference_a", "reference_b"];
        groups.shuffle(&mut rng);
        result.extend(groups.map(|g| (repeat, g.into())));
    }
    for delay in DELAYS {
        result.extend((1..=30).map(|repeat| (repeat, format!("candidate_{delay}"))));
    }
    result
}

fn group_count(row: &Row, name: &str) -> usize {
    row.observations.iter().filter(|o| o.group == name).count()
}

fn classification(row: &Row) -> (String, Option<u64>) {
    if row.error.is_some() {
        return ("measurement_error".into(), None);
    }
    if ["reference_a", "reference_b"]
        .iter()
        .any(|name| group_count(row, name) != 30)
    {
        return ("unmeasured".into(), None);
    }
    match statistics::baseline(&row.observations) {
        "excluded_silent" => ("excluded_silent".into(), None),
        "equivalent" => {
            for delay in DELAYS {
                match statistics::candidate(&row.observations, delay) {
                    "equivalent" => return ("evaluable".into(), Some(delay)),
                    "unmeasured" => return ("unmeasured".into(), None),
                    _ => {}
                }
            }
            ("evaluable".into(), None)
        }
        reason => (format!("excluded_baseline:{reason}"), None),
    }
}

fn validate(row: &Row, input: &Input, root: &Path, number: usize, reused: bool) {
    assert_eq!(row.patch, input.patch);
    assert_eq!(row.state_sha256, input.state_sha256);
    assert_eq!(row.audio_file, format!("attack-{number:03}.f32le"));
    assert_eq!(
        std::fs::metadata(root.join(&row.audio_file)).unwrap().len(),
        row.observations.len() as u64 * 76800
    );
    let expected = schedule(number);
    let extension: Vec<_> = expected
        .iter()
        .filter(|(_, name)| {
            !matches!(
                name.as_str(),
                "reference_a" | "reference_b" | "candidate_0" | "candidate_200"
            )
        })
        .collect();
    if reused {
        let pilot: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.parent().unwrap().join("method-pilot-v1/pilot.json")).unwrap(),
        )
        .unwrap();
        let original = pilot["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["patch"] == input.patch)
            .unwrap();
        assert_eq!(original["state_sha256"], input.state_sha256);
        assert!(row.observations.len() >= 120);
        assert_eq!(
            serde_json::to_value(&row.observations[..120]).unwrap(),
            original["observations"]
        );
        let pcm = std::fs::read(root.join(&row.audio_file)).unwrap();
        let old = std::fs::read(
            root.parent()
                .unwrap()
                .join("method-pilot-v1")
                .join(original["audio_file"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(&pcm[..old.len()], old);
    }
    for (order, o) in row.observations.iter().enumerate() {
        assert_eq!(o.order, order);
        assert_eq!(o.attack_rms.len(), 40);
        assert!((1..=30).contains(&o.repeat));
        assert!(o.attack_rms.iter().all(|v| v.is_finite() && *v >= 0.0));
        if !reused {
            assert_eq!(
                (o.repeat, o.group.as_str()),
                (expected[order].0, expected[order].1.as_str())
            );
        } else if order >= 120 {
            assert_eq!(
                (o.repeat, o.group.as_str()),
                (extension[order - 120].0, extension[order - 120].1.as_str())
            );
        }
        assert_eq!(
            row.observations
                .iter()
                .filter(|x| x.group == o.group && x.repeat == o.repeat)
                .count(),
            1
        );
    }
}

fn import_pilot(root: &Path, inputs: &[Input]) {
    let marker = root.join("pilot-import.json");
    if marker.exists() {
        return;
    }
    let pilot_root = root.parent().unwrap().join("method-pilot-v1");
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(pilot_root.join("pilot.json")).unwrap()).unwrap();
    assert_eq!(value["method"], "surge-attack-pilot-v1");
    let mut imported = Vec::new();
    for value in value["rows"].as_array().unwrap() {
        let mut row: Row = serde_json::from_value(value.clone()).unwrap();
        let number = inputs
            .iter()
            .position(|p| p.patch == row.patch && p.state_sha256 == row.state_sha256)
            .unwrap();
        assert_eq!(row.observations.len(), 120);
        let source = pilot_root.join(&row.audio_file);
        row.audio_file = format!("attack-{number:03}.f32le");
        let row_path = root.join(format!("row-{number:03}.json"));
        if row_path.exists() {
            let saved: Row = serde_json::from_slice(&std::fs::read(&row_path).unwrap()).unwrap();
            validate(&saved, &inputs[number], root, number, true);
            assert_eq!(saved.observations.len(), 120);
            imported.push(number);
            continue;
        }
        let destination = root.join(&row.audio_file);
        if destination.exists() {
            assert_eq!(
                std::fs::read(&destination).unwrap(),
                std::fs::read(&source).unwrap()
            );
        } else {
            std::fs::copy(source, destination).unwrap();
        }
        validate(&row, &inputs[number], root, number, true);
        write_json(&row_path, &row);
        imported.push(number);
    }
    assert_eq!(imported.len(), 7);
    write_json(&marker, &imported);
}

fn record(
    host: &ffi::Host,
    index: i32,
    state: &[u8],
    group: &str,
    repeat: usize,
    order: usize,
) -> Result<(Observation, Vec<u8>), String> {
    let instance = render::Instance::try_new(host, index, state)?;
    let processor = host
        .create_chain(instance.id, &[], false, render::RATE, render::BLOCK as u32)
        .ok_or_else(|| "host.create_chain failed".to_owned())?;
    let mut player = render::Player::new(host, &processor);
    let reference = group.starts_with("reference_");
    let before_rms = reference.then(|| envelope::rms(&player.settle_with_tail()));
    let delay = if reference {
        0
    } else {
        group
            .strip_prefix("candidate_")
            .unwrap()
            .parse::<u64>()
            .unwrap()
    };
    let attack = player.attack(delay);
    let o = Observation {
        group: group.into(),
        repeat,
        order,
        pre_note_frames: if reference {
            48128
        } else {
            delay as usize * 48
        },
        before_rms,
        attack_rms: envelope::rms(&attack),
        first_signal_frame: attack
            .as_chunks::<2>()
            .0
            .iter()
            .position(|s| s.iter().any(|x| x.abs() > 1e-6)),
        energy: attack.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>() / 2.0,
        peak: attack
            .iter()
            .map(|&x| f64::from(x.abs()))
            .fold(0.0, f64::max),
    };
    Ok((o, attack.iter().flat_map(|x| x.to_le_bytes()).collect()))
}

#[test]
#[ignore = "full approved 175-input fresh-instance survey; read-only user data, JSON/PCM in global work; no audio device"]
fn native_surge_attack_full_survey() {
    assert!(statistics::failure_bound() <= 0.05);
    let root =
        std::env::var_os("SURGE_ATTACK_SURVEY_DIR").expect("SURGE_ATTACK_SURVEY_DIR required");
    let root = Path::new(&root);
    assert!(root.is_dir());
    let contract = serde_json::json!({"method":"surge-attack-full-v1", "seed":SEED,
        "sample_rate":48000,"block_frames":256,"note":48,"velocity":100,"cc1":0,
        "reference_settle_frames":48128,"attack_ms":200,"window_ms":5,"repeats":30,
        "median_order":[5,26],"family":360,"limit_db":1,"delays":DELAYS,
        "failure_bound":statistics::failure_bound()});
    let contract_path = root.join("contract.json");
    if contract_path.exists() {
        let actual: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&contract_path).unwrap()).unwrap();
        assert_eq!(actual, contract);
    } else {
        write_json(&contract_path, &contract);
    }
    let inputs = inputs(root);
    assert_eq!(inputs.len(), 175);
    import_pilot(root, &inputs);
    let imported: Vec<usize> =
        serde_json::from_slice(&std::fs::read(root.join("pilot-import.json")).unwrap()).unwrap();
    let _dll = crate::native_library::load().unwrap();
    let host = unsafe { ffi::Host::new(render::wake, std::ptr::null_mut()) }.unwrap();
    let mut summary = Vec::new();
    for (number, input) in inputs.iter().enumerate() {
        let state = std::fs::read(root.join(&input.state_file)).unwrap();
        assert_eq!(corpus::digest(&state), input.state_sha256);
        let path = root.join(format!("row-{number:03}.json"));
        let mut row = if path.exists() {
            serde_json::from_slice::<Row>(&std::fs::read(&path).unwrap()).unwrap()
        } else {
            File::create_new(root.join(format!("attack-{number:03}.f32le"))).unwrap();
            Row {
                patch: input.patch.clone(),
                state_sha256: input.state_sha256.clone(),
                audio_file: format!("attack-{number:03}.f32le"),
                observations: Vec::new(),
                error: None,
            }
        };
        validate(&row, input, root, number, imported.contains(&number));
        if classification(&row).0 == "unmeasured" {
            match host.restore_plugin(&input.plugin) {
                None => {
                    row.error =
                        Some("host.restore_plugin could not restore the frozen plugin key".into());
                    write_json(&path, &row);
                }
                Some(info) => {
                    let mut pcm = std::fs::OpenOptions::new()
                        .append(true)
                        .open(root.join(&row.audio_file))
                        .unwrap();
                    for (repeat, group) in schedule(number) {
                        if classification(&row).0 != "unmeasured" {
                            break;
                        }
                        if row
                            .observations
                            .iter()
                            .any(|o| o.repeat == repeat && o.group == group)
                        {
                            continue;
                        }
                        ffi::trace::mark(&format!(
                            "survey patch={number} group={group} repeat={repeat}"
                        ));
                        let observation = record(
                            &host,
                            info.index,
                            &state,
                            &group,
                            repeat,
                            row.observations.len(),
                        );
                        let (o, bytes) = match observation {
                            Ok(value) => value,
                            Err(error) => {
                                row.error = Some(format!("{group} repeat={repeat}: {error}"));
                                write_json(&path, &row);
                                break;
                            }
                        };
                        pcm.write_all(&bytes).unwrap();
                        pcm.sync_all().unwrap();
                        row.observations.push(o);
                        write_json(&path, &row);
                    }
                }
            }
        }
        let (class, minimum) = classification(&row);
        let baseline =
            if group_count(&row, "reference_a") == 30 && group_count(&row, "reference_b") == 30 {
                statistics::baseline(&row.observations)
            } else {
                "unavailable"
            };
        let candidates: Vec<_> = DELAYS.iter().map(|&delay| serde_json::json!({"delay_ms":delay,
            "status":if baseline == "equivalent" { statistics::candidate(&row.observations, delay) } else { "baseline_excluded" },
            "measured_trials":group_count(&row, &format!("candidate_{delay}"))})).collect();
        eprintln!(
            "survey {}/175 | {} | {} | minimum={minimum:?} | trials={}",
            number + 1,
            input.patch,
            class,
            row.observations.len()
        );
        summary.push(
            serde_json::json!({"patch":input.patch,"format":input.plugin.format,
            "state_sha256":input.state_sha256,"classification":class,"baseline":baseline,
            "minimum_ms":minimum,"candidates":candidates,"error":row.error}),
        );
        write_json(&root.join("summary.json"), &summary);
    }
    assert_eq!(summary.len(), 175);
    assert!(summary.iter().all(|r| r["classification"] != "unmeasured"));
}
