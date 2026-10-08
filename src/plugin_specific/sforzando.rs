//! Compare measured ARIA settings while preserving original favorite snapshots.
use crate::config::PluginKey;
use flate2::read::ZlibDecoder;
use quick_xml::{events::Event, Reader};
use std::io::Read;

#[cfg(test)]
#[path = "sforzando_tests.rs"]
mod tests;

pub(crate) fn same_sound(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    if plugin.format != "CLAP" || plugin.id != "com.Plogue Art et Technologie, Inc.sforzando" {
        return false;
    }
    match (settings(left), settings(right)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn settings(state: &[u8]) -> Option<Vec<Vec<u8>>> {
    if state.get(..4)? != b"CEGP" {
        return None;
    }
    let size = u32::from_le_bytes(state.get(4..8)?.try_into().ok()?) as usize;
    if size > 1024 * 1024 {
        return None;
    }
    let mut decoder = ZlibDecoder::new(state.get(8..)?);
    let mut xml = Vec::new();
    decoder
        .by_ref()
        .take(size as u64 + 1)
        .read_to_end(&mut xml)
        .ok()?;
    if xml.len() != size || decoder.total_in() as usize != state.len() - 8 {
        return None;
    }
    std::str::from_utf8(&xml).ok()?;
    let mut reader = Reader::from_reader(xml.as_slice());
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut result = Vec::new();
    let mut root_seen = false;
    loop {
        let event = reader.read_event().ok()?;
        let is_empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(tag) | Event::Empty(tag) => {
                let name = tag.name().as_ref().to_vec();
                let attrs = tag.attributes().collect::<Result<Vec<_>, _>>().ok()?;
                if path.is_empty() {
                    if root_seen
                        || name != b"AriaSave"
                        || !attrs
                            .iter()
                            .any(|a| a.key.as_ref() == b"version" && a.value.as_ref() == b"1982")
                        || !attrs
                            .iter()
                            .any(|a| a.key.as_ref() == b"productID" && a.value.as_ref() == b"1014")
                    {
                        return None;
                    }
                    root_seen = true;
                }
                let runtime_param = path.len() == 2
                    && path[1] == b"Slot"
                    && name == b"Param"
                    && attrs.iter().any(|a| {
                        a.key.as_ref() == b"id" && matches!(a.value.as_ref(), b"1" | b"131")
                    })
                    && attrs.len() == 2
                    && attrs.iter().any(|a| {
                        a.key.as_ref() == b"value"
                            && std::str::from_utf8(a.value.as_ref())
                                .ok()
                                .and_then(|v| v.parse::<f64>().ok())
                                .is_some_and(|v| v.is_finite() && (0.0..=1.0).contains(&v))
                    });
                if !runtime_param {
                    let mut token = name.clone();
                    for a in attrs {
                        if path.len() == 1
                            && matches!(name.as_slice(), b"Settings" | b"Slot" | b"EffectSlot")
                            && a.key.as_ref() == b"sc"
                            && a.value.iter().all(u8::is_ascii_digit)
                            && !a.value.is_empty()
                        {
                            continue;
                        }
                        token.push(0);
                        token.extend_from_slice(a.key.as_ref());
                        token.push(0);
                        token.extend_from_slice(a.value.as_ref());
                    }
                    result.push(token);
                }
                if runtime_param && !is_empty {
                    return None;
                }
                if !is_empty {
                    path.push(name);
                }
            }
            Event::End(tag) => {
                if path.pop()?.as_slice() != tag.name().as_ref() {
                    return None;
                }
                let mut token = b"/".to_vec();
                token.extend_from_slice(tag.name().as_ref());
                result.push(token);
            }
            Event::Text(text) if text.iter().all(u8::is_ascii_whitespace) => {}
            Event::Decl(_) => {}
            Event::Eof => return (root_seen && path.is_empty()).then_some(result),
            _ => return None,
        }
    }
}
