use super::super::Instrument;

/// Floe state v30 with `(name, destination count)` per macro and `(cc, id)` mappings.
pub(crate) fn state(
    values: &[(u32, f32)],
    macros: [(&str, u8); 4],
    mappings: &[(u8, u32)],
) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&0x2a491f93u32.to_le_bytes());
    b.extend_from_slice(&30u16.to_le_bytes());
    b.extend_from_slice(&0x00020002u32.to_le_bytes());
    for _ in 0..3 {
        b.extend_from_slice(&[0, 0]); // No instrument, no velocity-curve points.
        b.extend_from_slice(&[0; 16 + 64 * 8 + 2]);
    }
    b.push(0); // Tags.
    b.extend_from_slice(&[0; 6]); // Author, description, instance ID strings.
    b.extend_from_slice(&(values.len() as u16).to_le_bytes());
    for (id, value) in values {
        b.extend_from_slice(&id.to_le_bytes());
        b.extend_from_slice(&value.to_le_bytes());
    }
    b.push(4);
    for (name, destinations) in macros {
        b.extend_from_slice(&(name.len() as u32).to_le_bytes());
        b.extend_from_slice(name.as_bytes());
        b.push(destinations);
        for d in 0..destinations as u32 {
            b.extend_from_slice(&d.to_le_bytes());
            b.extend_from_slice(&0.5f32.to_le_bytes());
        }
    }
    b.push(0); // No IR.
    b.extend_from_slice(&[0; 8]); // Integrity check, FX order/visibility counts.
    b.extend_from_slice(&(mappings.len() as u32).to_le_bytes());
    for (cc, id) in mappings {
        b.push(*cc);
        b.extend_from_slice(&id.to_le_bytes());
    }
    b.extend_from_slice(&[0; 20]); // Config, display name/category, UUID.
    b.push(0);
    b
}

#[test]
fn macros_with_destinations_carry_names_values_and_ccs() {
    let s = state(
        &[(0, 0.8), (101, 0.25), (102, 1.0), (104, 0.5)],
        [
            ("Macro 1", 1),
            ("Brightness", 2),
            ("Unused", 0),
            (" Macro 4 ", 3),
        ],
        &[(1, 101), (7, 0), (11, 101)],
    );
    let floe = Instrument::of("CLAP", "com.floe-audio.floe").unwrap();
    let movable: Vec<_> = floe
        .movable("CLAP", &s)
        .unwrap()
        .iter()
        .map(|m| m.number)
        .collect();
    assert_eq!(movable, [2, 4]);
    let macros = floe.routed("CLAP", &s).unwrap();
    let summary: Vec<_> = macros
        .iter()
        .map(|m| {
            (
                m.number,
                m.position,
                m.bipolar,
                m.name.as_deref(),
                m.ccs.clone(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            (1, Some(0.25), false, None, vec![1, 11]),
            (2, Some(1.0), false, Some("Brightness"), vec![]),
            (4, Some(0.5), false, None, vec![]),
        ]
    );
}

#[test]
fn identifies_macro_parameters_and_rejects_broken_states() {
    let i = Instrument::of("CLAP", "com.floe-audio.floe").unwrap();
    assert!(i.parameter(3, "Macro 3"));
    assert!(!i.parameter(3, "Macro 30"));
    let good = state(&[(101, 0.5)], [("", 1), ("", 0), ("", 0), ("", 0)], &[]);
    assert!(i.routed("CLAP", &good).is_some());
    assert!(i.routed("CLAP", &good[..good.len() - 1]).is_none());
    let duplicate = state(&[(101, 0.5), (101, 0.5)], Default::default(), &[]);
    assert!(i.routed("CLAP", &duplicate).is_none());
    let cc = state(&[], Default::default(), &[(128, 101)]);
    assert!(i.routed("CLAP", &cc).is_none());
    assert_eq!(i.range(), (0.0, 1.0));
}
