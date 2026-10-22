use super::{corpus, envelope, render};
use crate::{config, ffi};
use serde::Serialize;
use std::{fs::File, io::Write, path::Path};

pub(super) const TARGETS: [(&str, &str); 7] = [
    ("18e027a79f04e1d0-8500-4", "Clavi 2"),
    ("18dff271cbc28648-8c68-0", "Sequenced Bass & Pad 1"),
    ("18ddad02436ecf60-3b0c-3", "Surge XT 21"),
    ("18ddacfbee276644-3b0c-2", "Surge XT 20"),
    ("18ddacf69ee5f230-3b0c-1", "Surge XT 19"),
    ("18e0278df6d31108-8500-0", "Clavi 1"),
    ("18e05e2348bcf7ec-1a14-0", "Clavi 3"),
];
const REPEATS: usize = 30;

#[derive(Serialize)]
struct Observation {
    number: usize,
    before_rms: Vec<f64>,
    attack_rms: Vec<f64>,
    versus_first: envelope::Comparison,
    versus_previous_reference: Option<envelope::Comparison>,
}

#[derive(Serialize)]
struct Row {
    patch: String,
    state_sha256: String,
    audio_file: String,
    first_rms: Vec<f64>,
    observations: Vec<Observation>,
}

fn append_audio(file: &mut File, samples: &[f32]) {
    let bytes: Vec<_> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    file.write_all(&bytes).unwrap();
}

fn save(path: &Path, rows: &[Row]) {
    let report = serde_json::json!({
        "sample_rate": render::RATE, "block_frames": render::BLOCK,
        "note": 48, "velocity": 100, "cc1": 0,
        "window_ms": 5, "attack_ms": 200, "first_delay_ms": 200,
        "settle_frames": (render::RATE as usize).div_ceil(render::BLOCK) * render::BLOCK,
        "reference_count": REPEATS, "format": "CLAP",
        "audio_layout": "f32le stereo interleaved; 9600 frames per segment; first attack, then 30 pairs of pre-note tail and reference attack",
        "rows": rows,
    });
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}

#[test]
#[ignore = "requires the seven approved saved Surge XT patches; writes diagnostic report and PCM, no audio device"]
fn native_surge_attack_reference_variability() {
    let output = std::env::var_os("SURGE_ATTACK_REPEAT_REPORT")
        .expect("SURGE_ATTACK_REPEAT_REPORT must name a report in the global work directory");
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
    let _dll = crate::native_library::load().unwrap();
    let host = unsafe { ffi::Host::new(render::wake, std::ptr::null_mut()) }.unwrap();
    let mut rows = Vec::new();
    for (patch, (id, _)) in patches.iter().zip(TARGETS) {
        let info = host.restore_plugin(&patch.plugin).unwrap();
        let instance = render::Instance::new(&host, info.index, &patch.state);
        let processor = host
            .create_chain(instance.id, &[], false, render::RATE, render::BLOCK as u32)
            .unwrap();
        let mut player = render::Player::new(&host, &processor);
        let audio_file = format!("reference-{id}.f32le");
        let mut pcm = File::create(output.parent().unwrap().join(&audio_file)).unwrap();
        let first = player.attack(200);
        append_audio(&mut pcm, &first);
        rows.push(Row {
            patch: patch.label.clone(),
            state_sha256: corpus::digest(&patch.state),
            audio_file,
            first_rms: envelope::rms(&first),
            observations: Vec::new(),
        });
        for number in 1..=REPEATS {
            let before = player.settle_with_tail();
            let attack = player.attack(0);
            append_audio(&mut pcm, &before);
            append_audio(&mut pcm, &attack);
            let row = rows.last_mut().unwrap();
            let attack_rms = envelope::rms(&attack);
            let versus_first = envelope::compare(&row.first_rms, &attack_rms);
            let versus_previous_reference = row
                .observations
                .last()
                .map(|previous| envelope::compare(&previous.attack_rms, &attack_rms));
            row.observations.push(Observation {
                number,
                before_rms: envelope::rms(&before),
                attack_rms,
                versus_first,
                versus_previous_reference,
            });
            save(output, &rows);
        }
        pcm.sync_all().unwrap();
        eprintln!(
            "reference variability {}/7 | {} | 30/30 measured",
            rows.len(),
            patch.label
        );
    }
    assert_eq!(rows.len(), TARGETS.len());
    assert!(rows.iter().all(|row| row.observations.len() == REPEATS));
}
