use crate::linear_algebra::{
    CompressedSparseMatrix, PreparedSpdSolver, SparseMatrix, reduce_sparse_system, stable_l2_norm,
};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions};
pub(crate) use crate::modal_sparse_iteration::inverse_power_iteration;
use crate::modal_tridiagonal::smallest_tridiagonal_eigenpair;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

#[path = "modal_sparse_product.rs"]
mod product;

#[path = "modal_published_shape.rs"]
mod published_shape;

#[path = "modal_normalization.rs"]
mod normalization;

/// Configuration for the linear-memory inverse iteration used by sparse modal solvers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct InverseIterationOptions {
    pub(crate) max_iterations: usize,
    pub(crate) tolerance: f64,
}

impl Default for InverseIterationOptions {
    fn default() -> Self {
        Self {
            max_iterations: 128,
            tolerance: 1.0e-6,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SparseEigenpair {
    pub(crate) eigenvalue: f64,
    #[allow(dead_code)] // Reserved for solver telemetry once modal diagnostics enter the protocol.
    pub(crate) iterations: usize,
    #[allow(dead_code)] // Reserved for solver telemetry once modal diagnostics enter the protocol.
    pub(crate) residual_norm: f64,
    pub(crate) vector: Vec<f64>,
}

/// Applies M^-1/2 K M^-1/2 without materializing a dense mass-normalized matrix.
pub(crate) struct SparseMassNormalizedOperator {
    inverse_mass_sqrt: Vec<f64>,
    stiffness: CompressedSparseMatrix,
    staged_range_safe: bool,
}

impl SparseMassNormalizedOperator {
    pub(crate) fn new(stiffness: &SparseMatrix, mass: &[f64]) -> Result<Self, String> {
        if stiffness.size() != mass.len() {
            return Err("sparse modal stiffness and mass dimensions must match".to_string());
        }
        let mut inverse_mass_sqrt = Vec::with_capacity(mass.len());
        let mut staged_range_safe = true;
        for (index, value) in mass.iter().enumerate() {
            if !value.is_finite() || *value <= 0.0 {
                return Err("sparse modal mass entries must be finite and positive".to_string());
            }
            let inverse = value.sqrt().recip();
            staged_range_safe &= product::staged_factor_is_safe(inverse);
            inverse_mass_sqrt.push(inverse);
            checkpoint_chunk(SolverStage::SparseDiagonalMagnitude, index + 1, mass.len())?;
        }
        let stiffness = stiffness.compress(SpdPreconditioner::Jacobi)?;
        if staged_range_safe {
            for (block, entries) in stiffness.values.chunks(1024).enumerate() {
                staged_range_safe = entries
                    .iter()
                    .all(|&value| product::staged_factor_is_safe(value));
                checkpoint(
                    SolverStage::SparseDiagonalMagnitude,
                    block * 1024 + entries.len(),
                )?;
                if !staged_range_safe {
                    break;
                }
            }
        }
        Ok(Self {
            inverse_mass_sqrt,
            stiffness,
            staged_range_safe,
        })
    }

    /// Uses a Sturm sequence when the normalized operator is genuinely tridiagonal.
    /// This avoids ill-conditioned inverse solves for very long axial frame chains.
    pub(crate) fn smallest_tridiagonal_eigenpair(
        &self,
        tolerance: f64,
    ) -> Option<Result<SparseEigenpair, String>> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Some(Err(
                "tridiagonal modal tolerance must be finite and positive".into(),
            ));
        }
        if let Some(result) = self.uniform_axial_chain_eigenpair(tolerance) {
            return Some(result);
        }
        let size = self.inverse_mass_sqrt.len();
        if size < 2 {
            return None;
        }
        let mut diagonal = vec![0.0; size];
        let mut off_diagonal = vec![0.0; size - 1];
        for row in 0..size {
            for entry in self.stiffness.row_offsets[row]..self.stiffness.row_offsets[row + 1] {
                let column = self.stiffness.columns[entry];
                let value = self.stiffness.values[entry]
                    * self.inverse_mass_sqrt[row]
                    * self.inverse_mass_sqrt[column];
                if column == row {
                    diagonal[row] = value;
                } else if column == row + 1 {
                    off_diagonal[row] = value;
                } else if column + 1 != row {
                    return None;
                }
            }
        }
        if diagonal
            .iter()
            .chain(&off_diagonal)
            .any(|value| !value.is_finite())
            || off_diagonal.contains(&0.0)
        {
            return None;
        }
        Some(smallest_tridiagonal_eigenpair(
            &diagonal,
            &off_diagonal,
            tolerance,
        ))
    }

    /// Recognizes the fixed-free, uniformly discretized axial chain used by scale profiles.
    /// Its first discrete mode has a closed form, which is substantially more stable than
    /// reconstructing a million-component eigenvector from a near-singular recurrence.
    fn uniform_axial_chain_eigenpair(
        &self,
        tolerance: f64,
    ) -> Option<Result<SparseEigenpair, String>> {
        let size = self.inverse_mass_sqrt.len();
        if size < 2 {
            return None;
        }
        let masses = self
            .inverse_mass_sqrt
            .iter()
            .map(|value| value.recip().powi(2))
            .collect::<Vec<_>>();
        let mut diagonal = vec![0.0; size];
        let mut upper = vec![0.0; size - 1];
        let mut lower = vec![0.0; size - 1];
        for row in 0..size {
            for entry in self.stiffness.row_offsets[row]..self.stiffness.row_offsets[row + 1] {
                let column = self.stiffness.columns[entry];
                let value = self.stiffness.values[entry];
                if column == row {
                    diagonal[row] = value;
                } else if column == row + 1 {
                    upper[row] = value;
                } else if column + 1 == row {
                    lower[column] = value;
                } else {
                    return None;
                }
            }
        }
        let stiffness = -upper[0];
        let interior_mass = masses[0];
        if !(stiffness.is_finite()
            && stiffness > 0.0
            && interior_mass.is_finite()
            && interior_mass > 0.0)
        {
            return None;
        }
        let relative_tolerance = 1.0e-10;
        let structure_matches = diagonal.iter().enumerate().all(|(index, value)| {
            let expected = if index + 1 == size {
                stiffness
            } else {
                2.0 * stiffness
            };
            approximately_equal(*value, expected, relative_tolerance)
        }) && upper
            .iter()
            .chain(&lower)
            .all(|value| approximately_equal(*value, -stiffness, relative_tolerance))
            && masses.iter().enumerate().all(|(index, value)| {
                let expected = if index + 1 == size {
                    0.5 * interior_mass
                } else {
                    interior_mass
                };
                approximately_equal(*value, expected, relative_tolerance)
            });
        if !structure_matches {
            return None;
        }
        let theta = std::f64::consts::PI / (2 * size) as f64;
        // For size >= 2 the dimensionless factor is below one; do not form 4k/m first.
        let eigenvalue = (stiffness / interior_mass) * (4.0 * (theta * 0.5).sin().powi(2));
        if !eigenvalue.is_finite() || eigenvalue <= 0.0 {
            return Some(Err(
                "uniform axial modal eigenvalue is not positive and representable".into(),
            ));
        }
        let mut vector = masses
            .iter()
            .enumerate()
            .map(|(index, mass)| mass.sqrt() * ((index + 1) as f64 * theta).sin())
            .collect::<Vec<_>>();
        let norm = l2_norm(&vector);
        if !norm.is_finite() || norm <= 0.0 {
            return Some(Err(
                "uniform axial modal eigenvector is zero or non-finite".into()
            ));
        }
        vector.iter_mut().for_each(|value| *value /= norm);
        Some(self.apply(&vector).and_then(|applied| {
            let residual_norm = stable_l2_norm(
                applied
                    .iter()
                    .zip(&vector)
                    .map(|(value, mode)| value - eigenvalue * mode),
            );
            let residual_scale = l2_norm(&applied).max(eigenvalue.abs());
            // The direct sparse residual subtracts O(n^2)-scaled stiffness terms for this
            // million-segment chain. Keep the general tolerance, with a bounded f64 floor
            // that reflects that cancellation instead of rejecting the closed-form mode.
            let relative_floor = 2.0e-4;
            if !(residual_norm.is_finite() && residual_scale.is_finite() && residual_scale > 0.0)
                || residual_norm > tolerance.max(relative_floor) * residual_scale
            {
                return Err(format!(
                    "uniform axial modal residual is too large ({residual_norm:.6e})"
                ));
            }
            Ok(SparseEigenpair {
                eigenvalue,
                iterations: 0,
                residual_norm,
                vector,
            })
        }))
    }
}

pub(crate) struct ReducedSparseModalSystem {
    pub(crate) free_dofs: Vec<usize>,
    pub(crate) mass: Vec<f64>,
    pub(crate) operator: SparseMassNormalizedOperator,
}

impl ReducedSparseModalSystem {
    /// Factor the sparse normalized system once for both modal search probes.
    /// Solving in physical coordinates can introduce artificial mass-unit conditioning.
    pub(crate) fn prepare_normalized_inverse(&self) -> Result<PreparedSpdSolver, String> {
        PreparedSpdSolver::factor_with_options(
            self.operator.normalized_stiffness()?,
            SpdSolveOptions {
                preconditioner: SpdPreconditioner::IncompleteCholesky,
                progress_interval: None,
            },
        )
    }
}

/// Removes constrained dofs before constructing the mass-normalized sparse operator.
pub(crate) fn reduce_sparse_modal_system(
    stiffness: &SparseMatrix,
    mass: &[f64],
    constrained: &[usize],
) -> Result<ReducedSparseModalSystem, String> {
    if stiffness.size() != mass.len() {
        return Err("sparse modal stiffness and mass dimensions must match".to_string());
    }
    let zero_rhs = vec![0.0; mass.len()];
    let (reduced_stiffness, _, free_dofs) =
        reduce_sparse_system(stiffness, &zero_rhs, constrained)?;
    if free_dofs.is_empty() {
        return Err(
            "sparse modal system must leave at least one free degree of freedom".to_string(),
        );
    }
    let reduced_mass = free_dofs.iter().map(|dof| mass[*dof]).collect::<Vec<_>>();
    Ok(ReducedSparseModalSystem {
        operator: SparseMassNormalizedOperator::new(&reduced_stiffness, &reduced_mass)?,
        mass: reduced_mass,
        free_dofs,
    })
}

fn l2_norm(values: &[f64]) -> f64 {
    stable_l2_norm(values.iter().copied())
}

fn approximately_equal(left: f64, right: f64, relative_tolerance: f64) -> bool {
    if !left.is_finite() || !right.is_finite() {
        return false;
    }
    let scale = left.abs().max(right.abs());
    // An absolute normal-range floor can label different subnormal chains uniform.
    scale == 0.0 || (left / scale - right / scale).abs() <= relative_tolerance
}

#[cfg(test)]
#[path = "modal_sparse_chain_tests.rs"]
mod chain_tests;

#[cfg(test)]
#[path = "modal_mass_inverse_tests.rs"]
mod mass_inverse_tests;

#[cfg(test)]
#[path = "modal_normalization_tests.rs"]
mod normalization_tests;

#[cfg(test)]
mod tests {
    use crate::linear_algebra::{SparseMatrix, add_at};

    use super::{
        InverseIterationOptions, SparseMassNormalizedOperator, inverse_power_iteration,
        reduce_sparse_modal_system,
    };

    #[test]
    fn finds_the_smallest_eigenpair_of_a_diagonal_operator() {
        let diagonal = [2.0, 5.0, 9.0];
        let pair = inverse_power_iteration(
            diagonal.len(),
            InverseIterationOptions {
                max_iterations: 64,
                tolerance: 1.0e-9,
            },
            |vector| Ok(vector.iter().zip(diagonal).map(|(x, d)| x * d).collect()),
            |vector| Ok(vector.iter().zip(diagonal).map(|(x, d)| x / d).collect()),
        )
        .expect("diagonal inverse iteration should converge");
        assert!((pair.eigenvalue - 2.0).abs() < 1.0e-8);
        assert!(pair.residual_norm / pair.eigenvalue < 1.0e-9);
        assert_eq!(pair.vector.len(), diagonal.len());
        assert!(pair.iterations > 0);
    }

    #[test]
    fn inverse_iteration_does_not_false_converge_on_a_tiny_operator() {
        let scale = 1.0e-200;
        let diagonal = [2.0 * scale, 5.0 * scale, 9.0 * scale];
        let pair = inverse_power_iteration(
            diagonal.len(),
            InverseIterationOptions {
                max_iterations: 128,
                tolerance: 1.0e-9,
            },
            |vector| Ok(vector.iter().zip(diagonal).map(|(x, d)| x * d).collect()),
            |vector| Ok(vector.iter().zip(diagonal).map(|(x, d)| x / d).collect()),
        )
        .expect("tiny diagonal inverse iteration should converge relatively");

        assert!((pair.eigenvalue / scale - 2.0).abs() < 1.0e-8);
        assert!(pair.residual_norm / pair.eigenvalue < 1.0e-9);
        assert!(pair.iterations > 0);
    }

    #[test]
    fn inverse_iteration_accepts_a_resolved_tiny_inverse_vector() {
        let scale = 1.0e200;
        let diagonal = [2.0 * scale, 5.0 * scale, 9.0 * scale];
        let pair = inverse_power_iteration(
            diagonal.len(),
            InverseIterationOptions {
                max_iterations: 128,
                tolerance: 1.0e-9,
            },
            |vector| Ok(vector.iter().zip(diagonal).map(|(x, d)| x * d).collect()),
            |vector| Ok(vector.iter().zip(diagonal).map(|(x, d)| x / d).collect()),
        )
        .expect("large diagonal inverse iteration should retain its tiny inverse vector");

        assert!((pair.eigenvalue / scale - 2.0).abs() < 1.0e-8);
        assert!(pair.residual_norm / pair.eigenvalue < 1.0e-9);
        assert!(pair.iterations > 0);
    }

    #[test]
    fn mass_normalized_operator_applies_sparse_stiffness() {
        let mut stiffness = SparseMatrix::new(2);
        add_at(&mut stiffness, 0, 0, 4.0);
        add_at(&mut stiffness, 1, 1, 18.0);
        let operator = SparseMassNormalizedOperator::new(&stiffness, &[2.0, 3.0])
            .expect("positive mass should build an operator");
        let applied = operator.apply(&[1.0, 1.0]).unwrap();
        assert!((applied[0] - 2.0).abs() < 1.0e-12);
        assert!((applied[1] - 6.0).abs() < 1.0e-12);
    }

    #[test]
    fn recognizes_a_uniform_fixed_free_axial_chain() {
        let mut stiffness = SparseMatrix::new(3);
        for (row, column, value) in [
            (0, 0, 4.0),
            (0, 1, -2.0),
            (1, 0, -2.0),
            (1, 1, 4.0),
            (1, 2, -2.0),
            (2, 1, -2.0),
            (2, 2, 2.0),
        ] {
            add_at(&mut stiffness, row, column, value);
        }
        let operator = SparseMassNormalizedOperator::new(&stiffness, &[1.0, 1.0, 0.5])
            .expect("uniform axial chain should build");
        let pair = operator
            .smallest_tridiagonal_eigenpair(1.0e-9)
            .expect("uniform chain should be recognized")
            .expect("closed-form chain mode should solve");
        assert!(pair.eigenvalue > 0.0);
        assert!(pair.residual_norm / pair.eigenvalue < 1.0e-9);
        assert_eq!(pair.iterations, 0);
    }

    #[test]
    fn uniform_chain_avoids_intermediate_overflow_at_large_common_scale() {
        let scale = 1.0e200;
        let mut stiffness = SparseMatrix::new(3);
        for (row, column, value) in [
            (0, 0, 4.0 * scale),
            (0, 1, -2.0 * scale),
            (1, 0, -2.0 * scale),
            (1, 1, 4.0 * scale),
            (1, 2, -2.0 * scale),
            (2, 1, -2.0 * scale),
            (2, 2, 2.0 * scale),
        ] {
            add_at(&mut stiffness, row, column, value);
        }
        let operator = SparseMassNormalizedOperator::new(&stiffness, &[scale, scale, 0.5 * scale])
            .expect("large uniformly scaled chain should build");
        let pair = operator
            .smallest_tridiagonal_eigenpair(1.0e-9)
            .expect("uniform chain should be recognized")
            .expect("large uniformly scaled chain should solve without overflow");
        let theta = std::f64::consts::PI / 6.0;
        let expected = 8.0 * (theta * 0.5).sin().powi(2);

        assert!((pair.eigenvalue / expected - 1.0).abs() < 1.0e-12);
        assert!(pair.residual_norm / pair.eigenvalue < 1.0e-9);
    }

    #[test]
    fn tiny_nonuniform_chain_uses_the_general_tridiagonal_solver() {
        let scale = 1.0e-24;
        let mut stiffness = SparseMatrix::new(2);
        for (row, column, value) in [
            (0, 0, 4.0 * scale),
            (0, 1, -scale),
            (1, 0, -scale),
            (1, 1, 2.0 * scale),
        ] {
            add_at(&mut stiffness, row, column, value);
        }
        let operator = SparseMassNormalizedOperator::new(&stiffness, &[1.0, 1.0])
            .expect("tiny nonuniform chain should build");
        let pair = operator
            .smallest_tridiagonal_eigenpair(1.0e-9)
            .expect("connected chain should use a tridiagonal solver")
            .expect("tiny tridiagonal eigenproblem should solve");
        let expected = 3.0 - 2.0_f64.sqrt();

        assert!((pair.eigenvalue / scale - expected).abs() < 1.0e-9);
        assert!(pair.residual_norm / pair.eigenvalue < 1.0e-9);
    }

    #[test]
    fn reduced_sparse_modal_system_excludes_constrained_dofs() {
        let mut stiffness = SparseMatrix::new(3);
        for index in 0..3 {
            add_at(&mut stiffness, index, index, 2.0);
        }
        let system = reduce_sparse_modal_system(&stiffness, &[1.0, 2.0, 4.0], &[0])
            .expect("one fixed dof should leave a sparse modal system");
        assert_eq!(system.free_dofs, vec![1, 2]);
        let applied = system.operator.apply(&[1.0, 1.0]).unwrap();
        assert!((applied[0] - 1.0).abs() < 1.0e-12);
        assert!((applied[1] - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn reduced_system_solves_the_mass_normalized_inverse() {
        let mut stiffness = SparseMatrix::new(2);
        add_at(&mut stiffness, 0, 0, 4.0);
        add_at(&mut stiffness, 1, 1, 18.0);
        let system = reduce_sparse_modal_system(&stiffness, &[2.0, 3.0], &[])
            .expect("positive diagonal system should reduce");
        let result = system
            .prepare_normalized_inverse()
            .unwrap()
            .solve(&[2.0, 6.0])
            .unwrap();
        assert!((result[0] - 1.0).abs() < 1.0e-12);
        assert!((result[1] - 1.0).abs() < 1.0e-12);
    }
}
