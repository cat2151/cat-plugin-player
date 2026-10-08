//! Floe 2.0.2 / state v30: exclude CC1's Macro 1 and its preset-dirty flag.
use crate::config::PluginKey;

#[cfg(test)]
#[path = "floe_tests.rs"]
mod tests;

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
    fn array(&mut self) -> Option<()> {
        let size = self.u32()? as usize;
        self.take(size)?;
        Some(())
    }
    fn library_item(&mut self) -> Option<()> {
        self.take(8)?; // Library ID (u64), followed by the instrument/IR ID.
        self.array()
    }
}

// Decode lengths and IDs rather than relying on offsets in one user's preset.
// All non-excluded bytes, including CC mappings and macro destinations, must match.
fn exclusions(state: &[u8]) -> Option<(usize, usize)> {
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
    let mut ids = std::collections::HashSet::new();
    let mut macro_position = None;
    for _ in 0..count {
        let id = r.u32()?;
        if !ids.insert(id) {
            return None;
        }
        let position = r.position;
        let value = f32::from_le_bytes(r.take(4)?.try_into().ok()?);
        if !value.is_finite() {
            return None;
        }
        if id == 101 {
            if !(0.0..=1.0).contains(&value) {
                return None;
            }
            macro_position = Some(position);
        }
    }
    if r.byte()? != 4 {
        return None;
    }
    for _ in 0..4 {
        r.array()?; // Macro name.
        let destinations = r.byte()? as usize;
        r.take(destinations.checked_mul(8)?)?;
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
    let mappings = r.u32()?;
    let mut cc1_macro = false;
    for _ in 0..mappings {
        let cc = r.byte()?;
        let id = r.u32()?;
        if cc > 127 {
            return None;
        }
        if cc == 1 {
            // Other CC1 destinations need their own measured comparison rules.
            if id != 101 || cc1_macro {
                return None;
            }
            cc1_macro = true;
        }
    }
    if !cc1_macro {
        return None;
    }
    r.take(4)?; // Instance reset/keyswitch/seed configuration.
    r.array()?; // Display name.
    r.array()?; // Display category.
    r.take(8)?; // Stable preset UUID.
    let dirty = r.position;
    if r.byte()? > 1 || r.position != state.len() {
        return None;
    }
    Some((macro_position?, dirty))
}

pub(crate) fn same_sweep_sound(plugin: &PluginKey, left: &[u8], right: &[u8]) -> bool {
    if plugin.format != "CLAP" || plugin.id != "com.floe-audio.floe" {
        return false;
    }
    let (Some((lm, ld)), Some((rm, rd))) = (exclusions(left), exclusions(right)) else {
        return false;
    };
    lm == rm
        && ld == rd
        && left.len() == right.len()
        && left[..lm] == right[..rm]
        && left[lm + 4..ld] == right[rm + 4..rd]
}
