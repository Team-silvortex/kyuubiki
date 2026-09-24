use super::EquilibriumMetric;
use crate::linear_algebra::{SparseMatrix, add_at};

#[test]
fn row_metric_keeps_weak_forces_and_moments_visible() {
    for large_force in [1e20, 1e100, 1e300] {
        let metric = EquilibriumMetric::new(
            &SparseMatrix::new(3),
            &[0.0; 3],
            &[large_force, 10.0, 0.5],
            1.0,
            &[0, 1, 2],
        )
        .unwrap();
        assert_eq!(metric.norm(&[0.0, 1.0, 0.0]), 0.1);
        assert_eq!(metric.norm(&[0.0, 0.0, 0.25]), 0.25);
    }
}

#[test]
fn row_metric_keeps_an_unloaded_equations_local_cancellation_scale() {
    let mut tangent = SparseMatrix::new(2);
    add_at(&mut tangent, 0, 0, 1e160);
    add_at(&mut tangent, 0, 1, -1e160);
    for load_factor in [0.0, -1.0, 1.0] {
        let metric =
            EquilibriumMetric::new(&tangent, &[1.0, 1.0], &[0.0, 1e160], load_factor, &[0, 1])
                .unwrap();
        assert!(metric.norm(&[1e145, 0.0]) < 1e-10);
        assert!(metric.norm(&[1e156, 0.0]) > 1e-5);
    }
}

#[test]
fn row_metric_never_relaxes_the_existing_global_residual_requirement() {
    let mut tangent = SparseMatrix::new(1);
    add_at(&mut tangent, 0, 0, 1e300);
    let metric = EquilibriumMetric::new(&tangent, &[1.0], &[10.0], 1.0, &[0]).unwrap();
    assert_eq!(metric.norm(&[1.0]), 0.1);
}

#[test]
fn row_metric_accumulates_large_absolute_terms_without_scale_overflow() {
    let mut tangent = SparseMatrix::new(2);
    add_at(&mut tangent, 0, 0, 1e308);
    add_at(&mut tangent, 0, 1, -1e308);
    let metric =
        EquilibriumMetric::new(&tangent, &[1.0, 1.0], &[0.0, 1e308], 2.0, &[0, 1]).unwrap();
    let norm = metric.norm(&[1e308, 0.0]);
    assert!(norm.is_finite() && (norm - 0.5).abs() < 1e-15);
}

#[test]
fn row_metric_rejects_invalid_shapes_and_nonfinite_tangent_products() {
    let mut tangent = SparseMatrix::new(2);
    for (displacement, reference, free) in [
        (&[0.0][..], &[1.0, 2.0][..], &[0][..]),
        (&[0.0, 0.0][..], &[1.0][..], &[0][..]),
        (&[0.0, 0.0][..], &[1.0, 2.0][..], &[2][..]),
        (&[f64::NAN, 0.0][..], &[1.0, 2.0][..], &[1][..]),
    ] {
        assert!(EquilibriumMetric::new(&tangent, displacement, reference, 1.0, free).is_err());
    }
    add_at(&mut tangent, 0, 0, 1e308);
    let error = EquilibriumMetric::new(&tangent, &[2.0, 0.0], &[1.0, 1.0], 1.0, &[0])
        .err()
        .unwrap();
    assert!(
        error.contains("DOF 0") && error.contains("non-finite"),
        "{error}"
    );
    for coefficient in [f64::NAN, f64::INFINITY] {
        let mut tangent = SparseMatrix::new(1);
        add_at(&mut tangent, 0, 0, coefficient);
        assert!(EquilibriumMetric::new(&tangent, &[0.0], &[1.0], 1.0, &[0]).is_err());
    }
}

#[test]
fn row_metric_free_map_reordering_keeps_equations_aligned() {
    let mut tangent = SparseMatrix::new(3);
    add_at(&mut tangent, 0, 0, 3.0);
    add_at(&mut tangent, 2, 2, 10.0);
    let first =
        EquilibriumMetric::new(&tangent, &[1.0; 3], &[2.0, 1e300, 1.0], 1.0, &[0, 2]).unwrap();
    let swapped =
        EquilibriumMetric::new(&tangent, &[1.0; 3], &[2.0, 1e300, 1.0], 1.0, &[2, 0]).unwrap();
    assert_eq!(first.norm(&[0.1, 0.2]), swapped.norm(&[0.2, 0.1]));
}

#[test]
fn row_metric_is_frozen_while_trials_are_compared() {
    let mut tangent = SparseMatrix::new(2);
    add_at(&mut tangent, 0, 0, 10.0);
    let metric = EquilibriumMetric::new(&tangent, &[1.0, 0.0], &[1.0, 1e20], 1.0, &[0, 1]).unwrap();
    let initial = metric.norm(&[1.0, 0.0]);
    let trial_scaled =
        EquilibriumMetric::new(&tangent, &[1e10, 0.0], &[1.0, 1e20], 1.0, &[0, 1]).unwrap();
    assert!(trial_scaled.norm(&[2.0, 0.0]) < initial);
    assert!(metric.norm(&[2.0, 0.0]) > initial);
    assert!(metric.norm(&[0.5, 0.0]) < initial);
}

#[test]
fn predictor_metric_scales_initially_unloaded_rows_without_extra_tangent_solves() {
    let mut tangent = SparseMatrix::new(2);
    add_at(&mut tangent, 0, 0, 10.0);
    let metric = EquilibriumMetric::for_increment(
        &tangent,
        &[0.0, 0.0],
        &[0.0, 1e20],
        1.0,
        &[0, 1],
        &[2.0, 0.0],
    )
    .unwrap();
    assert!((metric.norm(&[1.0, 0.0]) - 1.0 / 21.0).abs() < 1e-15);
    assert!(metric.norm(&[2.0, 0.0]) > metric.norm(&[1.0, 0.0]));
    for (free, increment) in [
        (&[0][..], &[][..]),
        (&[2][..], &[0.0][..]),
        (&[0][..], &[f64::NAN][..]),
    ] {
        assert!(
            EquilibriumMetric::for_increment(&tangent, &[0.0; 2], &[1.0; 2], 1.0, free, increment)
                .is_err()
        );
    }
}

#[test]
fn row_metric_respects_cooperative_cancellation() {
    use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ResidualValidate {
                cancel.request_cancel();
            }
        },
        || EquilibriumMetric::new(&SparseMatrix::new(1), &[0.0], &[1.0], 1.0, &[0]),
    )
    .err()
    .unwrap();
    assert!(error.contains("cancelled"), "{error}");
    assert!(EquilibriumMetric::new(&SparseMatrix::new(1), &[0.0], &[1.0], 1.0, &[0]).is_ok());
}
