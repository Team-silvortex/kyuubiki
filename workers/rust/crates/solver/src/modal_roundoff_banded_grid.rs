// Test-only finite-box construction. Its Wide objective is not an operator receipt
// or an interval proof; only the independent callback can accept a direction.
use super::{triangular_grid::validate_certificate, vector_norm};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use std::cmp::Ordering;

#[path = "modal_roundoff_banded_grid_tests.rs"]
mod tests;

const STATES: usize = 729; // Three choices across the six-coordinate frontier.
const ROOT: usize = usize::MAX;

#[derive(Clone, Copy, Debug)]
pub(super) struct Plan {
    pub(super) payload_bytes: usize,
    pub(super) trials: usize,
    pub(super) terms: usize,
    pub(super) nodes: usize,
}

impl Plan {
    pub(super) fn for_passes(size: usize, passes: usize) -> Result<Self, String> {
        if !(1..=4).contains(&passes) {
            return Err("banded grid requires one to four preselected construction passes".into());
        }
        let mut plan = Self::new(size)?;
        if passes > 1 {
            // Previous shape and Wide center coexist with the next pass's payload.
            plan.payload_bytes += size * (size_of::<Wide>() + size_of::<f64>());
        }
        plan.trials *= passes;
        plan.terms *= passes;
        plan.nodes *= passes;
        if plan.payload_bytes > 4 * 1024 * 1024 || plan.terms > 16_000_000 {
            return Err("banded grid exceeds its cumulative construction budget".into());
        }
        Ok(plan)
    }

    pub(super) fn new(size: usize) -> Result<Self, String> {
        if !(2..=256).contains(&size) {
            return Err("banded grid requires 2..=256 coordinates".into());
        }
        let nodes = size * STATES;
        let trials = 3 * nodes;
        let terms = 7 * (trials + 3 * STATES);
        let payload_bytes = nodes * size_of::<Node>()
            + 2 * STATES * size_of::<Option<State>>()
            + size * (size_of::<[Wide; 7]>() + size_of::<[f64; 3]>() + 2 * size_of::<f64>());
        if payload_bytes > 4 * 1024 * 1024 || terms > 4_000_000 {
            return Err("banded grid exceeds its separate construction budget".into());
        }
        Ok(Self {
            payload_bytes,
            trials,
            terms,
            nodes,
        })
    }
}

#[derive(Clone, Copy)]
struct Node {
    parent: usize,
    value: f64,
}

#[derive(Clone, Copy)]
struct State {
    cost: Wide,
    values: [f64; 6],
    node: usize,
}

#[derive(Default, Debug)]
pub(super) struct Usage {
    pub(super) trials: usize,
    pub(super) terms: usize,
    pub(super) nodes: usize,
    pub(super) peak_states: usize,
}

#[derive(Debug)]
pub(super) struct Proposal {
    pub(super) shape: Vec<f64>,
    pub(super) score: Wide,
    pub(super) usage: Usage,
}

fn bounded(v: Wide) -> bool {
    v.high.is_finite() && v.low.is_finite() && v.low.abs() <= v.high.abs() && v.high.abs() <= 1e50
}

fn cmp(left: Wide, right: Wide) -> Ordering {
    left.high
        .total_cmp(&right.high)
        .then_with(|| left.low.total_cmp(&right.low))
}

fn domains(direction: &[Wide], anchor: usize) -> Result<Vec<[f64; 3]>, String> {
    if anchor >= direction.len() || direction.iter().any(|&v| !bounded(v)) {
        return Err("banded grid requires a finite direction and matching anchor".into());
    }
    let nearest: Vec<_> = direction.iter().map(|v| v.rounded()).collect();
    let norm = vector_norm(nearest.iter().copied())?;
    if !(0.25..=2.0).contains(&norm) || nearest[anchor] == 0.0 {
        return Err("banded grid direction is outside its unchanged norm bounds".into());
    }
    nearest
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            // Magnitude ordering makes sign reversal retain the same choice keys.
            let options = if i == anchor {
                [value; 3]
            } else {
                [
                    value,
                    value.abs().next_down().copysign(value),
                    value.abs().next_up().copysign(value),
                ]
            };
            if options
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 2.0 || (*v != 0.0 && v.abs() < 1e-100))
            {
                return Err("banded grid neighbor is outside its arithmetic bounds".into());
            }
            Ok(options)
        })
        .collect()
}

fn rows(matrix: &[Vec<Wide>], size: usize) -> Result<Vec<[Wide; 7]>, String> {
    if matrix.len() != size || matrix.iter().any(|r| r.len() != size) {
        return Err("banded grid requires a matching square matrix".into());
    }
    let mut maximum = 0.0_f64;
    for (i, row) in matrix.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            if !bounded(v) || (i.abs_diff(j) > 3 && (v.high != 0.0 || v.low != 0.0)) {
                return Err(
                    "banded grid requires finite entries and bandwidth at most three".into(),
                );
            }
            maximum = maximum.max(v.high.abs());
        }
        checkpoint_chunk(SolverStage::ModalRoundoffPrepare, i + 1, size)?;
    }
    if maximum == 0.0 || maximum < 1e-100 {
        return Err("banded grid matrix is outside its binary scaling bounds".into());
    }
    let scale = Wide::from(2.0_f64.powi(maximum.log2().floor() as i32));
    let mut rows = vec![[Wide::default(); 7]; size];
    for (i, row) in matrix.iter().enumerate() {
        for (j, &coefficient) in row
            .iter()
            .enumerate()
            .take((i + 4).min(size))
            .skip(i.saturating_sub(3))
        {
            let entry = coefficient.div(scale);
            if !bounded(entry)
                || (coefficient.high != 0.0 && (entry.high == 0.0 || entry.high.abs() < 1e-100))
            {
                return Err("banded grid binary scaling cannot discard retained entries".into());
            }
            rows[i][j + 3 - i] = entry;
        }
        checkpoint_chunk(SolverStage::ModalRoundoffPrepare, size + i + 1, 2 * size)?;
    }
    Ok(rows)
}

fn row_cost(
    rows: &[[Wide; 7]],
    row: usize,
    mut value: impl FnMut(usize) -> f64,
    usage: &mut Usage,
) -> Result<Wide, String> {
    let mut residual = Wide::default();
    for j in row.saturating_sub(3)..=(row + 3).min(rows.len() - 1) {
        residual = residual.add(rows[row][j + 3 - row].mul(Wide::from(value(j))));
        usage.terms += 1;
    }
    let cost = residual.mul(residual);
    if !bounded(cost) || cost.high < 0.0 || (residual.high != 0.0 && cost.high == 0.0) {
        return Err("banded grid objective is outside its arithmetic bounds".into());
    }
    Ok(cost)
}

pub(super) fn propose(
    matrix: &[Vec<Wide>],
    direction: &[Wide],
    anchor: usize,
) -> Result<Proposal, String> {
    checkpoint(SolverStage::ModalRoundoffPrepare, 0)?;
    let size = direction.len();
    let plan = Plan::new(size)?;
    let choices = domains(direction, anchor)?;
    let rows = rows(matrix, size)?;
    let mut nodes = Vec::with_capacity(plan.nodes);
    let mut current = vec![None; STATES];
    current[0] = Some(State {
        cost: Wide::default(),
        values: [0.0; 6],
        node: ROOT,
    });
    let mut usage = Usage::default();
    checkpoint(SolverStage::ModalRoundoffSearch, 0)?;
    for (k, options) in choices.iter().enumerate() {
        let mut next: Vec<Option<State>> = vec![None; STATES];
        for (key, state) in current
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|s| (i, s)))
        {
            for (choice, &value) in options
                .iter()
                .enumerate()
                .take(if k == anchor { 1 } else { 3 })
            {
                usage.trials += 1;
                checkpoint_chunk(SolverStage::ModalVectorScan, usage.trials, plan.trials)?;
                let added = if k >= 3 {
                    row_cost(
                        &rows,
                        k - 3,
                        |j| {
                            if j == k {
                                value
                            } else {
                                state.values[6 - (k - j)]
                            }
                        },
                        &mut usage,
                    )?
                } else {
                    Wide::default()
                };
                let cost = state.cost.add(added);
                let new_key = (key % (STATES / 3)) * 3 + choice;
                if next[new_key].is_none_or(|old| cmp(cost, old.cost).is_lt()) {
                    let mut values = state.values;
                    values.rotate_left(1);
                    values[5] = value;
                    next[new_key] = Some(State {
                        cost,
                        values,
                        node: state.node,
                    });
                }
            }
        }
        let mut retained = 0;
        for state in next.iter_mut().flatten() {
            nodes.push(Node {
                parent: state.node,
                value: state.values[5],
            });
            state.node = nodes.len() - 1;
            retained += 1;
        }
        usage.peak_states = usage.peak_states.max(retained);
        current = next;
        checkpoint(SolverStage::ModalRoundoffSearch, k + 1)?;
    }
    checkpoint(SolverStage::ModalVectorDot, 0)?;
    let mut best = None;
    for (key, state) in current
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.map(|s| (i, s)))
    {
        let mut cost = state.cost;
        for row in size.saturating_sub(3)..size {
            cost = cost.add(row_cost(
                &rows,
                row,
                |j| state.values[6 - (size - j)],
                &mut usage,
            )?);
        }
        if best.is_none_or(|(old, _)| cmp(cost, old).is_lt()) {
            best = Some((cost, state.node));
        }
        checkpoint(SolverStage::ModalVectorDot, key + 1)?;
    }
    let (score, mut node) = best.ok_or("banded grid has no retained frontier")?;
    checkpoint(SolverStage::ModalVectorUpdate, 0)?;
    let mut shape = vec![0.0; size];
    for k in (0..size).rev() {
        let record = nodes
            .get(node)
            .ok_or("banded grid lost its bounded reconstruction")?;
        shape[k] = record.value;
        node = record.parent;
        checkpoint_chunk(SolverStage::ModalVectorUpdate, size - k, size)?;
    }
    if node != ROOT || shape[anchor].to_bits() != choices[anchor][0].to_bits() {
        return Err("banded grid reconstruction changed its frozen anchor".into());
    }
    usage.nodes = nodes.len();
    if usage.trials > plan.trials || usage.terms > plan.terms || usage.nodes > plan.nodes {
        return Err("banded grid exceeded its measured construction budget".into());
    }
    let norm = vector_norm(shape.iter().copied())?;
    if !(0.25..=2.0).contains(&norm) {
        return Err("banded grid proposal changed its unchanged direction norm bounds".into());
    }
    Ok(Proposal {
        shape,
        score,
        usage,
    })
}

pub(super) fn checked(
    matrix: &[Vec<Wide>],
    direction: &[Wide],
    anchor: usize,
    certificate: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Proposal, String> {
    checked_passes(matrix, direction, anchor, 1, certificate)
}

pub(super) fn propose_passes(
    matrix: &[Vec<Wide>],
    direction: &[Wide],
    anchor: usize,
    passes: usize,
) -> Result<Proposal, String> {
    let plan = Plan::for_passes(direction.len(), passes)?;
    let mut proposal = propose(matrix, direction, anchor)?;
    for _ in 1..passes {
        let center: Vec<_> = proposal.shape.iter().copied().map(Wide::from).collect();
        let mut next = propose(matrix, &center, anchor)?;
        if cmp(next.score, proposal.score).is_gt() {
            return Err("banded grid lost its retained finite-box objective".into());
        }
        next.usage.trials += proposal.usage.trials;
        next.usage.terms += proposal.usage.terms;
        next.usage.nodes += proposal.usage.nodes;
        next.usage.peak_states = next.usage.peak_states.max(proposal.usage.peak_states);
        proposal = next;
    }
    if proposal.usage.trials > plan.trials
        || proposal.usage.terms > plan.terms
        || proposal.usage.nodes > plan.nodes
    {
        return Err("banded grid exceeded its measured cumulative construction budget".into());
    }
    Ok(proposal)
}

pub(super) fn checked_passes(
    matrix: &[Vec<Wide>],
    direction: &[Wide],
    anchor: usize,
    passes: usize,
    mut certificate: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Proposal, String> {
    let proposal = propose_passes(matrix, direction, anchor, passes)?;
    // A second fresh receipt is required even when the first one passes.
    for step in 0..2 {
        checkpoint(SolverStage::ModalRoundoffValidate, step)?;
        let receipt = certificate(&proposal.shape)?;
        validate_certificate(direction.len(), &receipt)?;
        if receipt.0 > 1e-8 {
            return Err(format!(
                "banded grid did not reach its unchanged residual gate ({:e})",
                receipt.0
            ));
        }
    }
    checkpoint(SolverStage::ModalRoundoffValidate, 2)?;
    Ok(proposal)
}
