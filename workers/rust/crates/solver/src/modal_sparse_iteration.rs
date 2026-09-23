use crate::linear_algebra::stable_l2_norm;
use crate::modal_sparse::{InverseIterationOptions, SparseEigenpair};
use crate::solver_control::{SolverStage, checkpoint};

/// Search from two deterministic probes so a uniform higher eigenmode cannot end the
/// search by itself. Residual convergence alone is not a proof of spectral minimality.
pub(crate) fn inverse_power_iteration(
    size: usize,
    options: InverseIterationOptions,
    apply_operator: impl Fn(&[f64]) -> Result<Vec<f64>, String>,
    solve_inverse: impl Fn(&[f64]) -> Result<Vec<f64>, String>,
) -> Result<SparseEigenpair, String> {
    if size == 0 {
        return Err("sparse modal operator must have at least one degree of freedom".into());
    }
    if options.max_iterations == 0 || !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err("sparse modal inverse iteration options are invalid".into());
    }
    let uniform = vec![1.0 / (size as f64).sqrt(); size];
    let first = iterate(uniform, options, &apply_operator, &solve_inverse)?;
    let mut varied: Vec<_> = (0..size)
        .map(|index| {
            let mut bits = (index as u64).wrapping_add(0x9e3779b97f4a7c15);
            bits = (bits ^ (bits >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            bits = (bits ^ (bits >> 27)).wrapping_mul(0x94d049bb133111eb);
            bits ^= bits >> 31;
            (bits >> 11) as f64 * (2.0 / ((1_u64 << 53) as f64)) - 1.0
        })
        .collect();
    let norm = stable_l2_norm(varied.iter().copied());
    varied.iter_mut().for_each(|value| *value /= norm);
    // Both probes must converge; an unresolved probe could contain a lower mode.
    let second = iterate(varied, options, &apply_operator, &solve_inverse)?;
    Ok(if first.eigenvalue <= second.eigenvalue {
        first
    } else {
        second
    })
}

fn iterate(
    mut vector: Vec<f64>,
    options: InverseIterationOptions,
    apply_operator: &impl Fn(&[f64]) -> Result<Vec<f64>, String>,
    solve_inverse: &impl Fn(&[f64]) -> Result<Vec<f64>, String>,
) -> Result<SparseEigenpair, String> {
    let size = vector.len();
    let mut last_eigenvalue = f64::NAN;
    let mut last_residual_norm = f64::NAN;
    for iteration in 0..=options.max_iterations {
        checkpoint(SolverStage::ModalIteration, iteration)?;
        let applied = apply_operator(&vector)?;
        if applied.len() != size {
            return Err("sparse modal operator returned an invalid vector size".into());
        }
        let eigenvalue: f64 = vector.iter().zip(&applied).map(|(a, b)| a * b).sum();
        let residual_norm = stable_l2_norm(
            applied
                .iter()
                .zip(&vector)
                .map(|(value, component)| value - eigenvalue * component),
        );
        let residual_scale = stable_l2_norm(applied.iter().copied()).max(eigenvalue.abs());
        if !eigenvalue.is_finite() || eigenvalue <= 0.0 || !residual_norm.is_finite() {
            return Err(
                "sparse modal iteration produced a nonpositive or non-finite eigenpair".into(),
            );
        }
        if !(residual_scale.is_finite() && residual_scale > 0.0) {
            return Err("sparse modal operator has zero or non-finite scale".into());
        }
        last_eigenvalue = eigenvalue;
        last_residual_norm = residual_norm;
        if residual_norm <= options.tolerance * residual_scale {
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
            return Err("sparse modal inverse solve returned an invalid vector size".into());
        }
        let norm = stable_l2_norm(next.iter().copied());
        if !norm.is_finite() || norm <= 0.0 {
            return Err("sparse modal inverse solve returned a zero or non-finite vector".into());
        }
        vector = next.into_iter().map(|value| value / norm).collect();
    }
    Err(format!(
        "sparse modal inverse iteration did not converge within {} iterations (eigenvalue={last_eigenvalue:.6e}, residual={last_residual_norm:.6e})",
        options.max_iterations,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver_control::{SolverControl, with_solver_observer};

    fn symmetric_problem() -> Result<SparseEigenpair, String> {
        // A = I + uu^T, u = (1,1,1): the uniform seed is the highest mode (4), not 1.
        inverse_power_iteration(
            3,
            InverseIterationOptions::default(),
            |v| Ok(v.iter().map(|x| x + v.iter().sum::<f64>()).collect()),
            |v| Ok(v.iter().map(|x| x - v.iter().sum::<f64>() / 4.0).collect()),
        )
    }

    #[test]
    fn independent_probe_escapes_a_uniform_higher_mode() {
        let pair = symmetric_problem().unwrap();
        assert!((pair.eigenvalue - 1.0).abs() < 1.0e-10);
        assert!(pair.residual_norm < 1.0e-6);
    }

    #[test]
    fn cancellation_stops_iteration_and_leaves_replay_clean() {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalIteration {
                    cancel.request_cancel();
                }
            },
            || {
                let result = symmetric_problem();
                assert!(
                    result.is_err(),
                    "iteration must observe cancellation itself"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert!((symmetric_problem().unwrap().eigenvalue - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn unconverged_second_probe_cannot_return_the_uniform_higher_mode() {
        let error = inverse_power_iteration(
            3,
            InverseIterationOptions {
                max_iterations: 1,
                tolerance: 1.0e-12,
            },
            |v| Ok(v.iter().map(|x| x + v.iter().sum::<f64>()).collect()),
            |v| Ok(v.to_vec()),
        )
        .unwrap_err();
        assert!(error.contains("did not converge"), "{error}");
    }
}
