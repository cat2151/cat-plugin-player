//! Preserve the selected track and its effective conductor tempo.
use crate::smf::model::{Music, Track};

pub fn extract(music: &Music, track: usize) -> Result<Music, String> {
    let target = music
        .tracks
        .get(track)
        .ok_or("Selection track is missing")?;
    let end = target.events.last().map_or(0, |e| e.tick);
    let tempo = |e: &crate::smf::model::Event| e.status == 0xff && e.meta_type == 0x51;
    let mut events = if music.format == 2 {
        target.events.clone()
    } else {
        let mut events: Vec<_> = music
            .tracks
            .iter()
            .flat_map(|t| &t.events)
            .filter(|e| tempo(e) && e.tick <= end)
            .cloned()
            .collect();
        events.extend(target.events.iter().filter(|e| !tempo(e)).cloned());
        events.sort_by_key(|e| e.tick);
        events
    };
    let mut previous = 0;
    for event in &mut events {
        let delta = event
            .tick
            .checked_sub(previous)
            .ok_or("Selection events out of order")?;
        if delta > 0x0fff_ffff {
            return Err("Selection delta exceeds SMF limit".into());
        }
        event.delta = delta as u32;
        previous = event.tick;
    }
    Ok(Music {
        format: 0,
        division: music.division,
        tracks: vec![Track { events }],
    })
}
