//! Six Sines macros 1-6 in its `<patch><params><p id v/>` XML state.
use super::Macro;
#[cfg(test)]
#[path = "six_sines_tests.rs"]
mod tests;
use quick_xml::{events::Event, Reader};

pub const ID: &str = "org.baconpaul.six-sines";
pub const MACROS: usize = 6;
/// Each modulated node's three source/target slots, as in Six Sines' `computeMacroUsage`:
/// `(first id, stride, nodes, source offset, target offset)`. Slot `i` reads its source
/// at `+source+2i` and its target at `+target+i`.
const NODES: [(u32, u32, u32, u32, u32); 7] = [
    (1500, 250, 6, 150, 160),   // Op Source.
    (10000, 100, 6, 40, 55),    // Op Self (Feedback).
    (20000, 100, 6, 50, 60),    // Op Mixer.
    (30000, 200, 15, 70, 80),   // Matrix (Op a to Op b).
    (40000, 250, 6, 50, 60),    // Macro.
    (50000, 1000, 2, 100, 200), // Main Tune, Main Pan.
    (500, 0, 1, 620, 650),      // Main Output.
];
/// Mod Source values of macro 1 and of macro 1's modulated output.
const SOURCES: [i64; 2] = [400, 410];

/// `Macro N Level`, which Six Sines saves over -1..1.
pub fn level(number: usize) -> u32 {
    40000 + 250 * (number as u32 - 1)
}

/// Saved parameter values by id, and each macro's saved name.
type Saved = (
    std::collections::HashMap<u32, f64>,
    [Option<String>; MACROS],
);

#[allow(
    clippy::collapsible_if,
    reason = "Also compiled by the Rust 2021 player"
)]
fn read(state: &[u8]) -> Option<Saved> {
    // The saved state ends with the XML string's terminating NUL.
    let end = state.iter().rposition(|&b| b != 0)? + 1;
    let xml = state.get(..end)?;
    std::str::from_utf8(xml).ok()?;
    let mut reader = Reader::from_reader(xml);
    let mut params = std::collections::HashMap::new();
    let mut names: [Option<String>; MACROS] = Default::default();
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut root = false;
    let mut name_slot = None;
    loop {
        let (element, is_start) = match reader.read_event().ok()? {
            Event::Start(e) => (e, true),
            Event::Empty(e) => (e, false),
            Event::End(_) => {
                path.pop()?;
                name_slot = None;
                continue;
            }
            Event::CData(text) if name_slot.is_some() => {
                let text = std::str::from_utf8(&text).ok()?.trim().to_owned();
                names[name_slot?] = Some(text);
                continue;
            }
            Event::Eof => break,
            _ => continue,
        };
        let attribute = |key: &[u8]| {
            element
                .attributes()
                .flatten()
                .find(|a| a.key.as_ref() == key)
                .and_then(|a| {
                    a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .ok()
                        .map(|v| v.into_owned())
                })
        };
        let tag = element.name().as_ref().to_vec();
        if path.is_empty() {
            if root || tag != b"patch" || attribute(b"id").as_deref() != Some(ID) {
                return None;
            }
            root = true;
        } else if path.len() == 2 && path[1] == b"params" && tag == b"p" {
            if let (Some(id), Some(v)) = (attribute(b"id"), attribute(b"v")) {
                params.insert(id.parse().ok()?, v.parse().ok()?);
            }
        } else if path.len() == 2 && path[1] == b"macroNames" && tag == b"macroName" {
            if let Some(i) = attribute(b"idx").and_then(|i| i.parse::<usize>().ok()) {
                name_slot = (i < MACROS).then_some(i);
            }
        }
        if is_start {
            path.push(tag);
        }
    }
    root.then_some((params, names))
}

pub(super) fn parse(state: &[u8]) -> Option<Vec<Macro>> {
    let (params, names) = read(state)?;
    let value = |id| params.get(&id).map(|v: &f64| v.round() as i64);
    let mut used = [false; MACROS];
    for (first, stride, nodes, source, target) in NODES {
        for node in 0..nodes {
            let base = first + stride * node;
            for slot in 0..3 {
                if value(base + target + slot).unwrap_or(0) == 0 {
                    continue;
                }
                let Some(s) = value(base + source + 2 * slot) else {
                    continue;
                };
                for first_source in SOURCES {
                    if (first_source..first_source + MACROS as i64).contains(&s) {
                        used[(s - first_source) as usize] = true;
                    }
                }
            }
        }
    }
    let mut names = names;
    Some(
        (0..MACROS)
            .filter(|&i| used[i])
            .map(|i| {
                let number = i + 1;
                Macro {
                    number,
                    // -1..1 to the 0-1 operating position.
                    position: params
                        .get(&level(number))
                        .filter(|v| (-1.0..=1.0).contains(*v))
                        .map(|v| (v + 1.0) / 2.0),
                    bipolar: true,
                    name: names[i]
                        .take()
                        .filter(|n| !n.is_empty() && *n != format!("Macro {number}")),
                    ccs: Vec::new(),
                }
            })
            .collect(),
    )
}
