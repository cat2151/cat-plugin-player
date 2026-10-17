//! Row labels that always keep the preset stem, shortening the parts before it.

pub(super) const PLUGIN_SEPARATOR: &str = " — ";
const ELLIPSIS: &str = "…";

/// `(prefix, name)` of `plugin — folder/…/stem` fitted into `room`.
///
/// The stem stays whole. Every part before it first shrinks to one character
/// and `…`; parts that still do not fit are dropped from the left. The width
/// left over then restores the parts nearest the stem first.
pub(super) fn fit(
    plugin: Option<&str>,
    name: &str,
    room: f32,
    char_width: impl Fn(char) -> f32,
) -> (String, String) {
    let width = |text: &str| text.chars().map(&char_width).sum::<f32>();
    let (folders, stem) = name
        .rsplit_once('/')
        .map_or((None, name), |(folders, stem)| (Some(folders), stem));
    let parts: Vec<_> = plugin
        .into_iter()
        .chain(folders.into_iter().flat_map(|folders| folders.split('/')))
        .map(|text| Part::new(text, &char_width))
        .collect();
    let ellipsis = width(ELLIPSIS);
    let after = |at: usize| {
        width(if plugin.is_some() && at == 0 {
            PLUGIN_SEPARATOR
        } else {
            "/"
        })
    };
    let stem_width = width(stem);
    let total = |kept: &[Option<usize>]| {
        stem_width
            + kept
                .iter()
                .enumerate()
                .filter_map(|(at, kept)| {
                    kept.map(|kept| parts[at].width(kept, ellipsis) + after(at))
                })
                .sum::<f32>()
    };
    let mut kept: Vec<_> = parts.iter().map(|part| Some(part.len())).collect();
    if total(&kept) > room {
        kept = parts.iter().map(|part| Some(part.len().min(1))).collect();
        for at in 0..kept.len() {
            if total(&kept) <= room {
                break;
            }
            kept[at] = None;
        }
        if total(&kept) > room {
            return (
                String::new(),
                Part::new(stem, &char_width).squeeze(room, ellipsis),
            );
        }
        for at in (0..kept.len()).rev() {
            while kept[at].is_some_and(|shown| shown < parts[at].len()) {
                kept[at] = kept[at].map(|shown| shown + 1);
                if total(&kept) > room {
                    kept[at] = kept[at].map(|shown| shown - 1);
                    break;
                }
            }
        }
    }
    let shown = |at: usize| kept[at].map(|kept| parts[at].show(kept));
    let first_folder = usize::from(plugin.is_some());
    let prefix = (first_folder == 1)
        .then(|| shown(0))
        .flatten()
        .map(|plugin| plugin + PLUGIN_SEPARATOR)
        .unwrap_or_default();
    let mut path: Vec<_> = (first_folder..parts.len()).filter_map(shown).collect();
    path.push(stem.to_string());
    (prefix, path.join("/"))
}

/// One path part with the width of each of its leading-character counts.
struct Part<'a> {
    text: &'a str,
    /// `widths[k]` is the width of the first `k` characters.
    widths: Vec<f32>,
}

impl<'a> Part<'a> {
    fn new(text: &'a str, char_width: impl Fn(char) -> f32) -> Self {
        let widths = std::iter::once(0.0)
            .chain(text.chars().scan(0.0, |sum, c| {
                *sum += char_width(c);
                Some(*sum)
            }))
            .collect();
        Self { text, widths }
    }

    fn len(&self) -> usize {
        self.widths.len() - 1
    }

    fn width(&self, kept: usize, ellipsis: f32) -> f32 {
        self.widths[kept] + if kept < self.len() { ellipsis } else { 0.0 }
    }

    fn show(&self, kept: usize) -> String {
        let head: String = self.text.chars().take(kept).collect();
        if kept < self.len() {
            // "Voyage…", not "Voyage …".
            head.trim_end().to_string() + ELLIPSIS
        } else {
            head
        }
    }

    /// The longest head ending in `…` that fits, keeping at least one character.
    fn squeeze(&self, room: f32, ellipsis: f32) -> String {
        let kept = (1..self.len())
            .rev()
            .find(|&kept| self.width(kept, ellipsis) <= room)
            .unwrap_or(1.min(self.len()));
        self.show(kept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fit_chars(plugin: Option<&str>, name: &str, room: usize) -> String {
        let (prefix, name) = fit(plugin, name, room as f32, |_| 1.0);
        prefix + &name
    }

    #[test]
    fn whole_label_is_kept_when_it_fits() {
        let label = "Surge XT Effects — Reverb 2/Cathedral 1";
        let fitted = fit_chars(Some("Surge XT Effects"), "Reverb 2/Cathedral 1", 39);
        assert_eq!(fitted, label);
        let (prefix, name) = fit(
            Some("Surge XT Effects"),
            "Reverb 2/Cathedral 1",
            39.0,
            |_| 1.0,
        );
        assert_eq!(
            (prefix.as_str(), name.as_str()),
            ("Surge XT Effects — ", "Reverb 2/Cathedral 1")
        );
    }

    #[test]
    fn nearest_parts_regain_width_first_and_stem_stays_whole() {
        let name = "Airwindows/Saturation And More/Single-Ended Triode";
        assert_eq!(
            fit_chars(Some("Surge XT Effects"), name, 35),
            "S… — A…/Satura…/Single-Ended Triode"
        );
        assert_eq!(
            fit_chars(Some("Surge XT Effects"), "Reverb 2/Cathedral 1", 35),
            "Surge XT Ef… — Reverb 2/Cathedral 1"
        );
    }

    #[test]
    fn parts_are_dropped_from_the_left_then_the_stem_is_squeezed() {
        let name = "Airwindows/Saturation And More/Single-Ended Triode";
        assert_eq!(
            fit_chars(Some("Surge XT Effects"), name, 25),
            "A…/S…/Single-Ended Triode"
        );
        assert_eq!(
            fit_chars(Some("Surge XT Effects"), name, 24),
            "Sat…/Single-Ended Triode"
        );
        assert_eq!(
            fit_chars(Some("Surge XT Effects"), name, 19),
            "Single-Ended Triode"
        );
        assert_eq!(fit_chars(Some("Surge XT Effects"), name, 10), "Single-En…");
        assert_eq!(fit_chars(None, "Hall", 1), "H…");
    }

    #[test]
    fn single_part_names_and_one_character_parts() {
        assert_eq!(fit_chars(None, "Hall", 10), "Hall");
        assert_eq!(fit_chars(Some("Shu"), "Plate", 12), "Shu — Plate");
        assert_eq!(fit_chars(Some("Shu"), "Plate", 10), "S… — Plate");
        assert_eq!(fit_chars(None, "A/Long Folder/Stem", 9), "A/L…/Stem");
        assert_eq!(fit_chars(None, "Long Folder/Stem", 11), "Long…/Stem");
    }
}
