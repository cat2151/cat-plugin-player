use std::{fs::File, io::Write, path::Path};

pub(super) struct Output {
    wav: File,
    report: File,
}

impl Output {
    pub fn new(path: &Path) -> Result<Self, String> {
        let sidecar = path.with_extension("json");
        if path == sidecar || path.exists() || sidecar.exists() {
            return Err("WAV/JSON outputs must be distinct new files".into());
        }
        let wav = File::create_new(path).map_err(|e| e.to_string())?;
        let report = File::create_new(sidecar).map_err(|e| e.to_string())?;
        Ok(Self { wav, report })
    }

    pub fn wav(&mut self, pcm: &[f32]) -> Result<(), String> {
        let length = u32::try_from(pcm.len() * 4).map_err(|_| "PCM too large")?;
        let mut bytes = Vec::with_capacity(56 + length as usize);
        bytes.extend(b"RIFF");
        bytes.extend((48 + length).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16_u32.to_le_bytes());
        bytes.extend(3_u16.to_le_bytes()); // IEEE float
        bytes.extend(2_u16.to_le_bytes());
        bytes.extend(super::RATE.to_le_bytes());
        bytes.extend((super::RATE * 8).to_le_bytes());
        bytes.extend(8_u16.to_le_bytes());
        bytes.extend(32_u16.to_le_bytes());
        bytes.extend(b"fact");
        bytes.extend(4_u32.to_le_bytes());
        bytes.extend((pcm.len() as u32 / 2).to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(length.to_le_bytes());
        bytes.extend(pcm.iter().flat_map(|s| s.to_le_bytes()));
        self.wav
            .write_all(&bytes)
            .and_then(|_| self.wav.sync_all())
            .map_err(|e| e.to_string())
    }

    pub fn report(&mut self, report: &serde_json::Value) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
        self.report
            .write_all(&bytes)
            .and_then(|_| self.report.sync_all())
            .map_err(|e| e.to_string())
    }
}
