//! Floe 2.0.2 / state v30: macros 1-4, their destinations and the saved CC mappings.
use super::Macro;
#[cfg(test)]
#[path = "floe_tests.rs"]
pub(crate) mod tests;

pub const ID: &str = "com.floe-audio.floe";
pub const MACROS: usize = 4;
/// Parameter id of `Macro 1`; macros 2-4 follow it.
pub const FIRST_MACRO: u32 = 101;

/// Byte layout of one decoded state.
pub struct State {
    /// Each parameter id with the byte offset of its f32 value.
    pub parameters: Vec<(u32, usize)>,
    /// Each macro's saved name and destination count.
    pub macros: [(String, usize); MACROS],
    /// `(CC number, parameter id)` pairs.
    pub mappings: Vec<(u8, u32)>,
    /// Byte offset of the preset-dirty flag, the state's last byte.
    pub dirty: usize,
}

impl State {
    pub fn value(&self, state: &[u8], id: u32) -> Option<f32> {
        let &(_, at) = self.parameters.iter().find(|(p, _)| *p == id)?;
        Some(f32::from_le_bytes(state.get(at..at + 4)?.try_into().ok()?))
    }
}

struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn take(&mut self, size: usize) -> Option<&[u8]> {
        let end = self.position.checked_add(size)?;
        let bytes = self.data.get(self.position..end)?;
        self.position = end;
        Some(bytes)
    }

    fn byte(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<()> {
        let size = self.u16()? as usize;
        std::str::from_utf8(self.take(size)?).ok()?;
        Some(())
    }
    fn array(&mut self) -> Option<&[u8]> {
        let size = self.u32()? as usize;
        self.take(size)
    }
    fn library_item(&mut self) -> Option<()> {
        self.take(8)?; // Library ID (u64), followed by the instrument/IR ID.
        self.array()?;
        Some(())
    }
}

/// Decodes lengths and IDs from the start to the last byte rather than relying on
/// offsets in one user's preset.
pub fn parse(state: &[u8]) -> Option<State> {
    let mut r = Reader {
        data: state,
        position: 0,
    };
    if r.u32()? != 0x2a491f93 || r.u16()? != 30 || r.u32()? != 0x00020002 {
        return None;
    }
    for _ in 0..3 {
        match r.byte()? {
            1 => r.library_item()?,
            0 | 2..=4 => (),
            _ => return None,
        }
        let points = r.byte()? as usize;
        r.take(points.checked_mul(12)?)?; // Three f32 values per velocity-curve point.
        r.take(16 + 64 * 8 + 2)?; // Harmony bitset, arp steps, slice arp config.
    }
    let tags = r.byte()?;
    for _ in 0..tags {
        r.string()?;
    }
    r.string()?; // Author.
    r.string()?; // Description.
    r.string()?; // Instance ID.
    let count = r.u16()?;
    let mut parameters: Vec<(u32, usize)> = Vec::with_capacity(count.into());
    let mut ids = std::collections::HashSet::new();
    for _ in 0..count {
        let id = r.u32()?;
        if !ids.insert(id) {
            return None;
        }
        let position = r.position;
        if !f32::from_le_bytes(r.take(4)?.try_into().ok()?).is_finite() {
            return None;
        }
        parameters.push((id, position));
    }
    if r.byte()? != MACROS as u8 {
        return None;
    }
    let mut macros: [(String, usize); MACROS] = Default::default();
    for slot in &mut macros {
        slot.0 = String::from_utf8_lossy(r.array()?).into_owned();
        slot.1 = r.byte()? as usize;
        r.take(slot.1.checked_mul(8)?)?; // Parameter id (u32) and amount (f32).
    }
    match r.byte()? {
        1 => r.library_item()?,
        0 => (),
        _ => return None,
    }
    if r.u32()? != 0 {
        return None;
    } // Integrity check number.
    let effects = r.u16()? as usize;
    r.take(effects)?;
    let visible = r.u16()? as usize;
    r.take(visible)?;
    let count = r.u32()?;
    let mut mappings = Vec::new();
    for _ in 0..count {
        let cc = r.byte()?;
        let id = r.u32()?;
        if cc > 127 {
            return None;
        }
        mappings.push((cc, id));
    }
    r.take(4)?; // Instance reset/keyswitch/seed configuration.
    r.array()?; // Display name.
    r.array()?; // Display category.
    r.take(8)?; // Stable preset UUID.
    let dirty = r.position;
    if r.byte()? > 1 || r.position != state.len() {
        return None;
    }
    Some(State {
        parameters,
        macros,
        mappings,
        dirty,
    })
}

/// Macros with at least one destination, as Floe's own destination list counts them.
pub(super) fn routed(state: &[u8]) -> Option<Vec<Macro>> {
    let s = parse(state)?;
    Some(
        s.macros
            .iter()
            .enumerate()
            .filter(|(_, (_, destinations))| *destinations > 0)
            .map(|(i, (name, _))| {
                let id = FIRST_MACRO + i as u32;
                let name = name.trim();
                Macro {
                    number: i + 1,
                    position: s
                        .value(state, id)
                        .map(f64::from)
                        .filter(|v| (0.0..=1.0).contains(v)),
                    bipolar: false,
                    name: (!name.is_empty() && name != format!("Macro {}", i + 1))
                        .then(|| name.to_owned()),
                    ccs: s
                        .mappings
                        .iter()
                        .filter(|&&(_, p)| p == id)
                        .map(|&(cc, _)| cc)
                        .collect(),
                }
            })
            .collect(),
    )
}
