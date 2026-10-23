//! ADR 0038: the MIDI CCs that move TyrellN6's Control A / B, only when the patch uses them.
use super::Controls;
use crate::config::PluginKey;

#[cfg(test)]
#[path = "tyrell_n6_control_tests.rs"]
mod tests;

/// `#ms=` names, the `[PCore]` keys holding their CCs, the Usage prefix, and the row labels.
const CONTROLS: [(&str, &str, &str, &str); 2] = [
    ("CtrlA", "CtrlA", "Ctrl A", "Control A"),
    ("CtrlB", "CtrlB", "Ctrl B", "Control B"),
];

/// Keys holding a modulation source index, per `#cm=` section; each is a CLAP
/// parameter showing the `#ms=` names, except the LFOs' undisclosed `DMS1`.
const SOURCES: [(&str, &[&str]); 4] = [
    ("PCore", &["MMS1", "MMS2", "MMVS1", "MMVS2"]),
    (
        "Tyrell",
        &["TMSrc1", "TMSrc2", "PWSrc", "FMSrc1", "FMSrc2", "ARMod"],
    ),
    ("LFO1", &["FMS1", "DMS1"]),
    ("LFO2", &["FMS1", "DMS1"]),
];

pub(super) fn supports(plugin: &PluginKey) -> bool {
    plugin.format == "CLAP" && plugin.id == "com.u-he.TyrellN6"
}

pub(super) fn controls(plugin: &PluginKey, state: &[u8]) -> Controls {
    supports(plugin)
        .then(|| parse(state))
        .flatten()
        .unwrap_or_default()
}

fn parse(state: &[u8]) -> Option<Controls> {
    let text = String::from_utf8_lossy(state);
    // The compressed section after the settings holds no control assignments.
    let mut lines = text
        .lines()
        .take_while(|line| !line.starts_with("// Section for ugly"));
    let mut sources: Vec<&str> = Vec::new();
    let (mut section, mut usage) = ("", None);
    let mut routed = [false; CONTROLS.len()];
    let mut ccs = [None; CONTROLS.len()];
    let mut known = false;
    while let Some(line) = lines.next() {
        if line == "Usage:" {
            usage = lines.next();
        } else if line == "#AM=TyrellN6" {
            known = true;
        } else if let Some(name) = line.strip_prefix("#ms=") {
            sources.push(name);
        } else if let Some(name) = line.strip_prefix("#cm=") {
            section = name;
        } else if let Some((key, value)) = line.split_once('=') {
            let keys = SOURCES.iter().find(|(s, _)| *s == section);
            if keys.is_some_and(|(_, keys)| keys.contains(&key)) {
                let source = value.parse::<usize>().ok().and_then(|i| sources.get(i));
                for (i, control) in CONTROLS.iter().enumerate() {
                    routed[i] |= source == Some(&control.0);
                }
            } else if section == "PCore" {
                if let Some(i) = CONTROLS.iter().position(|control| control.1 == key) {
                    ccs[i] = value.parse::<u8>().ok().filter(|cc| *cc < 128);
                }
            }
        }
    }
    known.then_some(())?;
    let mut controls = Controls::default();
    for (i, control) in CONTROLS.iter().enumerate() {
        controls.targets[i] = ccs[i]
            .filter(|_| routed[i])
            .map(crate::sequence_modulation::MacroTarget::Cc);
        controls.labels[i] = control.3;
        controls.names[i] = usage.and_then(|usage| name(usage, control.2));
    }
    Some(controls)
}

/// `'MW = lo pass\r\nCtrl A = bass dominator'` (escaped line breaks) names Control A.
fn name(usage: &str, prefix: &str) -> Option<String> {
    usage
        .trim_matches('\'')
        .split("\\r\\n")
        .find_map(|entry| entry.strip_prefix(prefix)?.trim_start().strip_prefix('='))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}
