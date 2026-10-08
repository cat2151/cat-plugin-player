//! Exclude only measured editor zoom and live CC1 values from favorite comparison.
use crate::config::PluginKey;
use quick_xml::{events::Event, Reader};

#[cfg(test)]
#[path = "surge_xt_tests.rs"]
mod tests;

fn matches_plugin(plugin: &PluginKey) -> bool {
    matches!(
        (plugin.format.as_str(), plugin.id.as_str()),
        ("CLAP", "org.surge-synth-team.surge-xt") | ("VST3", "ABCDEF019182FAEB566D624153675854")
    )
}

// ADR 0008: provisional settling time while the audio thread applies restored state.
pub(super) fn automatic_note_start_delay(plugin: &PluginKey) -> std::time::Duration {
    if matches_plugin(plugin) {
        std::time::Duration::from_millis(100)
    } else {
        std::time::Duration::ZERO
    }
}

fn parts(state: &[u8], vst: bool) -> Option<(&[u8], &[u8])> {
    if state.get(..4)? != b"sub3" {
        return None;
    }
    let size = u32::from_le_bytes(state.get(4..8)?.try_into().ok()?) as usize;
    let end = 32usize.checked_add(size)?;
    let mut total = end;
    for bytes in state.get(8..32)?.as_chunks::<4>().0 {
        total = total.checked_add(u32::from_le_bytes(*bytes) as usize)?;
    }
    // JUCE VST3 adds this measured private trailer; compare it unchanged below.
    let trailer = state.get(total..)?;
    if !trailer.is_empty()
        && !(vst && trailer == b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0JUCEPrivateData")
    {
        return None;
    }
    let xml = state.get(32..end)?;
    std::str::from_utf8(xml).ok()?;
    Some((xml, state.get(end..)?))
}

fn runtime_field(path: &[Vec<u8>], tag: &[u8], key: &[u8]) -> bool {
    (path.len() == 1 && path[0] == b"patch" && tag == b"modwheel" && matches!(key, b"s0" | b"s1"))
        || (path.len() == 2
            && path[0] == b"patch"
            && path[1] == b"dawExtraState"
            && tag == b"instanceZoomFactor"
            && key == b"v")
}

fn valid_value(tag: &[u8], value: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(value) else {
        return false;
    };
    if tag == b"modwheel" {
        text.parse::<f64>()
            .is_ok_and(|n| n.is_finite() && (0.0..=1.0).contains(&n))
    } else {
        text.parse::<i32>()
            .is_ok_and(|n| n == -1 || (1..=1000).contains(&n))
    }
}

pub(crate) fn same_sound(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    if !matches_plugin(plugin) {
        return false;
    }
    let (Some((lx, lt)), Some((rx, rt))) = (
        parts(left, plugin.format == "VST3"),
        parts(right, plugin.format == "VST3"),
    ) else {
        return false;
    };
    if left[8..32] != right[8..32] || lt != rt {
        return false;
    }
    let (mut left, mut right) = (Reader::from_reader(lx), Reader::from_reader(rx));
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut root_seen = false;
    loop {
        let (Ok(a), Ok(b)) = (left.read_event(), right.read_event()) else {
            return false;
        };
        let is_start = matches!(a, Event::Start(_));
        if is_start != matches!(b, Event::Start(_)) {
            return false;
        }
        match (&a, &b) {
            (Event::Start(a), Event::Start(b)) | (Event::Empty(a), Event::Empty(b)) => {
                if a.name() != b.name() {
                    return false;
                }
                let tag = a.name();
                let (Ok(aa), Ok(bb)) = (
                    a.attributes().collect::<Result<Vec<_>, _>>(),
                    b.attributes().collect::<Result<Vec<_>, _>>(),
                ) else {
                    return false;
                };
                if aa.len() != bb.len() {
                    return false;
                }
                if path.is_empty() {
                    if root_seen
                        || tag.as_ref() != b"patch"
                        || !aa.iter().any(|attr| {
                            attr.key.as_ref() == b"revision" && attr.value.as_ref() == b"24"
                        })
                    {
                        return false;
                    }
                    root_seen = true;
                }
                for (a, b) in aa.iter().zip(&bb) {
                    if a.key != b.key
                        || (a.value != b.value
                            && !(runtime_field(&path, tag.as_ref(), a.key.as_ref())
                                && valid_value(tag.as_ref(), &a.value)
                                && valid_value(tag.as_ref(), &b.value)))
                    {
                        return false;
                    }
                }
                if is_start {
                    path.push(tag.as_ref().to_vec());
                }
            }
            (Event::End(a), Event::End(b)) => {
                if a != b || path.pop().as_deref() != Some(a.name().as_ref()) {
                    return false;
                }
            }
            (Event::Eof, Event::Eof) => return root_seen && path.is_empty(),
            (Event::Decl(_), Event::Decl(_)) | (Event::Text(_), Event::Text(_)) => {
                if a != b {
                    return false;
                }
            }
            _ => return false,
        }
    }
}
