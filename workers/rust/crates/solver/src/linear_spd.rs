use std::time::Instant;

use crate::linear_algebra::{CompressedSparseMatrix, SparseMatrix, sparse_to_dense};
use crate::linear_dense::solve_linear_system;
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveProfile, SpdSolveStage};

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
    let mut timings = SpdTimings::default();
    let started = Instant::now();
    matrix.apply_preconditioner_into(preconditioner, &r, &mut z, &mut preconditioner_workspace);
    timings.preconditioner_ms += elapsed_ms(started);
    p.clone_from(&z);

    let started = Instant::now();
    let rhs_norm = dot(rhs, rhs).sqrt().max(1.0);
    timings.dot_ms += elapsed_ms(started);
    let tolerance = 1.0e-9 * rhs_norm;
    let started = Instant::now();
    let mut rz_old = dot(&r, &z);
    timings.dot_ms += elapsed_ms(started);
    if !rz_old.is_finite() || rz_old.abs() < 1.0e-20 {
        let started = Instant::now();
        let residual_norm = dot(&r, &r).sqrt();
        timings.dot_ms += elapsed_ms(started);
        return Ok(SpdSolveProfile {
            solution: x,
            iterations: 0,
            matrix_non_zero_count: matrix.non_zero_count(),
            residual_norm,
            stages: timings.into_stages(),
        });
    }

    let max_iter = (size.saturating_mul(8)).clamp(256, 40_000);

    for iteration in 0..max_iter {
        let started = Instant::now();
        matrix.multiply_vector_into(&p, &mut ap);
        timings.matvec_ms += elapsed_ms(started);
        let started = Instant::now();
        let mut denom = dot(&p, &ap);
        timings.dot_ms += elapsed_ms(started);
        if !denom.is_finite() || denom.abs() < 1.0e-20 {
            p.clone_from(&z);
            let started = Instant::now();
            matrix.multiply_vector_into(&p, &mut ap);
            timings.matvec_ms += elapsed_ms(started);
            let started = Instant::now();
            denom = dot(&p, &ap);
            timings.dot_ms += elapsed_ms(started);
            if !denom.is_finite() || denom.abs() < 1.0e-20 {
                return solve_spd_fallback(fallback_source, rhs, "system is singular");
            }
        }

        let alpha = rz_old / denom;
        let mut residual_squared = 0.0;
        let started = Instant::now();
        for index in 0..size {
            x[index] += alpha * p[index];
            r[index] -= alpha * ap[index];
            residual_squared += r[index] * r[index];
        }
        timings.vector_update_ms += elapsed_ms(started);

        if iteration % 64 == 63 {
            let started = Instant::now();
            residual_squared = recompute_residual(matrix, rhs, &x, &mut r, &mut ax);
            timings.residual_recompute_ms += elapsed_ms(started);
        }

        let residual_norm = residual_squared.sqrt();
        if residual_norm <= tolerance {
            return Ok(SpdSolveProfile {
                solution: x,
                iterations: iteration + 1,
                matrix_non_zero_count: matrix.non_zero_count(),
                residual_norm,
                stages: timings.into_stages(),
            });
        }

        let started = Instant::now();
        matrix.apply_preconditioner_into(preconditioner, &r, &mut z, &mut preconditioner_workspace);
        timings.preconditioner_ms += elapsed_ms(started);

        let started = Instant::now();
        let rz_new = dot(&r, &z);
        timings.dot_ms += elapsed_ms(started);
        if !rz_new.is_finite() {
            return solve_spd_fallback(fallback_source, rhs, "iterative solver diverged");
        }
        let beta = if rz_old.abs() < 1.0e-20 {
            0.0
        } else {
            rz_new / rz_old
        };
        let started = Instant::now();
        for index in 0..size {
            p[index] = z[index] + beta * p[index];
        }
        timings.direction_update_ms += elapsed_ms(started);
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
    debug_assert_eq!(lhs.len(), rhs.len());
    let mut sum = 0.0;
    for index in 0..lhs.len() {
        sum += lhs[index] * rhs[index];
    }
    sum
}

#[derive(Debug, Default)]
struct SpdTimings {
    direction_update_ms: f64,
    dot_ms: f64,
    matvec_ms: f64,
    preconditioner_ms: f64,
    residual_recompute_ms: f64,
    vector_update_ms: f64,
}

impl SpdTimings {
    fn into_stages(self) -> Vec<SpdSolveStage> {
        [
            ("solve_spd_matvec", self.matvec_ms),
            ("solve_spd_preconditioner", self.preconditioner_ms),
            ("solve_spd_vector_update", self.vector_update_ms),
            ("solve_spd_direction_update", self.direction_update_ms),
            ("solve_spd_dot", self.dot_ms),
            ("solve_spd_residual_recompute", self.residual_recompute_ms),
        ]
        .into_iter()
        .filter(|(_, elapsed_ms)| *elapsed_ms > 0.0)
        .map(|(label, elapsed_ms)| SpdSolveStage { label, elapsed_ms })
        .collect()
    }
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
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
            matrix_non_zero_count: matrix.non_zero_count(),
            residual_norm: 0.0,
            stages: Vec::new(),
        })
    } else {
        Err(reason.to_string())
    }
}
