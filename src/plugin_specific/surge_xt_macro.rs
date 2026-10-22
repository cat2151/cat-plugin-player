//! ADR 0036: the MIDI CCs that move Surge XT's Macro 1-8, only for macros the patch routes.
use super::surge_xt::{matches_plugin, parts};
use super::Controls;
use crate::config::PluginKey;
use crate::sequence_modulation::MACROS;
use quick_xml::{events::Event, Reader};

#[cfg(test)]
#[path = "surge_xt_macro_tests.rs"]
mod tests;

/// Upstream `modsources::ms_ctrl1`, as written in `<modrouting source>`; Macro 2-8 follow it.
const MACRO1_SOURCE: i32 = 7;

const LABELS: [&str; MACROS] = [
    "Macro 1", "Macro 2", "Macro 3", "Macro 4", "Macro 5", "Macro 6", "Macro 7", "Macro 8",
];

pub(super) fn supports(plugin: &PluginKey) -> bool {
    matches_plugin(plugin)
}

pub(super) fn controls(plugin: &PluginKey, state: &[u8]) -> Controls {
    matches_plugin(plugin)
        .then(|| parse(plugin, state))
        .flatten()
        .unwrap_or_default()
}

#[cfg(test)]
fn macro_ccs(plugin: &PluginKey, state: &[u8]) -> [Option<u8>; MACROS] {
    controls(plugin, state).ccs
}

/// A saved 0-1 position (bipolar macros: -1-1) on the CC scale that moves it.
fn cc_scale(value: Option<Vec<u8>>, bipolar: bool) -> Option<u8> {
    let value: f64 = std::str::from_utf8(&value?).ok()?.parse().ok()?;
    let unit = if bipolar { (value + 1.0) / 2.0 } else { value };
    (0.0..=1.0)
        .contains(&unit)
        .then(|| (unit * 127.0).round() as u8)
}

fn parse(plugin: &PluginKey, state: &[u8]) -> Option<Controls> {
    let (xml, _) = parts(state, plugin.format == "VST3")?;
    let mut reader = Reader::from_reader(xml);
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut routed = [false; MACROS];
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
