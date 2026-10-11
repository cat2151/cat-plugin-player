use super::*;

fn plugin(index: i32, kind: PluginKind, format: &str) -> PluginInfo {
    PluginInfo {
        index,
        name: format!("Plugin {index}"),
        vendor: "Test".into(),
        format: format.into(),
        id: format!("test.{index}"),
        bundle_path: String::new(),
        kind,
    }
}

#[test]
fn sole_choice_is_per_kind_and_ignores_unknown() {
    let plugins = vec![
        plugin(9, PluginKind::Unknown, "CLAP"),
        plugin(10, PluginKind::Instrument, "CLAP"),
        plugin(11, PluginKind::Effect, "CLAP"),
    ];
    assert_eq!(only_plugin(&plugins, true, PluginKind::Instrument), Some(1));
    assert_eq!(only_plugin(&plugins, true, PluginKind::Effect), Some(2));
    let mut multiple = plugins.clone();
    multiple.push(plugin(12, PluginKind::Instrument, "CLAP"));
    assert_eq!(only_plugin(&multiple, true, PluginKind::Instrument), None);
    assert_eq!(only_plugin(&[], true, PluginKind::Effect), None);
}

#[test]
fn clap_preference_and_effect_browser_support_define_the_choice() {
    let clap = plugin(8, PluginKind::Instrument, "CLAP");
    let mut vst = clap.clone();
    vst.format = "VST3".into();
    let plugins = vec![vst, clap];
    assert_eq!(only_plugin(&plugins, true, PluginKind::Instrument), Some(1));
    assert_eq!(only_plugin(&plugins, false, PluginKind::Instrument), None);
    assert_eq!(
        only_plugin(
            &[plugin(1, PluginKind::Effect, "VST3")],
            true,
            PluginKind::Effect
        ),
        None
    );
}

mod native;
