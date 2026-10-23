//! Six Sines: exclude the saved `Macro N Level` values, retaining full snapshots.
use crate::config::PluginKey;
use crate::instrument_macros::six_sines;
use quick_xml::events::{attributes::Attribute, Event};

#[cfg(test)]
#[path = "six_sines_tests.rs"]
mod tests;

/// The XML text and the bytes after it (the string's terminating NUL).
fn split(state: &[u8]) -> Option<(&[u8], &[u8])> {
    let end = state.iter().rposition(|&b| b != 0)? + 1;
    std::str::from_utf8(&state[..end]).ok()?;
    Some(state.split_at(end))
}

fn attribute<'a>(attributes: &'a [Attribute], key: &[u8]) -> Option<&'a [u8]> {
    attributes
        .iter()
        .find(|a| a.key.as_ref() == key)
        .map(|a| a.value.as_ref())
}

/// Whether `<p id>` is a `Macro N Level`.
fn macro_level(id: Option<&[u8]>) -> bool {
    let id = id
        .and_then(|id| std::str::from_utf8(id).ok())
        .and_then(|id| id.parse::<u32>().ok());
    (1..=six_sines::MACROS).any(|n| Some(six_sines::level(n)) == id)
}

fn level_value(value: &[u8]) -> bool {
    std::str::from_utf8(value)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .is_some_and(|v| (-1.0..=1.0).contains(&v))
}

pub(crate) fn same_sound(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    if plugin.format != "CLAP" || plugin.id != six_sines::ID {
        return false;
    }
    let (Some((left, left_end)), Some((right, right_end))) = (split(left), split(right)) else {
        return false;
    };
    if left_end != right_end {
        return false;
    }
    let mut left = quick_xml::Reader::from_reader(left);
    let mut right = quick_xml::Reader::from_reader(right);
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut root = false;
    loop {
        let (Ok(a), Ok(b)) = (left.read_event(), right.read_event()) else {
            return false;
        };
        let is_start = matches!(a, Event::Start(_));
        match (&a, &b) {
            (Event::Start(a), Event::Start(b)) | (Event::Empty(a), Event::Empty(b)) => {
                let tag = a.name().as_ref().to_vec();
                let (Ok(aa), Ok(bb)) = (
                    a.attributes().collect::<Result<Vec<_>, _>>(),
                    b.attributes().collect::<Result<Vec<_>, _>>(),
                ) else {
                    return false;
                };
                if tag != b.name().as_ref() || aa.len() != bb.len() {
                    return false;
                }
                if path.is_empty() {
                    if root
                        || tag != b"patch"
                        || attribute(&aa, b"id") != Some(six_sines::ID.as_bytes())
                    {
                        return false;
                    }
                    root = true;
                }
                let level = path.len() == 2
                    && path[1] == b"params"
                    && tag == b"p"
                    && attribute(&aa, b"id") == attribute(&bb, b"id")
                    && macro_level(attribute(&aa, b"id"));
                for (x, y) in aa.iter().zip(&bb) {
                    if x.key != y.key
                        || (x.value != y.value
                            && !(level
                                && x.key.as_ref() == b"v"
                                && level_value(&x.value)
                                && level_value(&y.value)))
                    {
                        return false;
                    }
                }
                if is_start {
                    path.push(tag);
                }
            }
            (Event::End(x), Event::End(y)) => {
                if x != y || path.pop().as_deref() != Some(x.name().as_ref()) {
                    return false;
                }
            }
            (Event::Eof, Event::Eof) => return root && path.is_empty(),
            _ => {
                if a != b {
                    return false;
                }
            }
        }
    }
}
