//! Tolerate measured MSEG serialization/recalculation drift, retaining full snapshots.
use crate::config::PluginKey;
use quick_xml::{events::Event, Reader};

#[cfg(test)]
#[path = "vaporizer2_tests.rs"]
mod tests;

const VERSION: &[u8] = b"VASTVaporizerParamsV2.20000";
// Measured saved-state differences; see ADR 0010 for bounds and limitations.
const TIME_EPSILON: f64 = 0.00011; // milliseconds
const POSITION_EPSILON: f64 = 0.000001; // normalized coordinate

fn xml(state: &[u8]) -> Option<&[u8]> {
    if state.get(..4)? != b"VC2!" {
        return None;
    }
    let size = u32::from_le_bytes(state.get(4..8)?.try_into().ok()?) as usize;
    if size.checked_add(9)? != state.len() || state.last()? != &0 {
        return None;
    }
    let xml = state.get(8..8 + size)?;
    std::str::from_utf8(xml).ok()?;
    Some(xml)
}

fn mseg(name: &[u8]) -> bool {
    matches!(
        name,
        b"msegData0" | b"msegData1" | b"msegData2" | b"msegData3" | b"msegData4"
    )
}

fn drift_field(path: &[Vec<u8>], tag: &[u8], key: &[u8]) -> bool {
    if path.first().map(Vec::as_slice) != Some(b"VASTvaporizer2")
        || path.get(1).map(Vec::as_slice) != Some(b"chunkData")
    {
        return false;
    }
    (path.len() == 2
        && mseg(tag)
        && matches!(
            key,
            b"m_fAttackTimeExternalSet" | b"m_fDecayTimeExternalSet" | b"m_fReleaseTimeExternalSet"
        ))
        || (path.len() == 3
            && mseg(&path[2])
            && matches!(tag, b"msegPoint1" | b"msegPoint2")
            && key == b"xVal")
}

fn close(left: &[u8], right: &[u8], key: &[u8]) -> bool {
    let epsilon = if key == b"xVal" {
        POSITION_EPSILON
    } else {
        TIME_EPSILON
    };
    let number = |value| std::str::from_utf8(value).ok()?.parse::<f64>().ok();
    match (number(left), number(right)) {
        (Some(a), Some(b)) => a.is_finite() && b.is_finite() && (a - b).abs() <= epsilon,
        _ => false,
    }
}

pub(crate) fn same_sound(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    compare(plugin, left, right, false)
}

pub(crate) fn same_sweep_sound(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    compare(plugin, left, right, true)
}

fn compare(plugin: &PluginKey, left: &[u8], right: &[u8], sweep_cc1: bool) -> bool {
    if plugin.format != "CLAP" || plugin.id != "com.vastdynamics.VAST2" {
        return false;
    }
    let (Some(left), Some(right)) = (xml(left), xml(right)) else {
        return false;
    };
    let (mut left, mut right) = (Reader::from_reader(left), Reader::from_reader(right));
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut root_seen = false;
    loop {
        let (Ok(a), Ok(b)) = (left.read_event(), right.read_event()) else {
            return false;
        };
        let is_start = matches!(a, Event::Start(_));
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
                let cc1_macro = sweep_cc1
                    && path.len() == 1
                    && path[0] == b"VASTvaporizer2"
                    && tag.as_ref() == b"PARAM"
                    && aa.iter().any(|attr| {
                        attr.key.as_ref() == b"id" && attr.value.as_ref() == b"m_fCustomModulator1"
                    })
                    && bb.iter().any(|attr| {
                        attr.key.as_ref() == b"id" && attr.value.as_ref() == b"m_fCustomModulator1"
                    });
                if path.is_empty() {
                    if root_seen
                        || tag.as_ref() != b"VASTvaporizer2"
                        || !aa.iter().any(|attr| {
                            attr.key.as_ref() == b"PatchVersion" && attr.value.as_ref() == VERSION
                        })
                    {
                        return false;
                    }
                    root_seen = true;
                }
                for (a, b) in aa.iter().zip(&bb) {
                    if a.key != b.key
                        || (a.value != b.value
                            && !(drift_field(&path, tag.as_ref(), a.key.as_ref())
                                && close(&a.value, &b.value, a.key.as_ref()))
                            && !(cc1_macro
                                && a.key.as_ref() == b"text"
                                && valid_macro(&a.value)
                                && valid_macro(&b.value)))
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

fn valid_macro(value: &[u8]) -> bool {
    std::str::from_utf8(value)
        .ok()
        .and_then(|text| text.parse::<f64>().ok())
        .is_some_and(|n| n.is_finite() && (0.0..=100.0).contains(&n))
}
