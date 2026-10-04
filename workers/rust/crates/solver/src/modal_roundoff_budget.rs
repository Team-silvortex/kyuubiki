use crate::solver_control::{SolverStage, checkpoint};

pub(crate) const MAX_DOFS: usize = 256;
pub(super) const OUTER_STEPS: usize = 4;
pub(super) const FINE_STEPS: usize = 4;
pub(super) const PARTNERS: usize = 4;
pub(super) const MAX_CHECKS: usize = 80;
const MAX_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
const MAX_COMPONENT_VISITS: usize = 350_000_000;

#[derive(Debug)]
pub(super) struct Plan {
    pub(super) payload_bytes: usize,
    pub(super) component_visits: usize,
}

impl Plan {
    pub(super) fn new(size: usize) -> Result<Self, String> {
        checkpoint(SolverStage::ModalRoundoffPrepare, 0)?;
        if !(2..=MAX_DOFS).contains(&size) {
            return Err("modal roundoff correction supports 2..=256 active dofs".into());
        }
        let square = size
            .checked_mul(size)
            .ok_or("modal roundoff budget overflow")?;
        // Conservative simultaneous numeric payload plus row headers/indices.
        // This excludes allocator bookkeeping, the caller's system and RSS.
        let payload_bytes = square
            .checked_mul(8 * std::mem::size_of::<f64>())
            .and_then(|n| n.checked_add(size * 512))
            .ok_or("modal roundoff memory budget overflow")?;
        // Component-visit bound for rank selection, Gram preparation, graph,
        // four bounded search sweeps and dense substitutions, not elapsed time.
        let component_visits = square
            .checked_mul(size)
            .and_then(|n| n.checked_mul(16))
            .and_then(|n| n.checked_add(1024 * square))
            .ok_or("modal roundoff work budget overflow")?;
        let plan = Self {
            payload_bytes,
            component_visits,
        };
        if plan.payload_bytes > MAX_PAYLOAD_BYTES || plan.component_visits > MAX_COMPONENT_VISITS {
            return Err("modal roundoff correction exceeds its resource budget".into());
        }
        Ok(plan)
    }
}

pub(crate) fn eligible(size: usize) -> bool {
    (2..=MAX_DOFS).contains(&size)
}

pub(super) struct Checks(usize);

impl Checks {
    pub(super) fn new() -> Self {
        Self(0)
    }

    pub(super) fn next(&mut self) -> Result<(), String> {
        if self.0 == MAX_CHECKS {
            return Err("modal roundoff correction exhausted its checked-product budget".into());
        }
        checkpoint(SolverStage::ModalRoundoffSearch, self.0)?;
        self.0 += 1;
        Ok(())
    }
}
