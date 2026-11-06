//! Catalog note identity shared by the resident protocol and saved layers.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub note: u8,
    pub name: String,
    pub velocity: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kit {
    pub layer: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub muted: Vec<u8>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub swing: std::collections::BTreeMap<u8, Swing>,
    pub notes: Vec<Note>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Option<u8>>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Swing {
    pub enabled: bool,
    pub amount: u8,
}

impl Kit {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .swing
            .iter()
            .any(|(&note, s)| note > 127 || !(1..=99).contains(&s.amount))
            || self.muted.iter().any(|&n| n > 127)
            || self.muted.windows(2).any(|n| n[0] >= n[1])
            || self.layer == 0
            || self.notes.is_empty()
            || self.notes.len() > 128
            || self.notes.iter().any(|n| {
                n.note > 127
                    || !(1..=127).contains(&n.velocity)
                    || n.name.len() > 4096
                    || n.name.contains('\0')
            })
            || self.notes.windows(2).any(|n| n[0].note >= n[1].note)
        {
            return Err("Invalid drum kit notes".into());
        }
        if let Some(rows) = &self.rows {
            let assigned: Vec<_> = rows.iter().flatten().copied().collect();
            let unique: std::collections::BTreeSet<_> = assigned.iter().copied().collect();
            if rows.is_empty()
                || rows.len() > 128
                || assigned.iter().any(|&n| n > 127)
                || assigned.len() != unique.len()
            {
                return Err("Invalid drum kit rows".into());
            }
        }
        Ok(())
    }
    pub fn velocity(&self, note: u8) -> u8 {
        self.notes
            .iter()
            .find(|n| n.note == note)
            .map_or(100, |n| n.velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn omitted_row_controls_keep_existing_metadata_unchanged() {
        let legacy =
            serde_json::json!({"layer":1,"notes":[{"note":36,"name":"Kick","velocity":100}]});
        let kit: Kit = serde_json::from_value(legacy.clone()).unwrap();
        kit.validate().unwrap();
        assert!(kit.muted.is_empty());
        assert!(kit.swing.is_empty());
        assert_eq!(serde_json::to_value(kit).unwrap(), legacy);
    }
    #[test]
    fn kit_metadata_roundtrip_and_rejects_ambiguous_assignments() {
        let kit = Kit {
            layer: 1,
            rows: None,
            muted: vec![],
            swing: Default::default(),
            notes: vec![
                Note {
                    note: 0,
                    name: "Original".into(),
                    velocity: 1,
                },
                Note {
                    note: 127,
                    name: String::new(),
                    velocity: 127,
                },
            ],
        };
        kit.validate().unwrap();
        assert_eq!(
            serde_json::from_slice::<Kit>(&serde_json::to_vec(&kit).unwrap()).unwrap(),
            kit
        );
        for notes in [
            vec![],
            vec![kit.notes[0].clone(); 2],
            vec![kit.notes[1].clone(), kit.notes[0].clone()],
        ] {
            assert!(Kit {
                notes,
                ..kit.clone()
            }
            .validate()
            .is_err());
        }
        let mut invalid = kit;
        invalid.notes[0].velocity = 0;
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn optional_edit_rows_keep_source_metadata_and_reject_duplicate_assignments() {
        let mut kit = Kit {
            layer: 1,
            rows: None,
            muted: vec![],
            swing: Default::default(),
            notes: vec![Note {
                note: 36,
                name: "Kick".into(),
                velocity: 100,
            }],
        };
        let legacy = serde_json::to_vec(&kit).unwrap();
        assert!(!String::from_utf8_lossy(&legacy).contains("rows"));
        kit.rows = Some(vec![None, None, Some(38), Some(36)]);
        kit.validate().unwrap();
        assert_eq!(
            serde_json::from_slice::<Kit>(&serde_json::to_vec(&kit).unwrap()).unwrap(),
            kit
        );
        kit.rows = Some(vec![Some(36), Some(36)]);
        assert!(kit.validate().is_err());
    }
}
