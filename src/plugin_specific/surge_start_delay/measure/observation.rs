use super::*;
use crate::patch_catalog::Candidate;
use std::{fs::File, process::Stdio, time::Duration};

pub(super) fn measure(args: &Options, c: &Candidate, exe: &Path, work: &Path) -> Entry {
    let mut entry = Entry {
        fxp_sha256: String::new(),
        state_sha256: None,
        candidate_onset_ms: None,
        reference_onset_ms: None,
        outcome: Outcome::Error,
        zero_wait_allowed: false,
        error: None,
    };
    let result = (|| {
        let raw = std::fs::read(&c.path).map_err(|e| e.to_string())?;
        entry.fxp_sha256 = hash(&raw);
        let dir = work.join(hash(c.display.as_bytes()));
        std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
        let state = crate::state_request::catalog(ID, &c.bundle_path, &c.path)?;
        entry.state_sha256 = Some(hash(&state));
        let state_file = dir.join("state.bin");
        std::fs::write(&state_file, state).map_err(|e| e.to_string())?;
        // A separate process prevents state, RNG and plugin globals leaking between waits.
        entry.reference_onset_ms = render(
            args,
            c,
            exe,
            &dir,
            &state_file,
            args.reference_wait_ms,
            "reference",
        )?;
        entry.candidate_onset_ms = render(
            args,
            c,
            exe,
            &dir,
            &state_file,
            args.candidate_wait_ms,
            "candidate",
        )?;
        entry.outcome = if entry.reference_onset_ms.is_none() || entry.candidate_onset_ms.is_none()
        {
            Outcome::Silent
        } else {
            Outcome::Measured
        };
        Ok::<_, String>(())
    })();
    if let Err(error) = result {
        entry.error = Some(error);
    }
    entry
}

fn render(
    args: &Options,
    c: &Candidate,
    exe: &Path,
    dir: &Path,
    state: &Path,
    wait: u32,
    label: &str,
) -> Result<Option<f64>, String> {
    let wav = dir.join(format!("{label}.wav"));
    let stdout =
        File::create(dir.join(format!("{label}.stdout.log"))).map_err(|e| e.to_string())?;
    let stderr =
        File::create(dir.join(format!("{label}.stderr.log"))).map_err(|e| e.to_string())?;
    let mut command = std::process::Command::new(exe);
    command
        .args(["--render-wav"])
        .arg(&wav)
        .args([
            "--plugin-format",
            "CLAP",
            "--plugin-id",
            ID,
            "--plugin-path",
        ])
        .arg(&c.bundle_path)
        .arg("--state-file")
        .arg(state)
        .arg("--wait-ms")
        .arg(wait.to_string())
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let start = Instant::now();
    let exit = loop {
        match child.try_wait() {
            Ok(Some(exit)) => break exit,
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
        if start.elapsed() > Duration::from_secs(args.timeout_seconds) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{label}: {wait}ms render timed out after {} seconds",
                args.timeout_seconds
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let side: serde_json::Value = serde_json::from_slice(
        &std::fs::read(wav.with_extension("json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if !exit.success() && side["status"] != "silent" {
        return Err(format!("{label} render failed: {side}"));
    }
    if side["state_sha256"] != hash(&std::fs::read(state).map_err(|e| e.to_string())?)
        || side["wait_ms"] != wait
    {
        return Err("Render provenance mismatch".into());
    }
    onset(
        &std::fs::read(wav).map_err(|e| e.to_string())?,
        args.onset_ratio,
    )
}

pub(crate) fn onset(wav: &[u8], ratio: f64) -> Result<Option<f64>, String> {
    let invalid = || "Invalid 200ms stereo 48kHz float WAV".to_string();
    if wav.get(..4) != Some(b"RIFF") || wav.get(8..12) != Some(b"WAVE") {
        return Err(invalid());
    }
    let mut at = 12;
    let mut pcm = None;
    let mut valid_format = false;
    while at + 8 <= wav.len() {
        let size = u32::from_le_bytes(wav[at + 4..at + 8].try_into().unwrap()) as usize;
        let data = wav.get(at + 8..at + 8 + size).ok_or_else(invalid)?;
        if &wav[at..at + 4] == b"fmt " {
            valid_format = data == [3, 0, 2, 0, 128, 187, 0, 0, 0, 220, 5, 0, 8, 0, 32, 0];
        }
        if &wav[at..at + 4] == b"data" {
            pcm = Some(data);
        }
        at += 8 + size + size % 2;
    }
    let pcm = pcm
        .filter(|p| p.len() == 9600 * 8 && valid_format)
        .ok_or_else(invalid)?;
    let mut prefix = vec![0.0f64];
    for frame in pcm.as_chunks::<8>().0 {
        let l = f64::from(f32::from_le_bytes(frame[..4].try_into().unwrap()));
        let r = f64::from(f32::from_le_bytes(frame[4..].try_into().unwrap()));
        if !l.is_finite() || !r.is_finite() {
            return Err(invalid());
        }
        prefix.push(prefix.last().unwrap() + (l * l + r * r) / 2.0);
    }
    if *prefix.last().unwrap() == 0.0 {
        return Ok(None);
    }
    let rms: Vec<_> = (0usize..9600)
        .map(|i| {
            ((prefix[(i + 24).min(9600)] - prefix[i.saturating_sub(24)]) / 48.0)
                .max(0.0)
                .sqrt()
        })
        .collect();
    let peak = rms.iter().copied().fold(0.0f64, f64::max);
    Ok(rms
        .iter()
        .position(|v| *v >= peak * ratio)
        .map(|i| i as f64 / 48.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_stereo_power_and_shift_are_distinguished() {
        fn wav(start: usize, amplitude: f32) -> Vec<u8> {
            let mut data = b"RIFF".to_vec();
            data.extend(0u32.to_le_bytes());
            data.extend(b"WAVEfmt ");
            data.extend(16u32.to_le_bytes());
            data.extend([3, 0, 2, 0, 128, 187, 0, 0, 0, 220, 5, 0, 8, 0, 32, 0]);
            data.extend(b"data");
            data.extend(76800u32.to_le_bytes());
            for i in 0..9600 {
                let v = if i >= start { amplitude } else { 0.0 };
                data.extend(v.to_le_bytes());
                data.extend((-v).to_le_bytes());
            }
            data
        }
        assert_eq!(onset(&wav(0, 0.0), 0.01).unwrap(), None);
        let a = onset(&wav(1000, 1.0), 0.01).unwrap().unwrap();
        let b = onset(&wav(1480, 0.5), 0.01).unwrap().unwrap();
        assert!((b - a - 10.0).abs() < 1e-10);
        assert!(onset(b"bad", 0.01).is_err());
    }
}
