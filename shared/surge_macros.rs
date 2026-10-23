//! Shared Surge XT state inspection; routed macros are independent of MIDI mappings.
use quick_xml::{events::Event, Reader};
pub const MACROS: usize = 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    Unipolar,
    Bipolar,
}
impl Polarity {
    pub fn from_bipolar(bipolar: bool) -> Self {
        if bipolar {
            Self::Bipolar
        } else {
            Self::Unipolar
        }
    }
    pub fn position(self, signal: f64) -> Option<f64> {
        let position = if self == Self::Bipolar {
            (signal + 1.0) / 2.0
        } else {
            signal
        };
        (signal.is_finite() && (0.0..=1.0).contains(&position)).then_some(position)
    }
    pub fn signal(self, position: f64) -> Option<f64> {
        (position.is_finite() && (0.0..=1.0).contains(&position)).then(|| {
            if self == Self::Bipolar {
                position * 2.0 - 1.0
            } else {
                position
            }
        })
    }
}
#[derive(Clone, Debug, Default)]
pub struct Controls {
    pub routed: [bool; MACROS],
    pub positions: [Option<f64>; MACROS],
    pub bipolar: [bool; MACROS],
    pub ccs: [Option<u8>; MACROS],
    pub labels: [&'static str; MACROS],
    pub wheel: Option<u8>,
    pub macros: [Option<u8>; MACROS],
    pub names: [Option<String>; MACROS],
}
pub fn supports(format: &str, id: &str) -> bool {
    matches!(
        (format, id),
        ("CLAP", "org.surge-synth-team.surge-xt") | ("VST3", "ABCDEF019182FAEB566D624153675854")
    )
}
/// Upstream `modsources::ms_ctrl1`, as written in `<modrouting source>`; Macro 2-8 follow it.
const MACRO1_SOURCE: i32 = 7;

const LABELS: [&str; MACROS] = [
    "Macro 1", "Macro 2", "Macro 3", "Macro 4", "Macro 5", "Macro 6", "Macro 7", "Macro 8",
];

/// A saved 0-1 position (bipolar macros: -1-1) on the CC scale that moves it.
fn unit_scale(value: Option<Vec<u8>>, bipolar: bool) -> Option<f64> {
    let value: f64 = std::str::from_utf8(&value?).ok()?.parse().ok()?;
    Polarity::from_bipolar(bipolar).position(value)
}

fn cc_scale(value: Option<Vec<u8>>, bipolar: bool) -> Option<u8> {
    unit_scale(value, bipolar).map(|v| (v * 127.0).round() as u8)
}

#[allow(
    clippy::collapsible_if,
    reason = "Also compiled by the Rust 2021 player"
)]
pub fn parse(state: &[u8], vst: bool) -> Option<Controls> {
    let (xml, _) = parts(state, vst)?;
    let mut reader = Reader::from_reader(xml);
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut routed = [false; MACROS];
    let mut unit_positions = [None; MACROS];
    let mut bipolar_flags = [false; MACROS];
    let (mut cc, mut channel) = ([None::<i32>; MACROS], [None::<i32>; MACROS]);
    let (mut wheel, mut positions) = (None, [None; MACROS]);
    let mut names: [Option<String>; MACROS] = Default::default();
    let number =
        |value: Option<Vec<u8>>| -> Option<i32> { std::str::from_utf8(&value?).ok()?.parse().ok() };
    loop {
        let (element, is_start) = match reader.read_event().ok()? {
            Event::Start(e) => (e, true),
            Event::Empty(e) => (e, false),
            Event::End(_) => {
                path.pop();
                continue;
            }
            Event::Eof => break,
            _ => continue,
        };
        let tag = element.name().as_ref().to_vec();
        let attribute = |key: &[u8]| {
            element
                .attributes()
                .flatten()
                .find(|a| a.key.as_ref() == key)
                .map(|a| a.value.into_owned())
        };
        let slot =
            |value: Option<i32>| usize::try_from(value?).ok().filter(|index| *index < MACROS);
        if path.len() == 1 && path[0] == b"patch" && tag == b"modwheel" {
            // The player's CC1 reaches both scenes; scene A stands for them.
            wheel = cc_scale(attribute(b"s0"), false);
        } else if path.len() == 2 && path[1] == b"customcontroller" && tag == b"entry" {
            if let Some(index) = slot(number(attribute(b"i"))) {
                let bipolar = attribute(b"bipolar").is_some_and(|b| b == b"1");
                positions[index] = cc_scale(attribute(b"v"), bipolar);
                unit_positions[index] = unit_scale(attribute(b"v"), bipolar);
                bipolar_flags[index] = bipolar;
                names[index] = element
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"label")
                    .and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok())
                    .map(|label| {
                        label
                            .trim_matches(|c: char| c.is_whitespace() || c.is_control())
                            .to_owned()
                    })
                    // Surge shows "Macro n" for its unnamed "-".
                    .filter(|label| !label.is_empty() && label != "-");
            }
        } else if tag == b"modrouting" {
            let source = number(attribute(b"source")).and_then(|s| s.checked_sub(MACRO1_SOURCE));
            if let Some(index) = slot(source) {
                routed[index] = true;
            }
        } else if path.len() == 3 && path[0] == b"patch" && path[1] == b"dawExtraState" {
            if let Some(index) = slot(number(attribute(b"p"))) {
                let value = number(attribute(b"v"));
                match (path[2].as_slice(), tag.as_slice()) {
                    (b"customcontrol_map", b"c") => cc[index] = value,
                    (b"customcontrol_chan_map", b"ch") => channel[index] = value,
                    _ => {}
                }
            }
        }
        if is_start {
            path.push(tag);
        }
    }
    // The player sends on channel 1 (0); Surge's -1 means any channel.
    Some(Controls {
        routed,
        positions: unit_positions,
        bipolar: bipolar_flags,
        ccs: std::array::from_fn(|i| {
            let cc = u8::try_from(cc[i]?).ok().filter(|cc| *cc < 128)?;
            (routed[i] && matches!(channel[i], Some(-1 | 0))).then_some(cc)
        }),
        labels: LABELS,
        wheel,
        macros: positions,
        names,
    })
}

pub fn parts(state: &[u8], vst: bool) -> Option<(&[u8], &[u8])> {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_macro_keeps_its_declared_polarity_and_zero() {
        assert_eq!(Polarity::Unipolar.signal(0.5), Some(0.5));
        assert_eq!(Polarity::Bipolar.signal(0.5), Some(0.0));
        for (signal, position) in [(-1.0, 0.0), (0.0, 0.5), (1.0, 1.0)] {
            assert_eq!(Polarity::Bipolar.position(signal), Some(position));
            assert_eq!(Polarity::Bipolar.signal(position), Some(signal));
        }
        assert!(Polarity::Unipolar.position(-1.0).is_none());
        assert!(Polarity::Bipolar.position(1.1).is_none());
    }
    #[test]
    fn routed_macro_does_not_require_midi_mapping() {
        let xml = br#"<patch><parameters><p><modrouting source="7" depth="0.3"/><modrouting source="14" depth="1"/></p></parameters></patch>"#;
        let mut state = b"sub3".to_vec();
        state.extend((xml.len() as u32).to_le_bytes());
        state.extend([0; 24]);
        state.extend(xml);
        let controls = parse(&state, false).unwrap();
        assert_eq!(
            controls.routed,
            [true, false, false, false, false, false, false, true]
        );
        assert_eq!(controls.ccs, [None; 8]);
        assert!(parse(b"sub3", false).is_none());
    }
}
