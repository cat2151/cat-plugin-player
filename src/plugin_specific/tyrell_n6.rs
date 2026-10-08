//! Sound comparison for favorite deduplication; stored snapshots stay intact.
use crate::config::PluginKey;

#[cfg(test)]
#[path = "tyrell_n6_tests.rs"]
mod native_tests;

const COMPRESSED: &str =
    "// Section for ugly compressed binary Data\n// DON'T TOUCH THIS\n\n$$$$1924\n";

pub(crate) fn same_sound(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    if left == right {
        return true;
    }
    if plugin.format != "CLAP" || plugin.id != "com.u-he.TyrellN6" {
        return false;
    }
    match (tyrell_settings(left), tyrell_settings(right)) {
        (Some(left), Some(right)) => {
            let left = left.trim_end_matches('\n');
            let right = right.trim_end_matches('\n');
            left == right
                || matches!((ui_operation(left), ui_operation(right)),
                    (Some(left), Some(right)) if left == right)
        }
        _ => false,
    }
}

// Build 16976 can change PCore UI_op from 9 to 10 after processing a
// restored state. Only this observed pair is equivalent; retain other values,
// sections, metadata, and every sound parameter in the comparison.
fn ui_operation(settings: &str) -> Option<(&str, &str)> {
    let (before, rest) = settings.split_once("\nUI_op=")?;
    let (value, after) = rest.split_once('\n')?;
    if !matches!(value, "9" | "10")
        || before.lines().rev().find(|line| line.starts_with("#cm=")) != Some("#cm=PCore")
        || settings
            .lines()
            .filter(|line| line.starts_with("UI_op="))
            .count()
            != 1
    {
        return None;
    }
    Some((before, after))
}

fn tyrell_settings(state: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(state).ok()?;
    let (settings, compressed) = text.split_once(COMPRESSED)?;
    // This layout was verified with TyrellN6 CLAP build 16976. Playing notes
    // and CC1 changes the compressed section without changing sound settings.
    // Keep metadata and sound parameters; same_sound handles the observed
    // PCore UI_op 9/10 transition and trailing empty lines separately.
    // Unknown revisions/layouts fall back to exact bytes, rather than guessing.
    if !settings.starts_with("#pgm=")
        || !settings.lines().any(|line| line == "#AM=TyrellN6")
        || !settings.lines().any(|line| line == "#Vers=10010")
        || !settings.lines().any(|line| line == "#Endian=little")
        || !settings.lines().any(|line| line == "Rev=16976")
        || !settings.lines().any(|line| line == "#cm=Tyrell")
    {
        return None;
    }
    let compressed = compressed.strip_suffix("\n\0\0")?;
    let (payload, checksum) = compressed.rsplit_once('=')?;
    if !payload.starts_with('?')
        || payload.len() < 10
        || checksum.is_empty()
        || !checksum.bytes().all(|b| b.is_ascii_digit())
        || !payload.bytes().all(|b| b.is_ascii_graphic() || b == b'\n')
    {
        return None;
    }
    Some(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin() -> PluginKey {
        PluginKey {
            format: "CLAP".into(),
            id: "com.u-he.TyrellN6".into(),
            name: "TyrellN6".into(),
            vendor: "u-he".into(),
            bundle_path: "X:/plugins/TyrellN6.clap".into(),
        }
    }

    fn state(cutoff: u32, runtime: &str) -> Vec<u8> {
        format!(
            "#pgm=Test.h2p\n#AM=TyrellN6\n#Vers=10010\n#Endian=little\n\
             #cm=PCore\nRev=16976\nUI_op=9\n#cm=Tyrell\nCutoff={cutoff}.00\n\n\
             {COMPRESSED}?abcdefghij{runtime}=81976\n\0\0"
        )
        .into_bytes()
    }

    #[test]
    fn observed_ui_transition_and_trailing_empty_lines_match() {
        let first = state(85, "one");
        let second = String::from_utf8(state(85, "two"))
            .unwrap()
            .replace("UI_op=9", "UI_op=10")
            .replace(&format!("\n\n{COMPRESSED}"), &format!("\n\n\n{COMPRESSED}"));
        assert!(same_sound(&plugin(), &first, second.as_bytes()));
        assert!(same_sound(&plugin(), second.as_bytes(), &first));
        assert!(!same_sound(&plugin(), &state(84, "one"), second.as_bytes()));
        for (from, to) in [
            ("UI_op=10", "UI_op=11"),
            ("UI_op=10", "UI_op=09"),
            ("UI_op=10", "UI_op=10\nUI_op=9"),
            ("UI_op=10", "#cm=Tyrell\nUI_op=10"),
            ("Rev=16976", "Rev=16976\n"),
            ("Test.h2p", "Other.h2p"),
        ] {
            let changed = second.replace(from, to);
            assert!(!same_sound(&plugin(), &first, changed.as_bytes()), "{to}");
        }
    }

    #[test]
    fn runtime_changes_match_but_sound_edits_do_not() {
        let first = state(85, "one");
        let played = state(85, "two");
        assert!(same_sound(&plugin(), &first, &played));
        assert!(!same_sound(&plugin(), &first, &state(84, "one")));
        let other_preset = String::from_utf8(played.clone())
            .unwrap()
            .replace("Test.h2p", "Other.h2p");
        assert!(!same_sound(&plugin(), &first, other_preset.as_bytes()));
    }

    #[test]
    fn unknown_plugins_versions_and_incomplete_data_use_exact_bytes() {
        let first = state(85, "one");
        let played = state(85, "two");
        for key in [
            PluginKey {
                id: "other".into(),
                ..plugin()
            },
            PluginKey {
                format: "VST3".into(),
                ..plugin()
            },
        ] {
            assert!(!same_sound(&key, &first, &played));
            assert!(same_sound(&key, &first, &first));
        }
        for (from, to) in [
            ("Rev=16976", "Rev=99999"),
            ("#Vers=10010", "#Vers=99999"),
            ("#AM=TyrellN6", "#AM=Other"),
            ("$$$$1924", "$$$$9999"),
            ("=81976", "=bad"),
            ("\0\0", ""),
        ] {
            let a = String::from_utf8(first.clone()).unwrap().replace(from, to);
            let b = String::from_utf8(played.clone()).unwrap().replace(from, to);
            assert!(!same_sound(&plugin(), a.as_bytes(), b.as_bytes()), "{from}");
            assert!(same_sound(&plugin(), a.as_bytes(), a.as_bytes()));
        }
        assert!(!same_sound(&plugin(), &[255, 1], &[255, 2]));
    }
}
