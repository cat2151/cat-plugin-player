use super::*;

#[test]
fn labels_strip_directories_and_preset_extensions_but_preserve_name_dots() {
    for (full, plugin, effect, expected) in [
        ("Bass/Bank/日本語 Flute.FXP", "Synth", false, "日本語 Flute"),
        (
            r"X:\presets\Warm.Pad.floe-preset",
            "Synth",
            false,
            "Warm.Pad",
        ),
        (
            "Dexed_cart_1.0/!Unsorted/Tim Garrett/TX7-34.SYX/22 <Flute.22>",
            "Dexed",
            false,
            "22 <Flute.22>",
        ),
        (
            "Dexed_cart_1.0/!Instruments/Woodwind/FLUTE04.SYX/04 FLUTE 22.6",
            "Dexed",
            false,
            "04 FLUTE 22.6",
        ),
        ("Bank.syx/03 FLUTE    4", "Dexed", false, "03 FLUTE    4"),
        (
            "Surge XT Effects: Reverb/Hall/Large Hall.srgfx",
            "Surge XT Effects",
            true,
            "Large Hall",
        ),
        (
            "Dragonfly Hall Reverb: Large Hall",
            "Dragonfly Hall Reverb",
            true,
            "Large Hall",
        ),
        (
            "Voyage Voyage: Shimmer/Long.1",
            "Voyage Voyage",
            true,
            "Long.1",
        ),
        ("  Plain.Name  ", "Synth", false, "Plain.Name"),
        (".hidden", "Synth", false, ".hidden"),
        ("Other Synth: Name", "Synth", true, "Other Synth: Name"),
    ] {
        assert_eq!(stem(full, plugin, effect), expected, "{full}");
    }
}

#[test]
fn missing_stems_fall_back_to_plugin_names() {
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "synth".into(),
        name: "Synth".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/synth.clap".into(),
    };
    for name in [
        None,
        Some(""),
        Some(" \t\n "),
        Some("path/"),
        Some("Synth: "),
    ] {
        let mut capture = FavoriteCapture::from(crate::sequence_pattern::SequencePattern::Steps);
        capture.preset_name = name.map(str::to_owned);
        assert_eq!(default_name(&capture, &plugin, true), "Synth");
    }
}
