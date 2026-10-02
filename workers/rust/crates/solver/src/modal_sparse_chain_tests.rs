use super::{SparseMassNormalizedOperator, approximately_equal};
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn chain(stiffness: f64, mass: f64, last_diagonal: f64) -> SparseMassNormalizedOperator {
    let mut matrix = SparseMatrix::new(2);
    for (i, j, value) in [
        (0, 0, 2.0 * stiffness),
        (0, 1, -stiffness),
        (1, 0, -stiffness),
        (1, 1, last_diagonal),
    ] {
        add_at(&mut matrix, i, j, value);
    }
    SparseMassNormalizedOperator::new(&matrix, &[mass, mass * 0.5]).unwrap()
}

#[test]
fn uniform_chain_keeps_a_finite_frequency_when_four_times_stiffness_overflows() {
    let operator = chain(5e307, 1.0, 5e307);
    let pair = operator
        .smallest_tridiagonal_eigenpair(1e-9)
        .unwrap()
        .unwrap();
    let expected = 5e307 * (2.0 - 2.0_f64.sqrt());
    assert!((pair.eigenvalue / expected - 1.0).abs() < 1e-12);
    assert!(pair.vector.iter().all(|x| x.is_finite()));
}

#[test]
fn tiny_nonuniform_chain_cannot_be_misclassified_as_uniform() {
    let operator = chain(1e-320, 1e-320, 2e-320);
    assert!(operator.uniform_axial_chain_eigenpair(1e-9).is_none());
    let pair = operator
        .smallest_tridiagonal_eigenpair(1e-9)
        .unwrap()
        .unwrap();
    let expected = 3.0 - 3.0_f64.sqrt();
    assert!((pair.eigenvalue / expected - 1.0).abs() < 1e-9);
}

#[test]
fn subnormal_nonuniformity_below_the_residual_floor_still_requires_general_solve() {
    let scale = 1e-318;
    let last = scale + 2.0 * f64::from_bits(1);
    let relative_change = last / scale - 1.0;
    let uniform_value = 2.0 - 2.0_f64.sqrt();
    let expected = 2.0 + relative_change - (2.0 + relative_change.powi(2)).sqrt();
    // The old shortcut could pass its residual floor while losing frequency accuracy.
    let shortcut_residual = 2.0_f64.sqrt() * relative_change;
    assert!(shortcut_residual < 2e-4 * uniform_value);
    assert!((uniform_value / expected - 1.0).abs() > 1e-6);
    let operator = chain(scale, scale, last);
    assert!(operator.uniform_axial_chain_eigenpair(1e-9).is_none());
    let actual = operator
        .smallest_tridiagonal_eigenpair(1e-9)
        .unwrap()
        .unwrap();
    assert!((actual.eigenvalue / expected - 1.0).abs() < 1e-12);
}

#[test]
fn uniform_admission_comparison_has_no_absolute_scale_floor() {
    let tiny = f64::from_bits(1);
    for scale in [tiny, 1e-320, f64::MIN_POSITIVE, 1e-200, 1.0, 5e307] {
        assert!(approximately_equal(scale, scale, 1e-10));
        assert!(!approximately_equal(scale, 2.0 * scale, 1e-10));
        assert!(!approximately_equal(scale, -scale, 1e-10));
        assert!(!approximately_equal(scale, 0.0, 1e-10));
    }
    assert!(approximately_equal(0.0, -0.0, 1e-10));
    assert!(approximately_equal(1.0, 1.0 + 5e-12, 1e-10));
    assert!(!approximately_equal(1.0, 1.0 + 5e-9, 1e-10));
    assert!(!approximately_equal(f64::INFINITY, f64::INFINITY, 1e-10));
    assert!(!approximately_equal(f64::NAN, 1.0, 1e-10));
}

#[test]
fn uniform_chain_closed_form_survives_common_and_independent_scales() {
    let subnormal = f64::from_bits(1 << 14);
    for (stiffness, mass) in [
        (subnormal, subnormal),
        (1e-200, 1e-200),
        (1.0, 1.0),
        (1e200, 1e200),
        (1e-200, 1.0),
        (5e307, 1.0),
    ] {
        let operator = chain(stiffness, mass, stiffness);
        let pair = operator
            .uniform_axial_chain_eigenpair(1e-9)
            .unwrap()
            .unwrap();
        let expected = (stiffness / mass) * (2.0 - 2.0_f64.sqrt());
        assert!((pair.eigenvalue / expected - 1.0).abs() < 1e-12);
        assert!(pair.residual_norm.is_finite());
        for component in pair.vector {
            assert!((component - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
        }
    }
}

#[test]
fn tiny_nonuniform_masses_use_the_general_chain_reference() {
    let scale = f64::from_bits(1 << 14);
    let mut matrix = SparseMatrix::new(2);
    for (i, j, value) in [
        (0, 0, 2.0 * scale),
        (0, 1, -scale),
        (1, 0, -scale),
        (1, 1, scale),
    ] {
        add_at(&mut matrix, i, j, value);
    }
    let operator = SparseMassNormalizedOperator::new(&matrix, &[scale; 2]).unwrap();
    assert!(operator.uniform_axial_chain_eigenpair(1e-9).is_none());
    let pair = operator
        .smallest_tridiagonal_eigenpair(1e-9)
        .unwrap()
        .unwrap();
    assert!((pair.eigenvalue / ((3.0 - 5.0_f64.sqrt()) * 0.5) - 1.0).abs() < 1e-12);
}

#[test]
fn modal_chain_rejects_invalid_tolerances_before_selecting_a_path() {
    let operator = chain(1.0, 1.0, 1.0);
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = operator
            .smallest_tridiagonal_eigenpair(tolerance)
            .unwrap()
            .unwrap_err();
        assert!(error.contains("tolerance"), "{error}");
    }
    assert!(
        operator
            .smallest_tridiagonal_eigenpair(1e-9)
            .unwrap()
            .is_ok()
    );
}

#[test]
fn unrepresentable_chain_frequency_fails_without_poisoning_replay() {
    let tiny = f64::from_bits(1);
    let error = chain(tiny, 8.0, tiny)
        .smallest_tridiagonal_eigenpair(1e-9)
        .unwrap()
        .unwrap_err();
    assert!(
        error.contains("eigenvalue") && error.contains("representable"),
        "{error}"
    );
    assert!(
        chain(1.0, 1.0, 1.0)
            .smallest_tridiagonal_eigenpair(1e-9)
            .unwrap()
            .is_ok()
    );
}

#[test]
fn closed_form_chain_propagates_residual_cancellation_and_replays() {
    let size = 128;
    let mut matrix = SparseMatrix::new(size);
    let mut mass = vec![1.0; size];
    mass[size - 1] = 0.5;
    for row in 0..size {
        add_at(
            &mut matrix,
            row,
            row,
            if row + 1 == size { 1.0 } else { 2.0 },
        );
        if row > 0 {
            add_at(&mut matrix, row, row - 1, -1.0);
            add_at(&mut matrix, row - 1, row, -1.0);
        }
    }
    let operator = SparseMassNormalizedOperator::new(&matrix, &mass).unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::SparseMatvec && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            let result = operator.smallest_tridiagonal_eigenpair(1e-9).unwrap();
            assert!(
                result.is_err(),
                "the fast path must propagate cancellation before scope exit"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
    let pair = operator
        .smallest_tridiagonal_eigenpair(1e-9)
        .unwrap()
        .unwrap();
    let expected = 4.0 * (std::f64::consts::PI / (4.0 * size as f64)).sin().powi(2);
    assert!((pair.eigenvalue / expected - 1.0).abs() < 1e-12);
}
