use super::EquilibriumMetric;
use crate::linear_algebra::SparseMatrix;

fn normalized_residual(
    residual: &[f64],
    external: &[f64],
    load_factor: f64,
    free: &[usize],
) -> f64 {
    EquilibriumMetric::new(
        &SparseMatrix::new(external.len()),
        &vec![0.0; external.len()],
        external,
        load_factor,
        free,
    )
    .map_or(f64::INFINITY, |metric| metric.norm(residual))
}

#[test]
fn nonlinear_equilibrium_normalization_ignores_constrained_reference_loads() {
    for support_load in [0.0, 1e30, -1e300] {
        for load_factor in [-3.0, 0.0, 0.25, 1.0, 3.0] {
            let expected = normalized_residual(&[2.0, 3.0], &[20.0, -40.0], load_factor, &[0, 1]);
            let actual = normalized_residual(
                &[2.0, 3.0],
                &[support_load, 20.0, support_load, -40.0],
                load_factor,
                &[1, 3],
            );
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn nonlinear_equilibrium_checks_the_actual_free_map_not_a_prefix() {
    let external = [200.0, 1e100, -100.0, -1e100];
    assert_eq!(
        normalized_residual(&[1.0, 2.0], &external, 2.0, &[2, 0]),
        0.005
    );
    assert_eq!(
        normalized_residual(&[2.0, 1.0], &external, 2.0, &[0, 2]),
        0.005
    );
}

#[test]
fn nonlinear_equilibrium_rejects_invalid_reduced_dimensions_and_indices() {
    for (residual, free) in [
        (&[0.0][..], &[][..]),
        (&[][..], &[0][..]),
        (&[0.0][..], &[usize::MAX][..]),
    ] {
        assert_eq!(
            normalized_residual(residual, &[1.0], 1.0, free),
            f64::INFINITY
        );
    }
}

#[test]
fn nonlinear_equilibrium_never_accepts_nonfinite_input_including_supports() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            normalized_residual(&[value], &[1.0], 1.0, &[0]),
            f64::INFINITY
        );
        assert_eq!(
            normalized_residual(&[0.0], &[value, 1.0], 1.0, &[1]),
            f64::INFINITY
        );
        assert_eq!(
            normalized_residual(&[0.0], &[1.0], value, &[0]),
            f64::INFINITY
        );
    }
}

#[test]
fn nonlinear_equilibrium_keeps_finite_normalization_without_multiplying_large_scales() {
    let result = normalized_residual(&[1e308], &[f64::MAX, 1e308], 2.0, &[1]);
    assert_eq!(result, 0.5);
    let result = normalized_residual(&[1e-300], &[1e300, 0.0], 1.0, &[1]);
    assert_eq!(result, 1e-300);
}

#[test]
fn nonlinear_equilibrium_all_constrained_state_has_no_free_residual() {
    assert_eq!(normalized_residual(&[], &[1e300, -1e300], 1.0, &[]), 0.0);
    assert!(normalized_residual(&[1.0], &[1e300, 0.0], 1.0, &[1]) > 1e-7);
}
