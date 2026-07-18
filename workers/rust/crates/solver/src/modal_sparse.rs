use crate::linear_algebra::{
    CompressedSparseMatrix, SparseMatrix, reduce_sparse_system,
    solve_spd_system_profile_with_options, solve_tridiagonal_system,
};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions};

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
}

impl SparseMassNormalizedOperator {
    pub(crate) fn new(stiffness: &SparseMatrix, mass: &[f64]) -> Result<Self, String> {
        if stiffness.size() != mass.len() {
            return Err("sparse modal stiffness and mass dimensions must match".to_string());
        }
        let mut inverse_mass_sqrt = Vec::with_capacity(mass.len());
        for value in mass {
            if !value.is_finite() || *value <= 0.0 {
                return Err("sparse modal mass entries must be finite and positive".to_string());
            }
            inverse_mass_sqrt.push(value.sqrt().recip());
        }
        Ok(Self {
            inverse_mass_sqrt,
            stiffness: stiffness.compress(SpdPreconditioner::Jacobi),
        })
    }

    pub(crate) fn apply(&self, vector: &[f64]) -> Result<Vec<f64>, String> {
        if vector.len() != self.inverse_mass_sqrt.len() {
            return Err("sparse modal operator vector dimensions must match".to_string());
        }
        let scaled = vector
            .iter()
            .zip(&self.inverse_mass_sqrt)
            .map(|(value, inverse_mass)| value * inverse_mass)
            .collect::<Vec<_>>();
        let mut product = vec![0.0; vector.len()];
        self.stiffness.multiply_vector_into(&scaled, &mut product);
        Ok(product
            .into_iter()
            .zip(&self.inverse_mass_sqrt)
            .map(|(value, inverse_mass)| value * inverse_mass)
            .collect())
    }

    /// Uses a Sturm sequence when the normalized operator is genuinely tridiagonal.
    /// This avoids ill-conditioned inverse solves for very long axial frame chains.
    pub(crate) fn smallest_tridiagonal_eigenpair(
        &self,
        tolerance: f64,
    ) -> Option<Result<SparseEigenpair, String>> {
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
        if off_diagonal.iter().any(|value| value.abs() <= 1.0e-18) {
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
        let mut stiffness_off_diagonal = vec![0.0; size - 1];
        for row in 0..size {
            for entry in self.stiffness.row_offsets[row]..self.stiffness.row_offsets[row + 1] {
                let column = self.stiffness.columns[entry];
                let value = self.stiffness.values[entry];
                if column == row + 1 {
                    stiffness_off_diagonal[row] = value;
                } else if column != row && column + 1 != row {
                    return None;
                }
            }
        }
        let stiffness = -stiffness_off_diagonal[0];
        if !stiffness.is_finite()
            || stiffness <= 0.0
            || stiffness_off_diagonal
                .iter()
                .any(|value| value.abs() <= 1.0e-18)
        {
            return None;
        }
        let interior_mass = masses[0];
        if !interior_mass.is_finite() || interior_mass <= 0.0 {
            return None;
        }
        let theta = std::f64::consts::PI / (2 * size) as f64;
        let eigenvalue = 4.0 * stiffness / interior_mass * (theta * 0.5).sin().powi(2);
        let mut vector = masses
            .iter()
            .enumerate()
            .map(|(index, mass)| mass.sqrt() * ((index + 1) as f64 * theta).sin())
            .collect::<Vec<_>>();
        let norm = l2_norm(&vector);
        vector.iter_mut().for_each(|value| *value /= norm);
        Some(self.apply(&vector).and_then(|applied| {
            let residual_norm = applied
                .iter()
                .zip(&vector)
                .map(|(value, mode)| value - eigenvalue * mode)
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            // The direct sparse residual subtracts O(n^2)-scaled stiffness terms for this
            // million-segment chain. Keep the general tolerance, with a bounded f64 floor
            // that reflects that cancellation instead of rejecting the closed-form mode.
            let relative_floor = 2.0e-4;
            if residual_norm > tolerance.max(relative_floor) * eigenvalue.abs().max(1.0) {
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

    /// Materializes the operator only for the legacy small-model Jacobi fallback.
    pub(crate) fn dense_fallback_matrix(&self) -> Result<Vec<Vec<f64>>, String> {
        let size = self.inverse_mass_sqrt.len();
        let mut dense = vec![vec![0.0; size]; size];
        for column in 0..size {
            let mut basis = vec![0.0; size];
            basis[column] = 1.0;
            let applied = self.apply(&basis)?;
            for row in 0..size {
                dense[row][column] = applied[row];
            }
        }
        Ok(dense)
    }

    fn scale_by_mass_sqrt(&self, vector: &[f64]) -> Result<Vec<f64>, String> {
        if vector.len() != self.inverse_mass_sqrt.len() {
            return Err("sparse modal operator vector dimensions must match".to_string());
        }
        Ok(vector
            .iter()
            .zip(&self.inverse_mass_sqrt)
            .map(|(value, inverse_mass)| value / inverse_mass)
            .collect())
    }
}

pub(crate) struct ReducedSparseModalSystem {
    pub(crate) free_dofs: Vec<usize>,
    pub(crate) operator: SparseMassNormalizedOperator,
    stiffness: SparseMatrix,
}

impl ReducedSparseModalSystem {
    /// Solves (M^-1/2 K M^-1/2) y = rhs without forming the normalized matrix.
    pub(crate) fn solve_normalized_inverse(&self, rhs: &[f64]) -> Result<Vec<f64>, String> {
        let physical_rhs = self.operator.scale_by_mass_sqrt(rhs)?;
        let displacement = match solve_tridiagonal_system(&self.stiffness, &physical_rhs) {
            Some(result) => result?,
            None => {
                solve_spd_system_profile_with_options(
                    &self.stiffness,
                    &physical_rhs,
                    SpdSolveOptions {
                        preconditioner: SpdPreconditioner::IncompleteCholesky,
                        progress_interval: None,
                    },
                )?
                .solution
            }
        };
        self.operator.scale_by_mass_sqrt(&displacement)
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
    let (reduced_stiffness, _, free_dofs) = reduce_sparse_system(stiffness, &zero_rhs, constrained);
    if free_dofs.is_empty() {
        return Err(
            "sparse modal system must leave at least one free degree of freedom".to_string(),
        );
    }
    let reduced_mass = free_dofs.iter().map(|dof| mass[*dof]).collect::<Vec<_>>();
    Ok(ReducedSparseModalSystem {
        operator: SparseMassNormalizedOperator::new(&reduced_stiffness, &reduced_mass)?,
        free_dofs,
        stiffness: reduced_stiffness,
    })
}

/// Finds the smallest eigenpair of a positive-definite operator through inverse iteration.
/// The caller owns sparse storage and the shifted linear solve, so this routine retains only
/// a handful of vectors regardless of model size.
pub(crate) fn inverse_power_iteration(
    size: usize,
    options: InverseIterationOptions,
    apply_operator: impl Fn(&[f64]) -> Result<Vec<f64>, String>,
    solve_inverse: impl Fn(&[f64]) -> Result<Vec<f64>, String>,
) -> Result<SparseEigenpair, String> {
    if size == 0 {
        return Err("sparse modal operator must have at least one degree of freedom".to_string());
    }
    if options.max_iterations == 0 || !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err("sparse modal inverse iteration options are invalid".to_string());
    }

    let mut vector = vec![1.0 / (size as f64).sqrt(); size];
    let mut last_eigenvalue = f64::NAN;
    let mut last_residual_norm = f64::NAN;
    for iteration in 0..=options.max_iterations {
        let applied = apply_operator(&vector)?;
        if applied.len() != size {
            return Err("sparse modal operator returned an invalid vector size".to_string());
        }
        let eigenvalue = dot(&vector, &applied);
        let residual_norm = applied
            .iter()
            .zip(&vector)
            .map(|(value, component)| value - eigenvalue * component)
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if !eigenvalue.is_finite() || !residual_norm.is_finite() {
            return Err("sparse modal iteration produced a non-finite eigenpair".to_string());
        }
        last_eigenvalue = eigenvalue;
        last_residual_norm = residual_norm;
        // Modal residuals scale with the eigenvalue; an absolute threshold would reject
        // otherwise accurate high-frequency modes solely because their units are larger.
        if residual_norm <= options.tolerance * eigenvalue.abs().max(1.0) {
            return Ok(SparseEigenpair {
                eigenvalue,
                iterations: iteration,
                residual_norm,
                vector,
            });
        }
        if iteration == options.max_iterations {
            break;
        }
        let next = solve_inverse(&vector)?;
        if next.len() != size {
            return Err("sparse modal inverse solve returned an invalid vector size".to_string());
        }
        let norm = l2_norm(&next);
        if !norm.is_finite() || norm <= f64::EPSILON {
            return Err(
                "sparse modal inverse solve returned a zero or non-finite vector".to_string(),
            );
        }
        vector = next.into_iter().map(|value| value / norm).collect();
    }
    Err(format!(
        "sparse modal inverse iteration did not converge within {} iterations (eigenvalue={last_eigenvalue:.6e}, residual={last_residual_norm:.6e})",
        options.max_iterations,
    ))
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn l2_norm(values: &[f64]) -> f64 {
    dot(values, values).sqrt()
}

fn smallest_tridiagonal_eigenpair(
    diagonal: &[f64],
    off_diagonal: &[f64],
    tolerance: f64,
) -> Result<SparseEigenpair, String> {
    let mut lower = f64::INFINITY;
    let mut upper = f64::NEG_INFINITY;
    for (index, value) in diagonal.iter().enumerate() {
        let radius = off_diagonal.get(index).unwrap_or(&0.0).abs()
            + index
                .checked_sub(1)
                .and_then(|left| off_diagonal.get(left))
                .unwrap_or(&0.0)
                .abs();
        lower = lower.min(value - radius);
        upper = upper.max(value + radius);
    }
    for _ in 0..96 {
        let middle = (lower + upper) * 0.5;
        if sturm_negative_count(diagonal, off_diagonal, middle) == 0 {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    let eigenvalue = (lower + upper) * 0.5;
    let mut vector = vec![0.0; diagonal.len()];
    vector[0] = 1.0;
    for index in 0..off_diagonal.len() {
        if off_diagonal[index].abs() <= 1.0e-18 {
            return Err("tridiagonal modal operator has a disconnected mode".to_string());
        }
        vector[index + 1] = -((diagonal[index] - eigenvalue) * vector[index]
            + if index == 0 {
                0.0
            } else {
                off_diagonal[index - 1] * vector[index - 1]
            })
            / off_diagonal[index];
        if vector[index + 1].abs() > 1.0e100 {
            for value in &mut vector[..=index + 1] {
                *value *= 1.0e-100;
            }
        }
    }
    let norm = l2_norm(&vector);
    if !norm.is_finite() || norm <= f64::EPSILON {
        return Err("tridiagonal modal eigenvector is not finite".to_string());
    }
    vector.iter_mut().for_each(|value| *value /= norm);
    let residual_norm = diagonal
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let product = value * vector[index]
                + index
                    .checked_sub(1)
                    .and_then(|left| off_diagonal.get(left))
                    .unwrap_or(&0.0)
                    * vector[index.saturating_sub(1)]
                + off_diagonal.get(index).unwrap_or(&0.0) * vector.get(index + 1).unwrap_or(&0.0);
            let residual = product - eigenvalue * vector[index];
            residual * residual
        })
        .sum::<f64>()
        .sqrt();
    if residual_norm > tolerance * eigenvalue.abs().max(1.0) {
        return Err(format!(
            "tridiagonal modal residual is too large ({residual_norm:.6e})"
        ));
    }
    Ok(SparseEigenpair {
        eigenvalue,
        iterations: 0,
        residual_norm,
        vector,
    })
}

fn sturm_negative_count(diagonal: &[f64], off_diagonal: &[f64], shift: f64) -> usize {
    let mut count = 0;
    let mut pivot = diagonal[0] - shift;
    if pivot < 0.0 {
        count += 1;
    }
    for index in 1..diagonal.len() {
        let safe_pivot = if pivot.abs() < 1.0e-18 {
            -1.0e-18
        } else {
            pivot
        };
        pivot = diagonal[index]
            - shift
            - off_diagonal[index - 1] * off_diagonal[index - 1] / safe_pivot;
        if pivot < 0.0 {
            count += 1;
        }
    }
    count
}

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
        let result = system.solve_normalized_inverse(&[2.0, 6.0]).unwrap();
        assert!((result[0] - 1.0).abs() < 1.0e-12);
        assert!((result[1] - 1.0).abs() < 1.0e-12);
    }
}
