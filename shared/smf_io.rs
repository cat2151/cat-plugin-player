use super::{ffi, model::Music};
use std::path::Path;

pub fn load(path: &Path) -> Result<Music, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ffi::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Serialize absolute ticks and preserve unknown meta/SysEx payloads.
pub fn encode(music: &Music) -> Result<Vec<u8>, String> {
    fn vlq(bytes: &mut Vec<u8>, value: u64) -> Result<(), String> {
        if value > 0x0fff_ffff {
            return Err("SMF VLQ exceeds four bytes".into());
        }
        let mut buffer = [0u8; 4];
        let mut index = 3;
        let mut value = value as u32;
        buffer[index] = (value & 127) as u8;
        value >>= 7;
        while value > 0 {
            index -= 1;
            buffer[index] = (value & 127) as u8 | 128;
            value >>= 7;
        }
        bytes.extend_from_slice(&buffer[index..]);
        Ok(())
    }
    let tracks = u16::try_from(music.tracks.len()).map_err(|_| "Too many SMF tracks")?;
    if tracks == 0 || tracks > 32767 || music.format > 2 || (music.format == 0 && tracks != 1) {
        return Err("Invalid SMF format or track count".into());
    }
    let mut bytes = b"MThd\0\0\0\x06".to_vec();
    bytes.extend_from_slice(&u16::from(music.format).to_be_bytes());
    bytes.extend_from_slice(&tracks.to_be_bytes());
    bytes.extend_from_slice(&music.division.to_be_bytes());
    for track in &music.tracks {
        let mut body = Vec::new();
        let mut previous = 0;
        for event in &track.events {
            vlq(
                &mut body,
                event
                    .tick
                    .checked_sub(previous)
                    .ok_or("SMF events are out of tick order")?,
            )?;
            previous = event.tick;
            body.push(event.status);
            match event.status {
                0xff | 0xf0 | 0xf7 => {
                    if event.status == 0xff {
                        body.push(event.meta_type);
                    }
                    vlq(&mut body, event.data.len() as u64)?;
                }
                0x80..=0xef => {
                    let length = if matches!(event.status & 0xf0, 0xc0 | 0xd0) {
                        1
                    } else {
                        2
                    };
                    if event.data.len() != length || event.data.iter().any(|b| *b >= 128) {
                        return Err("Invalid SMF channel event".into());
                    }
                }
                _ => return Err("Unsupported SMF status".into()),
            }
            body.extend_from_slice(&event.data);
        }
        if !track
            .events
            .last()
            .is_some_and(|e| e.status == 0xff && e.meta_type == 0x2f)
        {
            body.extend_from_slice(&[0, 0xff, 0x2f, 0]);
        }
        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(
            &u32::try_from(body.len())
                .map_err(|_| "SMF track too large")?
                .to_be_bytes(),
        );
        bytes.extend_from_slice(&body);
    }
    ffi::parse(&bytes)?;
    Ok(bytes)
}
