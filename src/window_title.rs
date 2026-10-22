//! Main window title naming the current instrument and effects with their presets.
use crate::plugin_list::PluginKind;
use crate::App;
use eframe::egui;

const APP_NAME: &str = "cat plugin player";

impl App {
    pub(crate) fn update_window_title(&self, ctx: &egui::Context) {
        let title = compose(
            self.instances
                .iter()
                .map(|i| (i.plugin.name.as_str(), i.kind, i.preset_name.as_deref())),
        );
        // Setting the native title every frame is redundant work for the OS.
        if ctx.input(|input| input.viewport().title.as_deref() != Some(title.as_str())) {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Title(title));
        }
    }
}

fn compose<'a>(instances: impl Iterator<Item = (&'a str, PluginKind, Option<&'a str>)>) -> String {
    let mut title = APP_NAME.to_owned();
    for (index, (plugin, kind, preset)) in instances.enumerate() {
        title.push_str(if index == 0 { " - " } else { " / " });
        title.push_str(plugin);
        if let Some(preset) = preset {
            title.push_str(": ");
            title.push_str(stem(preset, kind));
        }
    }
    title
}

fn stem(preset: &str, kind: PluginKind) -> &str {
    // The effect catalog names presets `<plugin>: <preset>`.
    let preset = match kind {
        PluginKind::Effect => preset.split_once(": ").map_or(preset, |(_, name)| name),
        _ => preset,
    };
    let name = preset.rsplit(['/', '\\']).next().unwrap_or(preset);
    // `Path::file_stem` would cut program names such as `04 FLUTE 22.6` or `01 <Flute.01>`.
    match name.rsplit_once('.') {
        Some((stem, extension))
            if !stem.is_empty()
                && extension.len() <= 6
                && extension.chars().all(|c| c.is_ascii_alphanumeric())
                && extension.chars().any(|c| c.is_ascii_alphabetic()) =>
        {
            stem
        }
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_every_instance_with_preset_stems_in_chain_order() {
        let title = compose(
            [
                (
                    "Surge XT",
                    PluginKind::Instrument,
                    Some("patches_3rdparty/Xenofish/Pads/Reflections.fxp"),
                ),
                ("Plain", PluginKind::Effect, None),
                (
                    "Surge XT Effects",
                    PluginKind::Effect,
                    Some("Surge XT Effects: Reverb/Large Hall"),
                ),
                (
                    "Dragonfly Hall Reverb",
                    PluginKind::Effect,
                    Some("Dragonfly Hall Reverb: Great Hall"),
                ),
            ]
            .into_iter(),
        );
        assert_eq!(
            title,
            "cat plugin player - Surge XT: Reflections / Plain / \
             Surge XT Effects: Large Hall / Dragonfly Hall Reverb: Great Hall"
        );
    }

    #[test]
    fn keeps_dots_inside_program_names() {
        for (preset, expected) in [
            (
                "Free Sounds/Programs/Mellotron/MkII Flute.sfz",
                "MkII Flute",
            ),
            (
                "Dexed_cart_1.0/!Instruments/Woodwind/FLUTE04.SYX/04 FLUTE 22.6",
                "04 FLUTE 22.6",
            ),
            (
                "Dexed_cart_1.0/!Unsorted/Tim Garrett/TX7-34.SYX/01 <Flute.01>",
                "01 <Flute.01>",
            ),
            ("Bass 1", "Bass 1"),
        ] {
            assert_eq!(stem(preset, PluginKind::Instrument), expected);
        }
    }

    #[test]
    fn is_the_app_name_alone_without_instances() {
        assert_eq!(compose(std::iter::empty()), APP_NAME);
    }
}
