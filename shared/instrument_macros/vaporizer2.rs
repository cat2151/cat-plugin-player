//! Vaporizer2 custom modulators 1-4 in its `VC2!`-wrapped JUCE XML state.
use super::Macro;
use quick_xml::{events::Event, Reader};
pub(super) const ID: &str = "com.vastdynamics.VAST2";
const MACROS: usize = 4;

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

/// `CustomModulator1`-`4` (or `m_fCustomModulator1`-`4`) to a zero-based slot.
fn slot(number: &str) -> Option<usize> {
    let n: usize = number.parse().ok()?;
    (1..=MACROS).contains(&n).then(|| n - 1)
}

#[allow(
    clippy::collapsible_if,
    reason = "Also compiled by the Rust 2021 player"
)]
pub(super) fn parse(state: &[u8]) -> Option<Vec<Macro>> {
    let mut reader = Reader::from_reader(xml(state)?);
    let mut depth = 0usize;
    let mut root = false;
    // Matrix rows by number: (source macro, bipolar).
    let mut rows: std::collections::BTreeMap<String, (Option<usize>, Option<bool>)> =
        Default::default();
    let mut positions = [None; MACROS];
    let mut names: [Option<String>; MACROS] = Default::default();
    loop {
        let (element, is_start) = match reader.read_event().ok()? {
            Event::Start(e) => (e, true),
            Event::Empty(e) => (e, false),
            Event::End(_) => {
                depth = depth.checked_sub(1)?;
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
        if depth == 0 {
            if root || element.name().as_ref() != b"VASTvaporizer2" {
                return None;
            }
            root = true;
            for (i, name) in names.iter_mut().enumerate() {
                let n = i + 1;
                *name = attribute(format!("CustomModulator{n}Text").as_bytes())
                    .map(|label| label.trim().to_owned())
                    // Vaporizer2 shows its unnamed default as "Custom<n>".
                    .filter(|label| !label.is_empty() && *label != format!("Custom{n}"));
            }
        } else if depth == 1 && element.name().as_ref() == b"PARAM" {
            if let (Some(id), Some(text)) = (attribute(b"id"), attribute(b"text")) {
                if let Some(i) = id.strip_prefix("m_fCustomModulator").and_then(slot) {
                    // Saved as 0-100; the host parameter is the same value over 0-1.
                    positions[i] = text
                        .parse::<f64>()
                        .ok()
                        .map(|v| v / 100.0)
                        .filter(|v| v.is_finite() && (0.0..=1.0).contains(v));
                } else if let Some(row) = id.strip_prefix("m_uModMatSrce") {
                    rows.entry(row.into()).or_default().0 =
                        text.strip_prefix("CustomModulator").and_then(slot);
                } else if let Some(row) = id.strip_prefix("m_uModMatPolarity") {
                    rows.entry(row.into()).or_default().1 = Some(text == "Bipolar");
                }
            }
        }
        if is_start {
            depth += 1;
        }
    }
    if !root {
        return None;
    }
    // Polarity belongs to each matrix row; the macro reads as bipolar only when every
    // row it drives centers on it, otherwise as its own 0-1 knob.
    let mut polarity: [Option<bool>; MACROS] = [None; MACROS];
    for (source, bipolar) in rows.into_values() {
        if let Some(i) = source {
            let bipolar = bipolar.unwrap_or(false);
            polarity[i] = Some(polarity[i].unwrap_or(true) && bipolar);
        }
    }
    Some(
        (0..MACROS)
            .filter_map(|i| {
                polarity[i].map(|bipolar| Macro {
                    number: i + 1,
                    position: positions[i],
                    bipolar,
                    name: names[i].take(),
                    ccs: Vec::new(),
                })
            })
            .collect(),
    )
}
