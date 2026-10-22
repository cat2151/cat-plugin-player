use super::{corpus, envelope, render, repeats::TARGETS};
use crate::{config, ffi};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

const REPEATS: usize = 30;
const SEED: u64 = 0x5355_5247_4550_494c;
const GROUPS: [&str; 4] = ["reference_a", "reference_b", "candidate_0", "candidate_200"];

#[derive(Deserialize, Serialize)]
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
struct Row {
    patch: String,
    state_sha256: String,
    audio_file: String,
    observations: Vec<Observation>,
}

fn schedule(rng: &mut StdRng) -> Vec<(usize, &'static str)> {
    (1..=REPEATS)
        .flat_map(|repeat| {
            let mut groups = GROUPS;
            groups.shuffle(rng);
            groups.into_iter().map(move |group| (repeat, group))
        })
        .collect()
}

fn document(rows: &[Row]) -> serde_json::Value {
    serde_json::json!({
        "method": "surge-attack-pilot-v1", "sample_rate": render::RATE,
        "block_frames": render::BLOCK, "seed": SEED,
        "note": 48, "velocity": 100, "cc1": 0, "window_ms": 5,
        "attack_ms": 200, "repeats": REPEATS, "format": "CLAP",
        "reference_settle_frames": (render::RATE as usize).div_ceil(render::BLOCK) * render::BLOCK,
        "audio_layout": "f32le stereo interleaved; one 9600-frame attack per observation, in order",
        "rows": rows,
    })
}

fn save(path: &Path, rows: &[Row]) {
    let temporary = path.with_extension("json.next");
    let mut file = File::create(&temporary).unwrap();
    file.write_all(&serde_json::to_vec_pretty(&document(rows)).unwrap())
        .unwrap();
    file.sync_all().unwrap();
    drop(file);
    std::fs::rename(temporary, path).unwrap();
}

fn resume(path: &Path) -> Vec<Row> {
    if !path.exists() {
        return Vec::new();
    }
    let mut actual: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows = actual.as_object_mut().unwrap().remove("rows").unwrap();
    let mut expected = document(&[]);
    expected.as_object_mut().unwrap().remove("rows");
    assert_eq!(actual, expected, "checkpoint method/input contract changed");
    serde_json::from_value(rows).unwrap()
}

fn validate(row: &Row, expected: &[(usize, &str)], path: &Path) {
    assert!(row.observations.len() <= expected.len());
    for (order, (observation, &(repeat, group))) in
        row.observations.iter().zip(expected).enumerate()
    {
        assert_eq!(observation.order, order);
        assert_eq!(observation.repeat, repeat);
        assert_eq!(observation.group, group);
        assert_eq!(observation.attack_rms.len(), 40);
    }
    assert_eq!(
        std::fs::metadata(path).unwrap().len(),
        row.observations.len() as u64 * 76_800,
        "checkpoint PCM and JSON do not match; preserve and investigate"
    );
}

#[test]
#[ignore = "requires seven approved Surge XT states; fresh-instance pilot, writes JSON/PCM, no audio device"]
fn native_surge_attack_fresh_instance_pilot() {
    let output = std::env::var_os("SURGE_ATTACK_PILOT_REPORT")
        .expect("SURGE_ATTACK_PILOT_REPORT must name a report in the global work directory");
    let output = Path::new(&output);
    let config = config::path().unwrap();
    let saved = corpus::load_saved(&config);
    let patches: Vec<_> = TARGETS
        .iter()
        .map(|(id, name)| {
            let label = format!("saved/{id}/{name}");
            let matches: Vec<_> = saved
                .iter()
                .filter(|patch| patch.label == label && patch.plugin.format == "CLAP")
                .collect();
            assert_eq!(matches.len(), 1, "need exactly one approved patch: {label}");
            matches[0]
        })
        .collect();
    let mut rng = StdRng::seed_from_u64(SEED);
    let schedules: Vec<_> = patches.iter().map(|_| schedule(&mut rng)).collect();
    let mut rows = resume(output);
    assert!(rows.len() <= patches.len());
    for (number, row) in rows.iter().enumerate() {
        assert_eq!(row.patch, patches[number].label);
        assert_eq!(row.state_sha256, corpus::digest(&patches[number].state));
        assert_eq!(row.audio_file, format!("pilot-{}.f32le", TARGETS[number].0));
        validate(
            row,
            &schedules[number],
            &output.parent().unwrap().join(&row.audio_file),
        );
        if number + 1 < rows.len() {
            assert_eq!(row.observations.len(), REPEATS * 4);
        }
    }
    let _dll = crate::native_library::load().unwrap();
    let host = unsafe { ffi::Host::new(render::wake, std::ptr::null_mut()) }.unwrap();
    for (number, (patch, (id, _))) in patches.iter().zip(TARGETS).enumerate() {
        if rows
            .get(number)
            .is_some_and(|row| row.observations.len() == REPEATS * 4)
        {
            continue;
        }
        let info = host.restore_plugin(&patch.plugin).unwrap();
        let audio_file = format!("pilot-{id}.f32le");
        let path = output.parent().unwrap().join(&audio_file);
        let mut pcm = if rows.get(number).is_some() {
            OpenOptions::new().append(true).open(path).unwrap()
        } else {
            let file = File::create_new(path).unwrap();
            rows.push(Row {
                patch: patch.label.clone(),
                state_sha256: corpus::digest(&patch.state),
                audio_file,
                observations: Vec::new(),
            });
            file
        };
        for &(repeat, group) in &schedules[number][rows[number].observations.len()..] {
            ffi::trace::mark(&format!(
                "pilot patch={} order={} group={group} repeat={repeat}",
                patch.label,
                rows[number].observations.len()
            ));
            let instance = render::Instance::new(&host, info.index, &patch.state);
            ffi::trace::mark("pilot chain create begin");
            let processor = host
                .create_chain(instance.id, &[], false, render::RATE, render::BLOCK as u32)
                .unwrap();
            ffi::trace::mark("pilot chain create end; pre-note processing begin");
            let mut player = render::Player::new(&host, &processor);
            let reference = group.starts_with("reference_");
            let before_rms = reference.then(|| envelope::rms(&player.settle_with_tail()));
            let delay = if group == "candidate_200" { 200 } else { 0 };
            ffi::trace::mark("pilot attack begin");
            let attack = player.attack(delay);
            ffi::trace::mark("pilot attack end");
            let bytes: Vec<_> = attack.iter().flat_map(|x| x.to_le_bytes()).collect();
            pcm.write_all(&bytes).unwrap();
            let row = rows.last_mut().unwrap();
            row.observations.push(Observation {
                group: group.to_owned(),
                repeat,
                order: row.observations.len(),
                pre_note_frames: if reference {
                    48_128
                } else {
                    delay as usize * 48
                },
                before_rms,
                attack_rms: envelope::rms(&attack),
                first_signal_frame: attack
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .position(|frame| frame.iter().any(|x| x.abs() > 1e-6)),
                energy: attack.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / 2.0,
                peak: attack
                    .iter()
                    .map(|x| f64::from(x.abs()))
                    .fold(0.0, f64::max),
            });
            pcm.sync_data().unwrap();
            save(output, &rows);
            ffi::trace::mark("pilot checkpoint saved; chain destroy begin");
            drop(processor);
            ffi::trace::mark("pilot chain destroy end");
            drop(instance);
            if repeat.is_multiple_of(10) && row_count(&rows).is_multiple_of(4) {
                eprintln!(
                    "fresh pilot {}/7 | {} | {repeat}/30 rounds",
                    rows.len(),
                    patch.label
                );
            }
        }
        pcm.sync_all().unwrap();
    }
    assert_eq!(rows.len(), 7);
    assert!(rows.iter().all(|row| row.observations.len() == REPEATS * 4));
}

fn row_count(rows: &[Row]) -> usize {
    rows.last().unwrap().observations.len()
}

#[test]
fn pilot_schedule_balances_every_round_and_repeats_with_fixed_seed() {
    let first = schedule(&mut StdRng::seed_from_u64(SEED));
    assert_eq!(first, schedule(&mut StdRng::seed_from_u64(SEED)));
    for (round, entries) in first.as_chunks::<4>().0.iter().enumerate() {
        let mut actual: Vec<_> = entries
            .iter()
            .map(|(repeat, group)| {
                assert_eq!(*repeat, round + 1);
                *group
            })
            .collect();
        actual.sort();
        let mut expected = GROUPS;
        expected.sort();
        assert_eq!(actual, expected);
    }
}

#[test]
fn pilot_checkpoint_preserves_prefix_and_rejects_pcm_or_contract_mismatch() {
    let path = std::env::temp_dir().join(format!(
        "cat-attack-checkpoint-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    let output = path.join("pilot.json");
    let pcm_path = path.join("audio.f32le");
    let expected = schedule(&mut StdRng::seed_from_u64(SEED));
    let rows = vec![Row {
        patch: "fixture".into(),
        state_sha256: "fixture".into(),
        audio_file: "audio.f32le".into(),
        observations: expected[..3]
            .iter()
            .enumerate()
            .map(|(order, &(repeat, group))| Observation {
                group: group.into(),
                repeat,
                order,
                pre_note_frames: 0,
                before_rms: None,
                attack_rms: vec![0.0; 40],
                first_signal_frame: None,
                energy: 0.0,
                peak: 0.0,
            })
            .collect(),
    }];
    let pcm = File::create_new(&pcm_path).unwrap();
    pcm.set_len(3 * 76_800).unwrap();
    save(&output, &rows);
    let loaded = resume(&output);
    validate(&loaded[0], &expected, &pcm_path);
    assert_eq!(
        serde_json::to_value(&loaded).unwrap(),
        serde_json::to_value(&rows).unwrap()
    );
    pcm.set_len(76_800).unwrap();
    assert!(std::panic::catch_unwind(|| validate(&loaded[0], &expected, &pcm_path)).is_err());
    let mut changed = document(&rows);
    changed["seed"] = serde_json::json!(0);
    std::fs::write(&output, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(std::panic::catch_unwind(|| resume(&output)).is_err());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(&output).unwrap()).unwrap(),
        changed
    );
    drop(pcm);
    std::fs::remove_file(&output).unwrap();
    std::fs::remove_file(&pcm_path).unwrap();
    std::fs::remove_dir(path).unwrap();
}
