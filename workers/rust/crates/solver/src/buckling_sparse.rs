use crate::buckling_math::{GeneralizedEigenpair, generalized_eigenpairs};
use crate::linear_algebra::{CompressedSparseMatrix, SparseMatrix, add_at, sparse_to_dense};
use crate::linear_banded::SymmetricBandCholesky;
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions};
use crate::linear_spd::solve_spd_compressed;
use crate::modal_math::jacobi_eigenpairs;

const MAX_SUBSPACE_ITERATIONS: usize = 96;
const RELATIVE_RESIDUAL_TOLERANCE: f64 = 1.0e-6;
const LARGE_SYSTEM_RESIDUAL_FLOOR: f64 = 2.0e-3;
const LARGE_SYSTEM_FACTOR_TOLERANCE: f64 = 1.0e-6;
const DENSE_BUCKLING_DOFS: usize = 512;

pub(crate) fn hybrid_generalized_eigenpairs(
    stiffness: &SparseMatrix,
    geometric: &SparseMatrix,
    mode_count: usize,
) -> Result<Vec<GeneralizedEigenpair>, String> {
    if stiffness.size() <= DENSE_BUCKLING_DOFS {
        generalized_eigenpairs(
            &sparse_to_dense(stiffness),
            &sparse_to_dense(geometric),
            mode_count,
        )
    } else {
        sparse_generalized_eigenpairs(stiffness, geometric, mode_count)
    }
}

/// Finds the lowest positive buckling factors without materializing K^-1 Kg.
/// K-inner-product subspace iteration keeps the projected problem symmetric.
pub(crate) fn sparse_generalized_eigenpairs(
    stiffness: &SparseMatrix,
    geometric: &SparseMatrix,
    mode_count: usize,
) -> Result<Vec<GeneralizedEigenpair>, String> {
    validate(stiffness, geometric)?;
    let requested = mode_count.max(1).min(stiffness.size());
    // Oversampling is required even for one requested mode. A scalar power
    // iteration stalls when the first two buckling factors are clustered,
    // while a small block isolates that cluster from the rest of the spectrum.
    let subspace_size = (requested * 2).max(requested + 2).min(stiffness.size());
    let elastic = stiffness.compress(SpdPreconditioner::Jacobi);
    let geometric = geometric.compress(SpdPreconditioner::Jacobi);
    let scaling = diagonal_scaling(stiffness)?;
    let scaled_stiffness = symmetrically_scaled(stiffness, &scaling);
    let scaled_elastic = scaled_stiffness.compress(SpdPreconditioner::SymmetricGaussSeidel);
    let banded_elastic = SymmetricBandCholesky::try_factor(&scaled_stiffness, 8_000_000)?;
    let mut basis = k_orthonormalize(initial_subspace(stiffness.size(), subspace_size), &elastic);
    if basis.len() < requested {
        return Err("buckling sparse eigensolver could not initialize its subspace".to_string());
    }

    let solve_options = SpdSolveOptions {
        preconditioner: SpdPreconditioner::SymmetricGaussSeidel,
        progress_interval: None,
    };
    let mut last_relative_residual = f64::INFINITY;
    let mut previous_factors: Vec<f64> = Vec::new();
    let mut last_factor_change = f64::INFINITY;
    for _ in 0..MAX_SUBSPACE_ITERATIONS {
        let mut candidates: Vec<Vec<f64>> = Vec::with_capacity(basis.len());
        let mut max_scaled_solution_norm = 0.0_f64;
        let mut max_solve_iterations = 0;
        for vector in &basis {
            let mut geometric_product = multiply(&geometric, vector);
            if !normalize_euclidean(&mut geometric_product) {
                continue;
            }
            let mut scaled_rhs = geometric_product
                .iter()
                .zip(&scaling)
                .map(|(value, scale)| value * scale)
                .collect::<Vec<_>>();
            if !normalize_euclidean(&mut scaled_rhs) {
                continue;
            }
            let (scaled_solution, solve_iterations) = if let Some(factor) = &banded_elastic {
                (
                    solve_banded_refined(factor, &scaled_elastic, &scaled_rhs)?,
                    0,
                )
            } else {
                let solved = solve_spd_compressed(
                    &scaled_elastic,
                    &scaled_rhs,
                    &scaled_stiffness,
                    &solve_options,
                )?;
                (solved.solution, solved.iterations)
            };
            max_scaled_solution_norm = max_scaled_solution_norm.max(l2_norm(&scaled_solution));
            max_solve_iterations = max_solve_iterations.max(solve_iterations);
            candidates.push(
                scaled_solution
                    .into_iter()
                    .zip(&scaling)
                    .map(|(value, scale)| value * scale)
                    .collect::<Vec<_>>(),
            );
        }
        let max_candidate_norm = candidates
            .iter()
            .map(|value| l2_norm(value))
            .fold(0.0, f64::max);
        let max_candidate_energy = candidates
            .iter()
            .map(|value| dot(value, &multiply(&elastic, value)))
            .fold(0.0, f64::max);
        basis = k_orthonormalize(candidates, &elastic);
        if basis.len() < requested {
            return Err(format!(
                "buckling reference load pattern has too few positive independent modes (candidate_norm={max_candidate_norm:.6e}, candidate_k_energy={max_candidate_energy:.6e}, scaled_solution_norm={max_scaled_solution_norm:.6e}, solve_iterations={max_solve_iterations})"
            ));
        }

        let geometric_products = basis
            .iter()
            .map(|vector| multiply(&geometric, vector))
            .collect::<Vec<_>>();
        let projected = projected_geometric(&basis, &geometric_products);
        let mut reciprocal_pairs = jacobi_eigenpairs(projected)
            .into_iter()
            .filter(|(value, _)| value.is_finite() && *value > 1.0e-12)
            .collect::<Vec<_>>();
        reciprocal_pairs.sort_by(|left, right| right.0.total_cmp(&left.0));
        if reciprocal_pairs.len() < requested {
            return Err(
                "buckling reference load pattern has too few positive finite modes".to_string(),
            );
        }

        let mut rotated_basis = Vec::with_capacity(reciprocal_pairs.len());
        let mut pairs = Vec::with_capacity(requested);
        last_relative_residual = 0.0;
        for (index, (reciprocal, coefficients)) in reciprocal_pairs.into_iter().enumerate() {
            let vector = combine(&basis, &coefficients);
            if index < requested {
                let elastic_product = multiply(&elastic, &vector);
                let geometric_product = multiply(&geometric, &vector);
                let eigenvalue = reciprocal.recip();
                let residual_norm = residual_norm(&elastic_product, &geometric_product, eigenvalue);
                let scale = l2_norm(&elastic_product)
                    .max(eigenvalue.abs() * l2_norm(&geometric_product))
                    .max(1.0);
                last_relative_residual = last_relative_residual.max(residual_norm / scale);
                pairs.push(GeneralizedEigenpair {
                    eigenvalue,
                    vector: vector.clone(),
                    residual_norm,
                });
            }
            rotated_basis.push(vector);
        }
        let factors = pairs.iter().map(|pair| pair.eigenvalue).collect::<Vec<_>>();
        last_factor_change = if previous_factors.len() == factors.len() {
            factors
                .iter()
                .zip(&previous_factors)
                .map(|(current, previous)| (current - previous).abs() / current.abs().max(1.0))
                .fold(0.0, f64::max)
        } else {
            f64::INFINITY
        };
        let converged_at_roundoff_floor = stiffness.size() > 4_096
            && last_relative_residual <= LARGE_SYSTEM_RESIDUAL_FLOOR
            && last_factor_change <= LARGE_SYSTEM_FACTOR_TOLERANCE;
        if last_relative_residual <= RELATIVE_RESIDUAL_TOLERANCE || converged_at_roundoff_floor {
            return Ok(pairs);
        }
        previous_factors = factors;
        basis = rotated_basis;
    }

    Err(format!(
        "buckling sparse subspace iteration did not converge within {MAX_SUBSPACE_ITERATIONS} iterations (relative residual={last_relative_residual:.6e}, factor change={last_factor_change:.6e})"
    ))
}

fn validate(stiffness: &SparseMatrix, geometric: &SparseMatrix) -> Result<(), String> {
    if stiffness.size() == 0 || stiffness.size() != geometric.size() {
        return Err(
            "buckling sparse generalized matrices must be square, non-empty, and equal-sized"
                .to_string(),
        );
    }
    Ok(())
}

fn diagonal_scaling(matrix: &SparseMatrix) -> Result<Vec<f64>, String> {
    (0..matrix.size())
        .map(|row| {
            let diagonal = matrix
                .row_entries(row)
                .iter()
                .find(|(column, _)| *column == row)
                .map(|(_, value)| *value)
                .unwrap_or(0.0);
            if !(diagonal.is_finite() && diagonal > 0.0) {
                return Err(
                    "buckling elastic stiffness must have a positive finite diagonal".to_string(),
                );
            }
            Ok(diagonal.sqrt().recip())
        })
        .collect()
}

fn symmetrically_scaled(matrix: &SparseMatrix, scaling: &[f64]) -> SparseMatrix {
    let mut scaled = SparseMatrix::new(matrix.size());
    for row in 0..matrix.size() {
        for &(column, value) in matrix.row_entries(row) {
            add_at(
                &mut scaled,
                row,
                column,
                value * scaling[row] * scaling[column],
            );
        }
    }
    scaled
}

fn solve_banded_refined(
    factor: &SymmetricBandCholesky,
    matrix: &CompressedSparseMatrix,
    rhs: &[f64],
) -> Result<Vec<f64>, String> {
    let mut solution = factor.solve(rhs)?;
    let target = 1.0e-11 * l2_norm(rhs).max(1.0);
    for _ in 0..4 {
        let product = multiply(matrix, &solution);
        let residual = rhs
            .iter()
            .zip(product)
            .map(|(right, left)| right - left)
            .collect::<Vec<_>>();
        if l2_norm(&residual) <= target {
            break;
        }
        let correction = factor.solve(&residual)?;
        add_scaled(&mut solution, &correction, 1.0);
    }
    Ok(solution)
}

fn initial_subspace(size: usize, count: usize) -> Vec<Vec<f64>> {
    (0..count)
        .map(|mode| {
            (0..size)
                .map(|index| {
                    let phase = std::f64::consts::PI * (index + 1) as f64 / (size + 1) as f64;
                    ((mode + 1) as f64 * phase).sin()
                        + 0.137 * ((mode + count + 1) as f64 * phase).cos()
                })
                .collect()
        })
        .collect()
}

fn k_orthonormalize(
    candidates: Vec<Vec<f64>>,
    stiffness: &CompressedSparseMatrix,
) -> Vec<Vec<f64>> {
    let mut basis: Vec<Vec<f64>> = Vec::with_capacity(candidates.len());
    let mut products: Vec<Vec<f64>> = Vec::with_capacity(candidates.len());
    for mut candidate in candidates {
        if !normalize_euclidean(&mut candidate) {
            continue;
        }
        for _ in 0..2 {
            for (vector, product) in basis.iter().zip(&products) {
                let projection = dot(product, &candidate);
                add_scaled(&mut candidate, vector, -projection);
            }
        }
        if !normalize_euclidean(&mut candidate) {
            continue;
        }
        let mut product = multiply(stiffness, &candidate);
        let norm_squared = dot(&candidate, &product);
        if !(norm_squared.is_finite() && norm_squared > f64::MIN_POSITIVE) {
            continue;
        }
        let inverse_norm = norm_squared.sqrt().recip();
        candidate
            .iter_mut()
            .for_each(|value| *value *= inverse_norm);
        product.iter_mut().for_each(|value| *value *= inverse_norm);
        basis.push(candidate);
        products.push(product);
    }
    basis
}

fn projected_geometric(basis: &[Vec<f64>], products: &[Vec<f64>]) -> Vec<Vec<f64>> {
    (0..basis.len())
        .map(|row| {
            (0..basis.len())
                .map(|column| dot(&basis[row], &products[column]))
                .collect()
        })
        .collect()
}

fn combine(basis: &[Vec<f64>], coefficients: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; basis[0].len()];
    for (vector, coefficient) in basis.iter().zip(coefficients) {
        add_scaled(&mut result, vector, *coefficient);
    }
    result
}

fn multiply(matrix: &CompressedSparseMatrix, vector: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; vector.len()];
    matrix.multiply_vector_into(vector, &mut result);
    result
}

fn add_scaled(target: &mut [f64], source: &[f64], scale: f64) {
    target
        .iter_mut()
        .zip(source)
        .for_each(|(value, source)| *value += scale * source);
}

fn residual_norm(elastic: &[f64], geometric: &[f64], eigenvalue: f64) -> f64 {
    elastic
        .iter()
        .zip(geometric)
        .map(|(left, right)| left - eigenvalue * right)
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn l2_norm(values: &[f64]) -> f64 {
    dot(values, values).sqrt()
}

fn normalize_euclidean(vector: &mut [f64]) -> bool {
    let norm = l2_norm(vector);
    if !(norm.is_finite() && norm > 1.0e-14) {
        return false;
    }
    vector.iter_mut().for_each(|value| *value /= norm);
    true
}

#[cfg(test)]
mod tests {
    use crate::linear_algebra::{SparseMatrix, add_at};

    use super::sparse_generalized_eigenpairs;

    #[test]
    fn finds_lowest_modes_with_semidefinite_geometric_stiffness() {
        let mut stiffness = SparseMatrix::new(4);
        let mut geometric = SparseMatrix::new(4);
        for (index, value) in [2.0, 5.0, 9.0, 9.0].into_iter().enumerate() {
            add_at(&mut stiffness, index, index, value);
        }
        for (index, value) in [1.0, 0.5, 0.0, 0.25].into_iter().enumerate() {
            add_at(&mut geometric, index, index, value);
        }

        let pairs = sparse_generalized_eigenpairs(&stiffness, &geometric, 2)
            .expect("sparse generalized eigensolver should converge");
        assert_eq!(pairs.len(), 2);
        assert!((pairs[0].eigenvalue - 2.0).abs() < 1.0e-8);
        assert!((pairs[1].eigenvalue - 10.0).abs() < 1.0e-8);
    }

    #[test]
    fn single_mode_request_converges_across_a_clustered_sparse_spectrum() {
        let size = 600;
        let mut stiffness = SparseMatrix::new(size);
        let mut geometric = SparseMatrix::new(size);
        for index in 0..size {
            let eigenvalue = match index {
                0 => 1.0,
                1 => 1.000_001,
                2 => 20.0,
                _ => 100.0 + index as f64,
            };
            add_at(&mut stiffness, index, index, 1.0);
            add_at(&mut geometric, index, index, eigenvalue.recip());
        }

        let pairs = sparse_generalized_eigenpairs(&stiffness, &geometric, 1)
            .expect("oversampled sparse iteration should resolve clustered first modes");
        assert_eq!(pairs.len(), 1);
        assert!((pairs[0].eigenvalue - 1.0).abs() < 1.0e-8);
        assert!(pairs[0].residual_norm < 1.0e-6);
    }
}
