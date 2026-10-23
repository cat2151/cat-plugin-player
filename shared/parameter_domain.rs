//! API values, host control positions, and plugin display strings have different meanings.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueSpace {
    Plain,
    Normalized,
}
impl ValueSpace {
    pub fn label(self) -> &'static str {
        match self {
            Self::Plain => "CLAP plain",
            Self::Normalized => "VST3 normalized",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApiValue(f64);
impl ApiValue {
    pub fn get(self) -> f64 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position(f64);
impl Position {
    pub fn new(value: f64) -> Result<Self, String> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(format!("Control position {value} outside [0, 1]"))
        }
    }
    pub fn get(self) -> f64 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Domain {
    space: ValueSpace,
    minimum: f64,
    maximum: f64,
}
impl Domain {
    pub fn new(space: ValueSpace, minimum: f64, maximum: f64) -> Result<Self, String> {
        if !minimum.is_finite()
            || !maximum.is_finite()
            || minimum >= maximum
            || !(maximum - minimum).is_finite()
            || space == ValueSpace::Normalized && (minimum != 0.0 || maximum != 1.0)
        {
            return Err(format!(
                "Invalid {} range [{minimum}, {maximum}]",
                space.label()
            ));
        }
        Ok(Self {
            space,
            minimum,
            maximum,
        })
    }
    pub fn value(self, raw: f64) -> Result<ApiValue, String> {
        if raw.is_finite() && (self.minimum..=self.maximum).contains(&raw) {
            Ok(ApiValue(raw))
        } else {
            Err(format!(
                "{} read {raw} outside [{}, {}]",
                self.space.label(),
                self.minimum,
                self.maximum
            ))
        }
    }
    // CLAP plain uses a linear host position. This is not a physical or perceptual unit conversion.
    pub fn position(self, value: ApiValue) -> Result<Position, String> {
        self.value(value.get())?;
        Position::new((value.0 - self.minimum) / (self.maximum - self.minimum))
    }
    pub fn api(self, position: Position) -> ApiValue {
        ApiValue(if position.0 == 0.0 {
            self.minimum
        } else if position.0 == 1.0 {
            self.maximum
        } else {
            self.minimum + position.0 * (self.maximum - self.minimum)
        })
    }
    pub fn describe(self, raw: f64, plugin_text: Result<&str, &str>) -> String {
        let display = match plugin_text {
            Ok("") => "Plugin ∅".into(),
            Ok(s) => format!("Plugin {s}"),
            Err(e) => format!("Plugin ? ({e})"),
        };
        match self.value(raw) {
            Ok(v) => format!(
                "{display} · pos {:.3} · {} {raw} [{}, {}]",
                self.position(v).expect("validated domain value").get(),
                self.space.label(),
                self.minimum,
                self.maximum
            ),
            Err(e) => format!("{display} · pos ? · {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bipolar_and_non_unit_plain_ranges_roundtrip_without_changing_units() {
        for (lo, hi) in [(-1.0, 1.0), (20.0, 20000.0), (-60.0, 12.0)] {
            let d = Domain::new(ValueSpace::Plain, lo, hi).unwrap();
            for p in [0.0, 0.25, 0.5, 1.0] {
                let v = d.api(Position::new(p).unwrap());
                assert!((d.position(v).unwrap().get() - p).abs() < 1e-12);
            }
            assert_eq!(d.api(Position::new(0.0).unwrap()).get(), lo);
            assert_eq!(d.api(Position::new(1.0).unwrap()).get(), hi);
        }
    }
    #[test]
    fn invalid_reads_and_domains_are_not_silently_clamped() {
        let d = Domain::new(ValueSpace::Plain, 0.0, 1.0).unwrap();
        for v in [-1.0, 2.0, f64::NAN, f64::INFINITY] {
            assert!(d.value(v).is_err());
        }
        assert!(Position::new(-0.1).is_err());
        assert!(Domain::new(ValueSpace::Normalized, -1.0, 1.0).is_err());
        assert!(Domain::new(ValueSpace::Plain, -f64::MAX, f64::MAX).is_err());
        assert!(d.describe(-1.0, Ok("0")).contains("pos ?"));
        assert!(d
            .describe(0.5, Ok("2.3 kHz"))
            .contains("Plugin 2.3 kHz · pos 0.500"));
    }
}
