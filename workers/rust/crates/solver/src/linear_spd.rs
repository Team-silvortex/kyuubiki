use crate::linear_algebra::{CompressedSparseMatrix, SparseMatrix, sparse_to_dense};
use crate::linear_dense::solve_linear_system;
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveProfile};

pub(crate) fn solve_spd_compressed(
    matrix: &CompressedSparseMatrix,
    rhs: &[f64],
    fallback_source: &SparseMatrix,
    preconditioner: SpdPreconditioner,
) -> Result<SpdSolveProfile, String> {
    let size = rhs.len();

    let mut x = vec![0.0; size];
    let mut r = rhs.to_vec();
    let mut z = vec![0.0; size];
    let mut p = vec![0.0; size];
    let mut ap = vec![0.0; size];
    let mut ax = vec![0.0; size];
    let mut preconditioner_workspace = vec![0.0; size];
    matrix.apply_preconditioner_into(preconditioner, &r, &mut z, &mut preconditioner_workspace);
    p.clone_from(&z);

    let rhs_norm = dot(rhs, rhs).sqrt().max(1.0);
    let tolerance = 1.0e-9 * rhs_norm;
    let mut rz_old = dot(&r, &z);
    if !rz_old.is_finite() || rz_old.abs() < 1.0e-20 {
        return Ok(SpdSolveProfile {
            solution: x,
            iterations: 0,
            residual_norm: dot(&r, &r).sqrt(),
        });
    }

    let max_iter = (size.saturating_mul(8)).clamp(256, 40_000);

    for iteration in 0..max_iter {
        matrix.multiply_vector_into(&p, &mut ap);
        let mut denom = dot(&p, &ap);
        if !denom.is_finite() || denom.abs() < 1.0e-20 {
            p.clone_from(&z);
            matrix.multiply_vector_into(&p, &mut ap);
            denom = dot(&p, &ap);
            if !denom.is_finite() || denom.abs() < 1.0e-20 {
                return solve_spd_fallback(fallback_source, rhs, "system is singular");
            }
        }

        let alpha = rz_old / denom;
        let mut residual_squared = 0.0;
        for index in 0..size {
            x[index] += alpha * p[index];
            r[index] -= alpha * ap[index];
            residual_squared += r[index] * r[index];
        }

        if iteration % 64 == 63 {
            residual_squared = recompute_residual(matrix, rhs, &x, &mut r, &mut ax);
        }

        let residual_norm = residual_squared.sqrt();
        if residual_norm <= tolerance {
            return Ok(SpdSolveProfile {
                solution: x,
                iterations: iteration + 1,
                residual_norm,
            });
        }

        matrix.apply_preconditioner_into(preconditioner, &r, &mut z, &mut preconditioner_workspace);

        let rz_new = dot(&r, &z);
        if !rz_new.is_finite() {
            return solve_spd_fallback(fallback_source, rhs, "iterative solver diverged");
        }
        let beta = if rz_old.abs() < 1.0e-20 {
            0.0
        } else {
            rz_new / rz_old
        };
        for index in 0..size {
            p[index] = z[index] + beta * p[index];
        }
        rz_old = rz_new;
    }

    solve_spd_fallback(fallback_source, rhs, "iterative solver did not converge")
}

fn recompute_residual(
    matrix: &CompressedSparseMatrix,
    rhs: &[f64],
    x: &[f64],
    residual: &mut [f64],
    ax: &mut [f64],
) -> f64 {
    matrix.multiply_vector_into(x, ax);
    let mut residual_squared = 0.0;
    for index in 0..rhs.len() {
        residual[index] = rhs[index] - ax[index];
        residual_squared += residual[index] * residual[index];
    }
    residual_squared
}

fn dot(lhs: &[f64], rhs: &[f64]) -> f64 {
    lhs.iter().zip(rhs.iter()).map(|(a, b)| a * b).sum()
}

fn solve_spd_fallback(
    matrix: &SparseMatrix,
    rhs: &[f64],
    reason: &str,
) -> Result<SpdSolveProfile, String> {
    if rhs.len() <= 1024 {
        solve_linear_system(sparse_to_dense(matrix), rhs.to_vec()).map(|solution| SpdSolveProfile {
            solution,
            iterations: 0,
            residual_norm: 0.0,
        })
    } else {
        Err(reason.to_string())
    }
}
