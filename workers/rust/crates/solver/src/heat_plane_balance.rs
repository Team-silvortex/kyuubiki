use crate::linear_algebra::{
    SparseMatrix, solve_spd_system_profile_with_options, sparse_residual_vector,
};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use crate::solver_postprocess::collect_results;
use kyuubiki_protocol::{HeatPlaneContactResult, HeatPlaneNodeInput};

const BALANCE_TOLERANCE: f64 = 1.0e-8;

pub(super) enum HeatBalanceError {
    Unresolved(String),
    Invalid(String),
}

impl From<String> for HeatBalanceError {
    fn from(error: String) -> Self {
        Self::Invalid(error)
    }
}

pub(super) struct HeatContactRecovery {
    pub contacts: Vec<HeatPlaneContactResult>,
    pub refinement_passes: usize,
    pub refinement_iterations: usize,
}

pub(super) fn recover_with_heat_balance(
    temperatures: &mut [f64],
    free: &[usize],
    matrix: &SparseMatrix,
    rhs: &[f64],
    options: SpdSolveOptions,
    mut recover: impl FnMut(&[f64]) -> Result<Vec<HeatPlaneContactResult>, HeatBalanceError>,
) -> Result<HeatContactRecovery, String> {
    let mut refinement_passes = 0;
    let mut refinement_iterations = 0;
    loop {
        let error = match recover(temperatures) {
            Ok(contacts) => {
                return Ok(HeatContactRecovery {
                    contacts,
                    refinement_passes,
                    refinement_iterations,
                });
            }
            Err(HeatBalanceError::Invalid(error)) => return Err(error),
            Err(HeatBalanceError::Unresolved(error)) => error,
        };
        if refinement_passes == 2 || free.is_empty() {
            return Err(error);
        }
        let solution = collect_results(
            SolverStage::ResidualValidate,
            free.iter().map(|&index| temperatures[index]),
        )?;
        let residual = sparse_residual_vector(matrix, rhs, &solution)?;
        // Correct iterative stopping error through the existing linear solver.
        // Never alter R, constraints, coefficients or the physical acceptance gate.
        let correction = solve_spd_system_profile_with_options(matrix, &residual, options.clone())?;
        for (index, (&node, &delta)) in free.iter().zip(&correction.solution).enumerate() {
            temperatures[node] += delta;
            if !temperatures[node].is_finite() {
                return Err(
                    "thermal contact balance refinement produced a non-finite temperature".into(),
                );
            }
            checkpoint_chunk(SolverStage::ResidualValidate, index + 1, free.len())?;
        }
        refinement_passes += 1;
        refinement_iterations += correction.iterations;
    }
}

pub(super) fn validate_heat_contact_balance<'a>(
    nodes: &[HeatPlaneNodeInput],
    temperatures: &[f64],
    contacts: &[HeatPlaneContactResult],
    triangles: impl IntoIterator<Item = ([usize; 3], &'a [[f64; 3]; 3])>,
    triangle_count: usize,
) -> Result<(), HeatBalanceError> {
    if contacts.is_empty() {
        return Ok(());
    }
    checkpoint(SolverStage::ResidualValidate, 0)?;
    let mut balances = vec![NodalBalance::default(); nodes.len()];
    let mut interface_nodes = vec![false; nodes.len()];
    for (index, contact) in contacts.iter().enumerate() {
        for node in contact.side_a.into_iter().chain(contact.side_b) {
            interface_nodes[node] = !nodes[node].fix_temperature;
        }
        checkpoint_chunk(SolverStage::ResidualValidate, index + 1, contacts.len())?;
    }
    for (index, node) in nodes.iter().enumerate() {
        if interface_nodes[index] {
            balances[index].add(-node.heat_load)?;
        }
        checkpoint_chunk(SolverStage::ResidualValidate, index + 1, nodes.len())?;
    }
    for (index, (map, stiffness)) in triangles.into_iter().enumerate() {
        for (row, &node) in map.iter().enumerate() {
            // Interior bulk equations keep the linear solver's convergence
            // contract. This check independently resolves the interface law.
            if !interface_nodes[node] {
                continue;
            }
            for (column, &other) in map.iter().enumerate() {
                if row != column {
                    // Rebuild physical bulk transfer before mixing it with
                    // large contact coefficients or an absolute temperature.
                    balances[node]
                        .add(stiffness[row][column] * (temperatures[other] - temperatures[node]))?;
                }
            }
        }
        checkpoint_chunk(SolverStage::ResidualValidate, index + 1, triangle_count)?;
    }
    for (index, contact) in contacts.iter().enumerate() {
        for endpoint in 0..2 {
            let a = contact.side_a[endpoint];
            let b = contact.side_b[endpoint];
            let power = contact.nodal_heat_flow_a_to_b_w[endpoint];
            if !nodes[a].fix_temperature {
                balances[a].add(power)?;
            }
            if !nodes[b].fix_temperature {
                balances[b].add(-power)?;
            }
        }
        checkpoint_chunk(SolverStage::ResidualValidate, index + 1, contacts.len())?;
    }
    for (index, balance) in balances.iter().enumerate() {
        let relative = balance.relative_residual();
        if !relative.is_finite() || relative > BALANCE_TOLERANCE {
            return Err(HeatBalanceError::Unresolved(format!(
                "thermal contact heat balance is unresolved at node {index} ({}): relative residual {relative:e} exceeds {BALANCE_TOLERANCE:e}",
                nodes[index].id
            )));
        }
        checkpoint_chunk(SolverStage::ResidualValidate, index + 1, nodes.len())?;
    }
    Ok(())
}

#[derive(Clone, Default)]
struct NodalBalance {
    scale: f64,
    sum: f64,
    correction: f64,
    magnitude: f64,
}

impl NodalBalance {
    fn add(&mut self, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("thermal contact heat balance contains a non-finite contribution".into());
        }
        if value == 0.0 {
            return Ok(());
        }
        if value.abs() > self.scale {
            let ratio = self.scale / value.abs();
            self.sum *= ratio;
            self.correction *= ratio;
            self.magnitude *= ratio;
            self.scale = value.abs();
        }
        // Per-node scaling avoids both an absolute-watt floor and domination
        // by unrelated high-power bodies; compensated sums retain cancellation.
        let term = value / self.scale;
        let next = self.sum + term;
        self.correction += if self.sum.abs() >= term.abs() {
            (self.sum - next) + term
        } else {
            (term - next) + self.sum
        };
        self.sum = next;
        self.magnitude += term.abs();
        Ok(())
    }

    fn relative_residual(&self) -> f64 {
        if self.magnitude == 0.0 {
            0.0
        } else {
            (self.sum + self.correction).abs() / self.magnitude
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BALANCE_TOLERANCE, HeatBalanceError, NodalBalance, recover_with_heat_balance};
    use crate::linear_algebra::{SparseMatrix, add_at};
    use crate::linear_solver_profile::SpdSolveOptions;
    use crate::solver_control::{SolverControl, with_solver_control};

    fn identity() -> SparseMatrix {
        let mut matrix = SparseMatrix::new(1);
        add_at(&mut matrix, 0, 0, 1.0);
        matrix
    }

    #[test]
    fn balance_is_relative_even_for_tiny_or_large_power() {
        for scale in [1e-300, 1e-12, 1.0, 1e12, 1e300] {
            let mut balanced = NodalBalance::default();
            balanced.add(scale).unwrap();
            balanced.add(-scale).unwrap();
            assert_eq!(balanced.relative_residual(), 0.0);
            let mut unbalanced = NodalBalance::default();
            unbalanced.add(scale).unwrap();
            unbalanced.add(-0.9999 * scale).unwrap();
            assert!(unbalanced.relative_residual() > BALANCE_TOLERANCE);
        }
    }

    #[test]
    fn finite_contributions_do_not_overflow_the_accumulator() {
        let mut balance = NodalBalance::default();
        for value in [1e308, 1e308, -1e308, -1e308] {
            balance.add(value).unwrap();
        }
        assert_eq!(balance.relative_residual(), 0.0);
    }

    #[test]
    fn nonfinite_contributions_fail_without_poisoning_a_replay() {
        let mut balance = NodalBalance::default();
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(balance.add(value).unwrap_err().contains("non-finite"));
        }
        balance.add(2.0).unwrap();
        balance.add(-2.0).unwrap();
        assert_eq!(balance.relative_residual(), 0.0);
    }

    #[test]
    fn unresolved_balance_stops_after_two_corrections() {
        let mut calls = 0;
        let result = recover_with_heat_balance(
            &mut [0.0],
            &[0],
            &identity(),
            &[1.0],
            SpdSolveOptions::default(),
            |_| {
                calls += 1;
                Err(HeatBalanceError::Unresolved("unresolved balance".into()))
            },
        );
        assert_eq!(result.err().unwrap(), "unresolved balance");
        assert_eq!(calls, 3);
    }

    #[test]
    fn invalid_recovery_is_not_retried_or_applied() {
        let mut calls = 0;
        let mut temperatures = [0.0];
        let result = recover_with_heat_balance(
            &mut temperatures,
            &[0],
            &identity(),
            &[1.0],
            SpdSolveOptions::default(),
            |_| {
                calls += 1;
                Err(HeatBalanceError::Invalid("non-finite recovery".into()))
            },
        );
        assert_eq!(result.err().unwrap(), "non-finite recovery");
        assert_eq!(calls, 1);
        assert_eq!(temperatures, [0.0]);
    }

    #[test]
    fn cancellation_stops_refinement_and_a_clean_replay_succeeds() {
        let control = SolverControl::default();
        let mut temperatures = [0.0];
        let result = with_solver_control(&control, || {
            recover_with_heat_balance(
                &mut temperatures,
                &[0],
                &identity(),
                &[1.0],
                SpdSolveOptions::default(),
                |_| {
                    control.request_cancel();
                    Err(HeatBalanceError::Unresolved("unresolved balance".into()))
                },
            )
        });
        assert!(result.err().unwrap().contains("cancelled"));
        assert_eq!(temperatures, [0.0]);
        let recovered = recover_with_heat_balance(
            &mut temperatures,
            &[0],
            &identity(),
            &[1.0],
            SpdSolveOptions::default(),
            |values| {
                if values[0] == 1.0 {
                    Ok(Vec::new())
                } else {
                    Err(HeatBalanceError::Unresolved("unresolved balance".into()))
                }
            },
        )
        .unwrap();
        assert_eq!(recovered.refinement_passes, 1);
        assert_eq!(temperatures, [1.0]);
    }
}
