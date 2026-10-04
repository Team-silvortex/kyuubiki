use super::{
    qr::MAX_GRID_RADIUS,
    triangular_grid::{
        Attempt, GridFit, MAX_CERTIFICATES, Plan as FitPlan, Rejection, require_unit_shape,
        validate_certificate,
    },
    vector_norm,
    wide_factor::bounded,
};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

const MAX_ATTEMPTS: usize = 3;
const MAX_TOTAL_CERTIFICATES: usize = MAX_ATTEMPTS * MAX_CERTIFICATES + 1;
const MAX_TOTAL_VISITS: usize = 1_050_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Order {
    Reverse,
    Natural,
    GridNorm,
}

#[derive(Clone, Copy)]
enum Contract {
    UnitShape,
    InternalDirection,
}

impl Contract {
    fn validate(self, vector: &[f64]) -> Result<(), String> {
        match self {
            Self::UnitShape => require_unit_shape(vector),
            Self::InternalDirection => {
                let norm = vector_norm(vector.iter().copied())?;
                if !norm.is_finite() || !(0.25..=2.0).contains(&norm) {
                    return Err(
                        "canonical internal direction requires a bounded nonunit norm".into(),
                    );
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct Plan {
    pub(super) payload_bytes: usize,
    pub(super) component_visits: usize,
    preflight_visits: usize,
    fit_visits: usize,
}

impl Plan {
    pub(super) fn new(size: usize) -> Result<Self, String> {
        let fit = FitPlan::new(size)?;
        let preflight_visits = 32 * size * size + 128 * size;
        // One factor is live at a time; also account for shared input and ordering.
        let payload_bytes = fit.payload_bytes + 32 * size * size + 256 * size;
        let component_visits = MAX_ATTEMPTS * fit.component_visits + preflight_visits;
        if payload_bytes > 8 * 1024 * 1024 || component_visits > MAX_TOTAL_VISITS {
            return Err("triangular grid portfolio exceeds its aggregate proposal budget".into());
        }
        Ok(Self {
            payload_bytes,
            component_visits,
            preflight_visits,
            fit_visits: fit.component_visits,
        })
    }

    fn canonical(size: usize) -> Result<Self, String> {
        let mut plan = Self::new(size)?;
        // Extra preflight/copy and all bounded coordinate restorations; no extra fits.
        let extra_visits = 4 * size * size + 256 * size;
        plan.preflight_visits += extra_visits;
        plan.component_visits += extra_visits;
        plan.payload_bytes += 16 * size * size + 128 * size;
        if plan.payload_bytes > 8 * 1024 * 1024 || plan.component_visits > MAX_TOTAL_VISITS {
            return Err("canonical grid exceeds its aggregate proposal budget".into());
        }
        Ok(plan)
    }
}

#[derive(Debug, Default)]
pub(super) struct Usage {
    pub(super) attempts: usize,
    pub(super) certificates: usize,
    pub(super) component_visits: usize,
}

struct Budget {
    usage: Usage,
    plan: Plan,
}

impl Budget {
    fn new(plan: Plan) -> Self {
        Self {
            usage: Usage {
                component_visits: plan.preflight_visits,
                ..Usage::default()
            },
            plan,
        }
    }

    fn start_attempt(&mut self) -> Result<(), String> {
        if self.usage.attempts >= MAX_ATTEMPTS
            || self.usage.component_visits + self.plan.fit_visits > self.plan.component_visits
        {
            return Err("triangular grid portfolio exhausted its aggregate attempt budget".into());
        }
        self.usage.attempts += 1;
        self.usage.component_visits += self.plan.fit_visits;
        checkpoint(SolverStage::ModalRoundoffPrepare, self.usage.attempts)?;
        Ok(())
    }

    fn certify(&mut self) -> Result<(), String> {
        if self.usage.certificates >= MAX_TOTAL_CERTIFICATES {
            return Err(
                "triangular grid portfolio exhausted its aggregate certificate budget".into(),
            );
        }
        self.usage.certificates += 1;
        checkpoint(SolverStage::ModalRoundoffSearch, self.usage.certificates)?;
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct Accepted {
    pub(super) shape: Vec<f64>,
    pub(super) order: Order,
    pub(super) anchor: usize,
    pub(super) usage: Usage,
}

struct PreparedOrders {
    anchor: usize,
    orders: [(Order, Vec<usize>); MAX_ATTEMPTS],
}

// Test-only portfolio: typed numerical rejections may continue; faults never do.
pub(super) fn search_unit_shape(
    matrix: &[Vec<Wide>],
    seed: &[f64],
    mass: &[f64],
    tolerance: f64,
    checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Accepted, String> {
    search_with_plan(
        matrix,
        seed,
        mass,
        tolerance,
        Plan::new(matrix.len())?,
        Contract::UnitShape,
        checked,
    )
}

fn search_with_plan(
    matrix: &[Vec<Wide>],
    seed: &[f64],
    mass: &[f64],
    tolerance: f64,
    plan: Plan,
    contract: Contract,
    mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Accepted, String> {
    let mut budget = Budget::new(plan);
    let PreparedOrders { anchor, orders } =
        prepare_orders(matrix, seed, mass, tolerance, contract)?;
    let mut norm_rejections = 0;
    for (policy, order) in orders {
        budget.start_attempt()?;
        let attempt = {
            let fit = GridFit::prepare(matrix, seed, anchor, &order)?;
            let receipt = |v: &[f64]| {
                budget.certify()?;
                checked(v)
            };
            match contract {
                Contract::UnitShape => {
                    fit.attempt_unit_shape(MAX_GRID_RADIUS, tolerance, receipt)?
                }
                Contract::InternalDirection => {
                    fit.attempt_direction(MAX_GRID_RADIUS, tolerance, receipt)?
                }
            }
        };
        let candidate = match attempt {
            Attempt::Accepted(candidate) => candidate,
            Attempt::Rejected(Rejection::Residual(_)) => continue,
            Attempt::Rejected(Rejection::UnitNorm(_)) => {
                norm_rejections += 1;
                continue;
            }
        };
        checkpoint(SolverStage::ModalRoundoffValidate, 3)?;
        budget.certify()?;
        let certificate = checked(&candidate)?;
        validate_certificate(seed.len(), &certificate)?;
        if certificate.0 > tolerance {
            return Err(match contract {
                Contract::UnitShape => {
                    "triangular grid portfolio lost its final physical certificate"
                }
                Contract::InternalDirection => {
                    "triangular grid portfolio lost its final normalized certificate"
                }
            }
            .into());
        }
        contract.validate(&candidate)?;
        if candidate[anchor].to_bits() != seed[anchor].to_bits() {
            return Err("triangular grid portfolio changed its frozen anchor".into());
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 4)?;
        return Ok(Accepted {
            shape: candidate,
            order: policy,
            anchor,
            usage: budget.usage,
        });
    }
    Err(format!(
        "triangular grid portfolio exhausted its bounded policies (attempts={}, certificates={}, norm_rejections={norm_rejections})",
        budget.usage.attempts, budget.usage.certificates
    ))
}

fn prepare_orders(
    matrix: &[Vec<Wide>],
    seed: &[f64],
    mass: &[f64],
    tolerance: f64,
    contract: Contract,
) -> Result<PreparedOrders, String> {
    let anchor = validate_inputs(matrix, seed, mass, tolerance, contract)?;
    let size = matrix.len();
    let natural: Vec<_> = (0..size).filter(|&i| i != anchor).collect();
    let reverse: Vec<_> = natural.iter().copied().rev().collect();
    let mut norms = vec![0.0; size];
    for &j in &natural {
        let grid = seed[j].next_up() - seed[j];
        let mut column = Vec::with_capacity(size);
        for (i, row) in matrix.iter().enumerate() {
            let scaled = row[j].mul(Wide::from(grid)).rounded();
            if !scaled.is_finite() || (row[j].high != 0.0 && scaled == 0.0) {
                return Err("triangular grid portfolio lost a nonzero grid column entry".into());
            }
            column.push(scaled);
            checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
        }
        norms[j] = vector_norm(column.iter().copied())?;
    }
    let mut ranked = natural.clone();
    ranked.sort_by(|&a, &b| norms[a].total_cmp(&norms[b]).then(a.cmp(&b)));
    Ok(PreparedOrders {
        anchor,
        orders: [
            (Order::Reverse, reverse),
            (Order::Natural, natural),
            (Order::GridNorm, ranked),
        ],
    })
}

fn validate_inputs(
    matrix: &[Vec<Wide>],
    seed: &[f64],
    mass: &[f64],
    tolerance: f64,
    contract: Contract,
) -> Result<usize, String> {
    let size = matrix.len();
    if seed.len() != size || mass.len() != size || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err("triangular grid portfolio requires matching data and a positive gate".into());
    }
    contract.validate(seed)?;
    let mut amplitudes = Vec::with_capacity(size);
    for (i, (&value, &m)) in seed.iter().zip(mass).enumerate() {
        if !value.is_finite()
            || value == 0.0
            || !(1e-50..=1e50).contains(&value.abs())
            || !m.is_finite()
            || m <= 0.0
        {
            return Err(
                "triangular grid portfolio requires resolved seed and positive mass".into(),
            );
        }
        amplitudes.push(value.abs() * m.sqrt());
        checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
    }
    let mut anchor = 0;
    for i in 1..size {
        if amplitudes[i] > amplitudes[anchor] {
            anchor = i;
        }
    }
    for (i, row) in matrix.iter().enumerate() {
        if row.len() != size {
            return Err("triangular grid portfolio requires square directions".into());
        }
        for (j, &value) in row.iter().enumerate() {
            if !bounded(value) {
                return Err("triangular grid portfolio requires bounded direction entries".into());
            }
            checkpoint_chunk(SolverStage::ModalVectorUpdate, j + 1, size)?;
        }
        checkpoint_chunk(SolverStage::ModalRoundoffPrepare, i + 1, size)?;
    }
    Ok(anchor)
}

#[path = "modal_roundoff_grid_portfolio_tests.rs"]
mod tests;

#[path = "modal_roundoff_grid_portfolio_benchmark.rs"]
mod benchmark;

#[path = "modal_roundoff_grid_canonical.rs"]
pub(super) mod canonical;
