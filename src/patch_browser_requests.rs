//! Latest-only intent and the two-key heavy-load gate, independent of native work.
use crate::random_patch_catalog::Candidate;

#[derive(Clone, Debug)]
pub(crate) struct Ticket {
    pub serial: u64,
    pub generation: u64,
    pub candidate: Candidate,
}

#[derive(Default)]
pub(crate) struct Requests {
    serial: u64,
    pub selected: Option<Ticket>,
    pub desired: Option<Ticket>,
    pub confirmation: Option<Ticket>,
    pub preparing: Option<Ticket>,
    pub applying: Option<Ticket>,
    pub applied: Option<Candidate>,
}

pub(crate) fn same(a: &Candidate, b: &Candidate) -> bool {
    a.format == b.format
        && a.plugin_id == b.plugin_id
        && a.path == b.path
        && a.display == b.display
        && a.bundle_path == b.bundle_path
}

impl Requests {
    pub fn select(&mut self, candidate: Candidate, generation: u64) {
        if self
            .selected
            .as_ref()
            .is_some_and(|old| old.generation == generation && same(&old.candidate, &candidate))
        {
            return;
        }
        self.serial += 1;
        let ticket = Ticket {
            serial: self.serial,
            generation,
            candidate,
        };
        self.desired =
            (!ticket.candidate.measurement.is_heavy_offline_load()).then(|| ticket.clone());
        self.selected = Some(ticket);
        self.confirmation = None;
    }

    pub fn space(&mut self) {
        self.confirmation = self
            .selected
            .as_ref()
            .filter(|ticket| ticket.candidate.measurement.is_heavy_offline_load())
            .cloned();
    }

    pub fn enter(&mut self) {
        if let Some(ticket) = self
            .confirmation
            .take()
            .filter(|ticket| self.current(ticket))
        {
            self.desired = Some(ticket);
        }
    }

    pub fn current(&self, ticket: &Ticket) -> bool {
        self.selected.as_ref().is_some_and(|selected| {
            selected.serial == ticket.serial && selected.generation == ticket.generation
        })
    }

    pub fn authorized(&self, ticket: &Ticket) -> bool {
        self.current(ticket)
            && self
                .desired
                .as_ref()
                .is_some_and(|desired| desired.serial == ticket.serial)
    }

    pub fn next(&mut self) -> Option<Ticket> {
        if self.preparing.is_some() || self.applying.is_some() {
            return None;
        }
        let ticket = self.desired.clone()?;
        self.preparing = Some(ticket.clone());
        Some(ticket)
    }

    pub fn ready(&mut self) -> Option<Ticket> {
        let ticket = self.preparing.take()?;
        if !self.authorized(&ticket) {
            return None;
        }
        self.desired = None;
        self.applying = Some(ticket.clone());
        Some(ticket)
    }

    pub fn failed_preparation(&mut self) -> bool {
        let current = self
            .preparing
            .take()
            .is_some_and(|ticket| self.authorized(&ticket));
        if current {
            self.desired = None;
        }
        current
    }

    pub fn finish(&mut self, success: bool) {
        if let Some(ticket) = self.applying.take().filter(|_| success) {
            self.applied = Some(ticket.candidate);
        }
    }

    pub fn invalidate(&mut self) {
        self.selected = None;
        self.desired = None;
        self.confirmation = None;
        // Running preparation must drain; an already-started live transaction finishes.
    }
}

#[cfg(test)]
mod tests;
