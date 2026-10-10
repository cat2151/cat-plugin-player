//! Prepare saved playback before changing plugin state or the live chain.
use crate::{favorites_store::Favorite, mml_input::MmlInput, SequencePattern};

pub(crate) struct Playback {
    pub input: MmlInput,
    pub selected: SequencePattern,
    pub transport: SequencePattern,
}

impl Favorite {
    pub(crate) fn prepare_playback(
        &self,
        input: &MmlInput,
        current: SequencePattern,
        selected: SequencePattern,
        rate: u32,
    ) -> Result<Playback, String> {
        let mut input = input.clone();
        if self.effect {
            return Ok(Playback {
                input,
                selected,
                transport: current,
            });
        }
        let selected = self
            .selected_sequence
            .unwrap_or_else(|| self.sequence_pattern.selection(current.selection(selected)));
        if selected == SequencePattern::Off {
            return Err("Invalid saved sequence selection".into());
        }
        if self.selected_sequence.is_some()
            && self.sequence_pattern != SequencePattern::Off
            && self.sequence_pattern != selected
        {
            return Err("Saved sequence and selection disagree".into());
        }
        if let Some(mml) = &self.mml {
            input.confirmed.clone_from(mml);
            input.buffer.clone_from(mml);
        }
        // Legacy snapshots have no text; Custom without a saved source is invalid.
        if self.mml.is_none()
            && (self.sequence_pattern == SequencePattern::Custom
                || self.selected_sequence == Some(SequencePattern::Custom))
        {
            return Err("Custom favorite requires saved MML".into());
        }
        input.restore_phrase(rate, selected)?;
        input.cancel();
        let transport = if current == SequencePattern::Off {
            current
        } else if self.selected_sequence.is_some() {
            selected
        } else {
            self.playback_pattern(current)
        };
        Ok(Playback {
            input,
            selected,
            transport,
        })
    }
}

impl crate::App {
    pub(crate) fn apply_playback(&mut self, playback: Playback) {
        self.mml_input = playback.input;
        self.selected_sequence = playback.selected;
        self.sequence_pattern = playback.transport;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepare_saved_mml_preserves_transport_and_rejects_missing_custom() {
        let mut favorite: Favorite = toml::from_str(
            r#"id = "a"
name = "test"
effect = false
sequence_pattern = "off"
selected_sequence = "custom"
mml = "C F G"
[plugin]
format = "CLAP"
id = "synth"
"#,
        )
        .unwrap();
        for current in [
            SequencePattern::Off,
            SequencePattern::Steps,
            SequencePattern::Custom,
        ] {
            let prepared = favorite
                .prepare_playback(&MmlInput::default(), current, SequencePattern::Steps, 48000)
                .unwrap();
            assert_eq!(prepared.input.confirmed, "C F G");
            assert!(prepared.input.phrase.is_some());
            assert_eq!(prepared.selected, SequencePattern::Custom);
            assert_eq!(
                prepared.transport,
                if current == SequencePattern::Off {
                    current
                } else {
                    SequencePattern::Custom
                }
            );
        }
        favorite.mml = None;
        assert!(favorite
            .prepare_playback(
                &MmlInput::default(),
                SequencePattern::Steps,
                SequencePattern::Steps,
                48000
            )
            .is_err());
        favorite.mml = Some(" ".into());
        assert!(favorite
            .prepare_playback(
                &MmlInput::default(),
                SequencePattern::Steps,
                SequencePattern::Steps,
                48000
            )
            .is_err());
    }

    #[test]
    fn inconsistent_saved_pattern_is_rejected_without_changing_live_input() {
        let mut favorite: Favorite = toml::from_str(
            r#"id = "a"
name = "test"
effect = false
sequence_pattern = "custom"
selected_sequence = "steps"
mml = ""
[plugin]
format = "CLAP"
id = "synth"
"#,
        )
        .unwrap();
        let mut input = MmlInput::default();
        input.confirmed = "o5 l8 c".into();
        input
            .restore_phrase(48000, SequencePattern::Custom)
            .unwrap();
        let phrase = input.phrase.clone().unwrap();
        for text in ["", "C F G"] {
            favorite.mml = Some(text.into());
            for current in [SequencePattern::Off, SequencePattern::Custom] {
                assert!(favorite
                    .prepare_playback(&input, current, SequencePattern::Custom, 48000)
                    .is_err());
                assert_eq!(input.confirmed, "o5 l8 c");
                assert!(std::sync::Arc::ptr_eq(
                    input.phrase.as_ref().unwrap(),
                    &phrase
                ));
            }
        }
    }
}
