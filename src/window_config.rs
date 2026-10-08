//! Logical main-window geometry and native editor placement policy.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct WindowConfig {
    #[serde(default)]
    pub main: MainWindow,
    #[serde(default)]
    pub plugin: PluginWindow,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MainWindow {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PluginWindow {
    #[serde(default)]
    pub placement: Placement,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    #[default]
    RightThenBottomRight,
}

impl MainWindow {
    pub fn position(&self) -> Option<eframe::egui::Pos2> {
        let (x, y) = (self.x?, self.y?);
        (x.is_finite() && y.is_finite()).then_some(eframe::egui::pos2(x, y))
    }

    pub fn inner_size(&self) -> eframe::egui::Vec2 {
        let valid = |value: Option<f32>, default| {
            value
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(default)
        };
        eframe::egui::vec2(valid(self.width, 900.0), valid(self.height, 600.0))
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{pos2, vec2};

    #[test]
    fn toml_geometry_supports_integer_and_fractional_logical_coordinates() {
        let config: crate::config::Config =
            toml::from_str("[window.main]\nx = -100\ny = 80.5\nwidth = 1200\nheight = 720.5")
                .unwrap();
        assert_eq!(config.window.main.position(), Some(pos2(-100.0, 80.5)));
        assert_eq!(config.window.main.inner_size(), vec2(1200.0, 720.5));
    }

    #[test]
    fn omitted_and_invalid_dimensions_fall_back_independently() {
        for (source, expected) in [
            ("", vec2(900.0, 600.0)),
            ("width = 1200", vec2(1200.0, 600.0)),
            ("height = 720", vec2(900.0, 720.0)),
            ("width = 0\nheight = -1", vec2(900.0, 600.0)),
            ("width = nan\nheight = inf", vec2(900.0, 600.0)),
            ("width = -inf\nheight = 720", vec2(900.0, 720.0)),
        ] {
            let config: crate::config::Config =
                toml::from_str(&format!("[window.main]\n{source}")).unwrap();
            assert_eq!(config.window.main.inner_size(), expected, "{source}");
        }
    }
}
