use std::time::Instant;

use crate::linear_algebra::{
    CompressedSparseMatrix, SparseMatrix, sparse_residual_norm, sparse_to_dense,
};
use crate::linear_dense::solve_linear_system;
use crate::linear_solver_profile::{SpdSolveOptions, SpdSolveProfile, SpdSolveStage};

pub(crate) fn solve_spd_compressed(
    matrix: &CompressedSparseMatrix,
    rhs: &[f64],
    fallback_source: &SparseMatrix,
    options: &SpdSolveOptions,
) -> Result<SpdSolveProfile, String> {
    let size = rhs.len();
    let solve_started = Instant::now();
    let preconditioner = options.preconditioner;
    let mut timings = SpdTimings::default();
    let started = Instant::now();
    let rhs_scale = rhs.iter().map(|value| value.abs()).fold(0.0, f64::max);
    timings.dot_ms += elapsed_ms(started);
    if rhs_scale == 0.0 {
        return Ok(SpdSolveProfile {
            solution: vec![0.0; size],
            iterations: 0,
            matrix_non_zero_count: matrix.non_zero_count(),
            residual_norm: 0.0,
            stages: timings.into_stages(),
        });
    }

    let mut x = vec![0.0; size];
    let mut r = rhs
        .iter()
        .map(|value| value / rhs_scale)
        .collect::<Vec<_>>();
    let mut z = vec![0.0; size];
    let mut p = vec![0.0; size];
    let mut ap = vec![0.0; size];
    let mut ax = vec![0.0; size];
    let mut preconditioner_workspace = vec![0.0; size];
    let started = Instant::now();
    matrix.apply_preconditioner_into(preconditioner, &r, &mut z, &mut preconditioner_workspace);
    timings.preconditioner_ms += elapsed_ms(started);
    p.clone_from(&z);

    let tolerance = (1.0e-9 * l2_norm(&r)).max(f64::MIN_POSITIVE);
    let started = Instant::now();
    let mut rz_old = dot(&r, &z);
    timings.dot_ms += elapsed_ms(started);
    if !rz_old.is_finite() || rz_old <= 0.0 {
        let started = Instant::now();
        let residual_norm = l2_norm(&r);
        timings.dot_ms += elapsed_ms(started);
        if residual_norm <= tolerance {
            return Ok(SpdSolveProfile {
                solution: x,
                iterations: 0,
                matrix_non_zero_count: matrix.non_zero_count(),
                residual_norm,
                stages: timings.into_stages(),
            });
        }
        return solve_spd_fallback(
            fallback_source,
            rhs,
            "preconditioner is not positive definite",
        );
    }

    let max_iter = (size.saturating_mul(8)).clamp(256, 40_000);

    for iteration in 0..max_iter {
        let started = Instant::now();
        matrix.multiply_vector_into(&p, &mut ap);
        timings.matvec_ms += elapsed_ms(started);
        let started = Instant::now();
        let mut denom = dot(&p, &ap);
        timings.dot_ms += elapsed_ms(started);
        if !denom.is_finite() || denom <= 0.0 {
            p.clone_from(&z);
            let started = Instant::now();
            matrix.multiply_vector_into(&p, &mut ap);
            timings.matvec_ms += elapsed_ms(started);
            let started = Instant::now();
            denom = dot(&p, &ap);
            timings.dot_ms += elapsed_ms(started);
            if !denom.is_finite() || denom <= 0.0 {
                return solve_spd_fallback(fallback_source, rhs, "system is singular");
            }
        }

        let alpha = rz_old / denom;
        if !alpha.is_finite() || alpha <= 0.0 {
            return solve_spd_fallback(fallback_source, rhs, "iterative solver diverged");
        }
        let mut residual_squared = 0.0;
        let started = Instant::now();
        for index in 0..size {
            x[index] += alpha * p[index];
            r[index] -= alpha * ap[index];
            residual_squared += r[index] * r[index];
        }
        timings.vector_update_ms += elapsed_ms(started);
        if !residual_squared.is_finite() {
            return solve_spd_fallback(fallback_source, rhs, "iterative solver diverged");
        }

        let residual_recomputed = iteration % 64 == 63;
        let mut restart_direction = false;
        if residual_recomputed {
            let recursive_norm = residual_squared.sqrt();
            let started = Instant::now();
            residual_squared = recompute_residual(matrix, rhs, rhs_scale, &x, &mut r, &mut ax);
            timings.residual_recompute_ms += elapsed_ms(started);
            restart_direction =
                residual_drift_requires_restart(recursive_norm, residual_squared.sqrt(), tolerance);
        }

        let residual_norm = residual_squared.sqrt();
        if options
            .progress_interval
            .is_some_and(|interval| interval > 0 && (iteration + 1) % interval == 0)
        {
            eprintln!(
                "solver progress: iteration={} residual={:.6e} tolerance={:.6e} elapsed_ms={:.3}",
                iteration + 1,
                residual_norm,
                tolerance,
                elapsed_ms(solve_started)
            );
        }
        if residual_norm <= tolerance {
            let verified_squared = if residual_recomputed {
                residual_squared
            } else {
                let recursive_norm = residual_norm;
                let started = Instant::now();
                let exact = recompute_residual(matrix, rhs, rhs_scale, &x, &mut r, &mut ax);
                timings.residual_recompute_ms += elapsed_ms(started);
                restart_direction =
                    residual_drift_requires_restart(recursive_norm, exact.sqrt(), tolerance);
                exact
            };
            let verified_norm = verified_squared.sqrt();
            if !verified_norm.is_finite() {
                return solve_spd_fallback(fallback_source, rhs, "iterative solver diverged");
            }
            if verified_norm <= tolerance {
                return Ok(SpdSolveProfile {
                    solution: rescale_solution(x, rhs_scale),
                    iterations: iteration + 1,
                    matrix_non_zero_count: matrix.non_zero_count(),
                    residual_norm: verified_norm * rhs_scale,
                    stages: timings.into_stages(),
                });
            }
        }

        let started = Instant::now();
        matrix.apply_preconditioner_into(preconditioner, &r, &mut z, &mut preconditioner_workspace);
        timings.preconditioner_ms += elapsed_ms(started);

        let started = Instant::now();
        let rz_new = dot(&r, &z);
        timings.dot_ms += elapsed_ms(started);
        if !rz_new.is_finite() || rz_new <= 0.0 {
            return solve_spd_fallback(fallback_source, rhs, "iterative solver diverged");
        }
        let started = Instant::now();
        if restart_direction {
            p.clone_from(&z);
        } else {
            let beta = rz_new / rz_old;
            if !beta.is_finite() || beta < 0.0 {
                return solve_spd_fallback(fallback_source, rhs, "iterative solver diverged");
            }
            for index in 0..size {
                p[index] = z[index] + beta * p[index];
            }
        }
        timings.direction_update_ms += elapsed_ms(started);
        rz_old = rz_new;
    }

    solve_spd_fallback(fallback_source, rhs, "iterative solver did not converge")
}

fn recompute_residual(
    matrix: &CompressedSparseMatrix,
    rhs: &[f64],
    rhs_scale: f64,
    x: &[f64],
    residual: &mut [f64],
    ax: &mut [f64],
) -> f64 {
    matrix.multiply_vector_into(x, ax);
    let mut residual_squared = 0.0;
    for index in 0..rhs.len() {
        residual[index] = rhs[index] / rhs_scale - ax[index];
        residual_squared += residual[index] * residual[index];
    }
    residual_squared
}

fn rescale_solution(mut solution: Vec<f64>, rhs_scale: f64) -> Vec<f64> {
    for value in &mut solution {
        *value *= rhs_scale;
    }
    solution
}

fn residual_drift_requires_restart(recursive: f64, exact: f64, tolerance: f64) -> bool {
    !recursive.is_finite()
        || !exact.is_finite()
        || (recursive - exact).abs() > 0.25 * recursive.max(exact).max(tolerance)
}

fn dot(lhs: &[f64], rhs: &[f64]) -> f64 {
    debug_assert_eq!(lhs.len(), rhs.len());
    let mut sum = 0.0;
    for index in 0..lhs.len() {
        sum += lhs[index] * rhs[index];
    }
    sum
}

fn l2_norm(values: &[f64]) -> f64 {
    let mut scale = 0.0;
    let mut sum_squares = 1.0;
    for value in values
        .iter()
        .map(|value| value.abs())
        .filter(|value| *value > 0.0)
    {
        if scale < value {
            sum_squares = 1.0 + sum_squares * (scale / value).powi(2);
            scale = value;
        } else {
            sum_squares += (value / scale).powi(2);
        }
    }
    if scale == 0.0 {
        0.0
    } else {
        scale * sum_squares.sqrt()
    }
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
        solve_linear_system(sparse_to_dense(matrix), rhs.to_vec()).map(|solution| {
            let residual_norm = sparse_residual_norm(matrix, rhs, &solution);
            SpdSolveProfile {
                solution,
                iterations: 0,
                matrix_non_zero_count: matrix.non_zero_count(),
                residual_norm,
                stages: Vec::new(),
            }
        })
    } else {
        Err(reason.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::linear_algebra::{SparseMatrix, add_at};
    use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions};

    use super::solve_spd_compressed;

    #[test]
    fn solves_high_stiffness_scale_without_treating_preconditioned_residual_as_zero() {
        let mut matrix = SparseMatrix::new(2);
        add_at(&mut matrix, 0, 0, 2.0e24);
        add_at(&mut matrix, 0, 1, -1.0e24);
        add_at(&mut matrix, 1, 0, -1.0e24);
        add_at(&mut matrix, 1, 1, 2.0e24);
        let options = SpdSolveOptions {
            preconditioner: SpdPreconditioner::IncompleteCholesky,
            progress_interval: None,
        };
        let compressed = matrix.compress(options.preconditioner);

        let profile = solve_spd_compressed(&compressed, &[1.0, 0.0], &matrix, &options)
            .expect("high-scale SPD system should solve");
        assert!(profile.iterations > 0);
        assert!((profile.solution[0] - 2.0e-24 / 3.0).abs() < 1.0e-35);
        assert!((profile.solution[1] - 1.0e-24 / 3.0).abs() < 1.0e-35);
    }
}
